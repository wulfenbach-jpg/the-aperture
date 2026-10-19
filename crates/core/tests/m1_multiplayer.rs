//! Focused M1 boundaries: 3/4-seat turn/priority scheduling, elimination
//! continuity, unequal-seat masking, and versioned replay/save behavior.

mod common;

use aperture_core::view::PlayerView;
use aperture_core::*;
use common::*;

fn three_decks() -> Vec<DeckList> {
    vec![deck_a(), deck_b(), deck_a()]
}

#[test]
fn three_seat_game_has_valid_ordered_seats_and_no_panic() {
    let (game, _) = Game::new_logged(3, &three_decks()).expect("three-seat game");
    assert_eq!(game.state.seat_count(), 3);
    game.state.validate().expect("valid state");
    assert_eq!(game.state.players[0].id, P0);
    assert_eq!(game.state.players[2].id, 2);
    assert!(Game::new(1, &[deck_a()]).is_err(), "one seat is rejected");
    let five = vec![deck_a(); 5];
    assert!(Game::new(1, &five).is_err(), "five seats are rejected");
}

#[test]
fn three_seat_first_player_draws_but_two_seat_first_player_skips() {
    // CR 103.8a vs 103.8c/800.7
    let (mut three, mut log) = Game::new_logged(9, &three_decks()).expect("three");
    start_game(&mut three, &mut log);
    assert!(run_until(
        &mut three,
        &mut log,
        |g| g.state.turn.number == 1
            && g.state.turn.step == Step::Draw
            && g.state.pending.is_none()
            && g.state.turn.priority.is_some(),
        128
    ));
    let first = three.state.first_player;
    assert_eq!(three.state.player(first).hand_size(), 8);

    let (mut two, mut log) = Game::new_logged(9, &[deck_a(), deck_b()]).expect("two-seat");
    start_game(&mut two, &mut log);
    assert!(run_until(
        &mut two,
        &mut log,
        |g| g.state.turn.number == 1
            && g.state.turn.step == Step::Draw
            && g.state.pending.is_none()
            && g.state.turn.priority.is_some(),
        128
    ));
    let first = two.state.first_player;
    assert_eq!(two.state.player(first).hand_size(), 7);
}

#[test]
fn three_seat_priority_round_resolves_exactly_one_item() {
    let mut game = fixture_n(11, 3)
        .hand(P0, &[CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(2)],
        },
    );
    assert_eq!(game.state.stack.len(), 1);
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.decision_owner(), Some(1));
    submit_ok(&mut game, 1, Action::Pass);
    assert_eq!(game.decision_owner(), Some(2));
    assert_eq!(game.state.stack.len(), 1, "not resolved yet");
    submit_ok(&mut game, 2, Action::Pass);
    assert_eq!(game.state.stack.len(), 0, "all three passed");
    assert_eq!(game.state.player(2).life, 18);
    assert_eq!(game.decision_owner(), Some(P0));
}

#[test]
fn last_live_seat_wins_after_simultaneous_losses() {
    let mut game = fixture_n(12, 3)
        .main(P0, Step::PrecombatMain)
        .life(1, 0)
        .life(2, 0)
        .build();
    submit_ok(&mut game, P0, Action::Pass);
    match &game.state.outcome {
        Some(Outcome::Win { winner, reason }) => {
            assert_eq!(*winner, P0);
            assert!(reason.contains("704.5a"), "{reason}");
        }
        other => panic!("expected P0 to win, got {other:?}"),
    }
}

#[test]
fn unequal_seats_are_masked_and_only_the_owner_has_legal_actions() {
    let game = fixture_n(13, 3)
        .hand(P0, &[CardId::Shock])
        .hand(1, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .library(2, &[CardId::Island, CardId::Island])
        .life(1, 7)
        .life(2, 3)
        .battlefield(1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let p0 = game.view_for(P0, &[]);
    assert_eq!(p0.seat_count, 3);
    assert_eq!(p0.opponents.len(), 2);
    let p1 = p0.opponent_view(1).expect("P1 projected");
    let p2 = p0.opponent_view(2).expect("P2 projected");
    assert_eq!(p1.hand_count, 3);
    assert_eq!(p1.life, 7);
    assert_eq!(p2.library_count, 2);
    assert_eq!(p2.life, 3);
    assert!(!p0.legal.is_empty(), "P0 owns the decision");
    let json = p0.to_json();
    for id in game
        .state
        .player(1)
        .hand
        .iter()
        .chain(game.state.player(2).library.iter())
    {
        assert!(
            !json.contains(&format!("\"id\":{},", id.0)),
            "hidden object {} leaked",
            id.0
        );
    }
    let p1_view: PlayerView = game.view_for(1, &[]);
    assert!(p1_view.legal.is_empty(), "non-owner gets no actions");
}

#[test]
fn three_seat_replay_and_save_round_trip_and_v1_is_rejected() {
    let (game, log) = Game::new_logged(21, &three_decks()).expect("game");
    let mut replay = Replay::new(
        21,
        game.state.first_player,
        three_decks(),
        AiSettings::default(),
    )
    .expect("replay");
    replay.final_digest = game.digest();
    let (replayed, _) = replay.run().expect("replay reproduces");
    assert_eq!(replayed.digest(), game.digest());

    let save = SessionSave::capture(&game, replay.clone(), log, false);
    let parsed = SessionSave::from_json(&save.to_json()).expect("save round-trips");
    assert_eq!(parsed.game().digest(), game.digest());
    assert_eq!(parsed.seats.len(), 3);

    let mut old = serde_json::to_value(&replay).expect("json");
    old["version"] = serde_json::json!(1);
    match Replay::from_json(&serde_json::to_string(&old).unwrap()) {
        Err(ReplayError::Version { got: 1, .. }) => {}
        other => panic!("expected an explicit version rejection, got {other:?}"),
    }
    assert!(matches!(
        Replay::from_json("{"),
        Err(ReplayError::Malformed { .. })
    ));
}

#[test]
fn first_multiplayer_mulligan_is_free_but_the_second_bottoms_one() {
    // CR 103.5c
    let (mut game, mut log) = Game::new_logged(31, &three_decks()).expect("game");
    submit_ok(&mut game, P0, Action::Mulligan { keep: false });
    assert_eq!(game.state.player(P0).mulligans, 1);
    submit_ok(&mut game, P0, Action::Mulligan { keep: true });
    assert_eq!(
        game.state.pending,
        Some(Pending::Mulligan { player: 1 }),
        "the free first mulligan bottoms no cards"
    );
    submit_ok(&mut game, 1, Action::Mulligan { keep: false });
    submit_ok(&mut game, 1, Action::Mulligan { keep: false });
    submit_ok(&mut game, 1, Action::Mulligan { keep: true });
    match &game.state.pending {
        Some(Pending::BottomCards { player, count }) => {
            assert_eq!(*player, 1);
            assert_eq!(*count, 1);
        }
        other => panic!("expected one card on the bottom, got {other:?}"),
    }
    let _ = &mut log;
}

#[test]
fn active_departure_keeps_the_turn_and_stack_alive_then_advances_to_the_next_live_seat() {
    // CR 800.4a/j/k: P0 (active) leaves mid-turn; the turn continues, a stack
    // item controlled by P1 still resolves, and the next turn belongs to P1.
    use aperture_core::state::StackItem;

    let mut game = fixture_n(40, 4)
        .hand(1, &[CardId::Shock])
        .battlefield(1, &[CardId::Mountain])
        .life(P0, 0)
        .main(P0, Step::PrecombatMain)
        .build();
    let shock = hand_card(&game, 1, CardId::Shock);
    game.state.players[1].hand.retain(|id| *id != shock);
    object_mut(&mut game.state, shock).zone = Zone::Stack;
    // Resolve Shock through the real stack so the remaining turn has work to do.
    game.state.stack.push(StackItem {
        object: shock,
        controller: 1,
        targets: vec![Target::Player(2)],
        cursor: 0,
    });

    submit_ok(&mut game, P0, Action::Pass);
    assert!(game.state.player(P0).lost);
    assert!(game.state.outcome.is_none(), "three live seats remain");
    assert_eq!(game.state.turn.active, P0, "the turn does not end");
    assert_eq!(game.state.turn.priority, Some(1), "priority passes to P1");
    assert_eq!(game.state.stack.len(), 1, "P1's stack object remains");

    submit_ok(&mut game, 1, Action::Pass);
    submit_ok(&mut game, 2, Action::Pass);
    submit_ok(&mut game, 3, Action::Pass);
    assert_eq!(game.state.stack.len(), 0, "one item resolved");
    assert_eq!(game.state.player(2).life, 18);
    assert_eq!(game.state.turn.active, P0);

    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.number == 2,
        512
    ));
    assert_eq!(game.state.turn.active, 1, "next live seat after P0 is P1");
}

#[test]
fn four_seat_turn_order_skips_eliminated_seats_and_wraps() {
    let forests = vec![CardId::Forest; 60];
    let mut game = fixture_n(41, 4)
        .library(P0, &forests)
        .library(2, &forests)
        .life(1, 0)
        .life(3, 0)
        .main(P0, Step::PrecombatMain)
        .build();
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.live_count(), 2);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.number == 2,
        512
    ));
    assert_eq!(game.state.turn.active, 2, "P1 is skipped");
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.number == 3,
        512
    ));
    assert_eq!(
        game.state.turn.active, P0,
        "P3 is skipped and the order wraps"
    );
}

#[test]
fn replay_rejects_a_mismatched_rules_revision() {
    let (game, _) = Game::new_logged(51, &three_decks()).expect("game");
    let mut replay = Replay::new(
        51,
        game.state.first_player,
        three_decks(),
        AiSettings::default(),
    )
    .expect("replay");
    replay.rules_revision = "1999-01-01".to_string();
    assert!(
        replay.run().is_err(),
        "a replay recorded under another rules revision must be rejected"
    );
}

#[test]
fn save_rejects_a_missing_object_reference_and_stale_replay_invariants() {
    let (game, log) = Game::new_logged(52, &three_decks()).expect("game");
    let replay = Replay::new(
        52,
        game.state.first_player,
        three_decks(),
        AiSettings::default(),
    )
    .expect("replay");
    let mut save = SessionSave::capture(&game, replay, log, false);

    // Drop an object still referenced by a zone; refresh the digest so only a
    // referential-integrity check can catch it.
    let missing = save.game.player(P0).hand[0];
    save.game.objects.remove(&missing);
    save.digest = aperture_core::digest::digest(&save.game);
    let json = serde_json::to_string(&save).expect("save json");
    assert!(
        SessionSave::from_json(&json).is_err(),
        "a save with a dangling zone reference must be rejected, not panic later"
    );

    // The embedded replay must agree with the snapshot's rules/first player.
    let mut stale = SessionSave::capture(
        &game,
        Replay::new(
            52,
            game.state.first_player,
            three_decks(),
            AiSettings::default(),
        )
        .expect("replay"),
        Vec::new(),
        false,
    );
    stale.replay.rules_revision = "1999-01-01".to_string();
    let json = serde_json::to_string(&stale).expect("stale json");
    assert!(
        SessionSave::from_json(&json).is_err(),
        "a save whose replay targets another rules revision must be rejected"
    );
}
