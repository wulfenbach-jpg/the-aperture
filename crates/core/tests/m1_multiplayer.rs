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
