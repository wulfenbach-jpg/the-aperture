//! Focused M1 boundaries: 3/4-seat turn/priority scheduling, elimination
//! continuity, unequal-seat masking, and versioned replay/save behavior.

mod common;

use aperture_core::view::PlayerView;
use aperture_core::*;
use common::*;

fn three_decks() -> Vec<DeckList> {
    vec![deck_a(), deck_b(), deck_a()]
}

/// Same public board/counts for every viewer, different hidden library
/// identities/order. Hands, battlefields and life totals are deliberately
/// unequal across seats.
fn hidden_state_game(seat_count: usize, flipped: bool) -> Game {
    let mut fixture = fixture_n(80 + seat_count as u64, seat_count);
    for seat in 0..seat_count {
        let player = seat as PlayerId;
        let hand = [
            CardId::Forest,
            CardId::Island,
            CardId::Mountain,
            CardId::Shock,
        ];
        let hand_count = seat % 4 + 1;
        fixture = fixture.hand(player, &hand[..hand_count]);
        let library_a = [
            CardId::Forest,
            CardId::Island,
            CardId::Mountain,
            CardId::Shock,
            CardId::Opt,
        ];
        let library_b = [
            CardId::Opt,
            CardId::Shock,
            CardId::Mountain,
            CardId::Island,
            CardId::Forest,
        ];
        let library_count = seat % 4 + 2;
        let library = if flipped {
            &library_b[..library_count]
        } else {
            &library_a[..library_count]
        };
        fixture = fixture.library(player, library);
        fixture = fixture.life(player, 20 - (seat as i32) * 3);
    }
    fixture.main(P0, Step::PrecombatMain).build()
}

/// Submit an accepted action and record the exact revision/decision identity in
/// a replay, so a save can be checked against its embedded action sequence.
fn submit_recorded(game: &mut Game, replay: &mut Replay, player: PlayerId, action: Action) {
    let revision = game.revision();
    let decision = game.decision();
    game.submit_action(player, action.clone())
        .expect("recorded action is accepted");
    replay.record(ReplayStep {
        player,
        revision,
        decision,
        action,
    });
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

    for old_version in [1u32, 2] {
        let mut old = serde_json::to_value(&replay).expect("json");
        old["version"] = serde_json::json!(old_version);
        match Replay::from_json(&serde_json::to_string(&old).unwrap()) {
            Err(ReplayError::Version { got, expected: 3 }) if got == old_version => {}
            other => panic!("expected an explicit replay-v{old_version} rejection, got {other:?}"),
        }
    }
    assert!(matches!(
        Replay::from_json("{"),
        Err(ReplayError::Malformed { .. })
    ));
}

#[test]
fn first_multiplayer_mulligan_is_free_but_the_second_bottoms_one() {
    // CR 103.5/103.5c: declare in starting-player order, execute the batch,
    // then the free first mulligan bottoms nothing and the second bottoms one.
    let (mut game, _) = Game::new_logged(31, &three_decks()).expect("game");
    let order = game
        .state
        .mulligan
        .as_ref()
        .expect("opening mulligan state")
        .order
        .clone();
    let first = order[0];
    submit_ok(&mut game, first, Action::Mulligan { keep: false });
    for player in order.iter().copied().skip(1) {
        submit_ok(&mut game, player, Action::Mulligan { keep: true });
    }
    assert_eq!(game.state.player(first).mulligans, 1);
    assert_eq!(
        game.state.pending,
        Some(Pending::Mulligan { player: first }),
        "the free first mulligan bottoms no cards; only the mulliganer declares again"
    );

    submit_ok(&mut game, first, Action::Mulligan { keep: false });
    assert_eq!(game.state.player(first).mulligans, 2);
    match &game.state.pending {
        Some(Pending::BottomCards { player, count }) => {
            assert_eq!(*player, first);
            assert_eq!(*count, 1, "the second multiplayer mulligan bottoms one");
        }
        other => panic!("expected one card on the bottom, got {other:?}"),
    }
    let bottom = game
        .legal_actions()
        .into_iter()
        .find(|legal| matches!(legal.action, Action::BottomCards { .. }))
        .expect("a bottom action is offered")
        .action;
    submit_ok(&mut game, first, bottom);
    assert_eq!(
        game.state.pending,
        Some(Pending::Mulligan { player: first }),
        "the next declaration round still belongs to the mulliganer"
    );
    submit_ok(&mut game, first, Action::Mulligan { keep: true });
    assert_eq!(game.state.pending, None);
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

#[test]
fn four_seat_unequal_views_keep_pending_choices_owner_only() {
    let mut game = fixture_n(60, 4)
        .hand(P0, &[CardId::Opt])
        .battlefield(P0, &[CardId::Island])
        .library(P0, &[CardId::Forest, CardId::Island])
        .hand(1, &[CardId::Forest, CardId::Forest])
        .library(1, &[CardId::Island, CardId::Island, CardId::Island])
        .hand(
            2,
            &[CardId::Shock, CardId::Shock, CardId::Shock, CardId::Shock],
        )
        .library(2, &[CardId::Mountain])
        .hand(3, &[CardId::GrizzlyBears])
        .library(3, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .life(1, 9)
        .life(2, 4)
        .life(3, 1)
        .main(P0, Step::PrecombatMain)
        .build();
    let opt = hand_card(&game, P0, CardId::Opt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: Vec::new(),
        },
    );
    for player in [P0, 1, 2, 3] {
        submit_ok(&mut game, player, Action::Pass);
    }
    assert!(
        matches!(
            game.state.pending,
            Some(Pending::OrderCard { player: P0, .. })
        ),
        "Opt's scry is pending for P0"
    );

    let p0 = game.view_for(P0, &[]);
    assert_eq!(p0.seat_count, 4);
    assert_eq!(p0.opponents.len(), 3);
    assert_eq!(p0.pending_viewed.len(), 1, "owner sees the looked-at card");
    assert_eq!(p0.legal.len(), 2, "owner gets the order choices");
    assert_eq!(p0.opponent_view(1).expect("P1").life, 9);
    assert_eq!(p0.opponent_view(2).expect("P2").life, 4);
    assert_eq!(p0.opponent_view(3).expect("P3").life, 1);

    let viewed = p0.pending_viewed[0].id.0;
    for seat in [1, 2, 3] {
        let view = game.view_for(seat, &[]);
        assert!(
            view.pending_viewed.is_empty(),
            "P{seat} must not see the card"
        );
        assert!(view.legal.is_empty(), "P{seat} has no decision");
        assert!(
            view.pending
                .as_deref()
                .is_some_and(|text| text.contains("P0")),
            "P{seat} sees only that P0 must choose"
        );
        let json = view.to_json();
        assert!(
            !json.contains(&format!("\"id\":{},", viewed)),
            "the scry card leaked into P{seat}'s JSON"
        );
    }

    let p0_json = p0.to_json();
    let hidden: Vec<ObjectId> = game
        .state
        .player(1)
        .hand
        .iter()
        .chain(game.state.player(2).library.iter())
        .chain(game.state.player(3).hand.iter())
        .copied()
        .collect();
    for id in hidden {
        assert!(
            !p0_json.contains(&format!("\"id\":{},", id.0)),
            "another seat's hidden object {} leaked into P0's JSON",
            id.0
        );
    }
}

#[test]
fn four_seat_eliminated_seat_projects_no_hidden_information() {
    let mut game = fixture_n(61, 4)
        .hand(3, &[CardId::Forest, CardId::Forest])
        .library(3, &[CardId::Island, CardId::Island])
        .battlefield(3, &[CardId::GrizzlyBears])
        .life(3, 0)
        .main(P0, Step::PrecombatMain)
        .build();
    let hidden: Vec<ObjectId> = game
        .state
        .player(3)
        .hand
        .iter()
        .chain(game.state.player(3).library.iter())
        .copied()
        .collect();
    submit_ok(&mut game, P0, Action::Pass);
    assert!(game.state.player(3).lost);
    assert_eq!(game.state.live_count(), 3);
    let p0 = game.view_for(P0, &[]);
    let eliminated = p0.opponent_view(3).expect("P3 projected as eliminated");
    assert!(eliminated.eliminated);
    assert_eq!(eliminated.hand_count, 0);
    assert_eq!(eliminated.library_count, 0);
    assert!(eliminated.battlefield.is_empty());
    assert!(eliminated.graveyard.is_empty());
    assert!(eliminated.exile.is_empty());
    let json = p0.to_json();
    for id in hidden {
        assert!(
            !json.contains(&format!("\"id\":{},", id.0)),
            "eliminated seat's hidden object {} leaked",
            id.0
        );
    }
}

#[test]
fn save_v3_is_rejected_explicitly() {
    let (game, log) = Game::new_logged(62, &three_decks()).expect("game");
    let replay = Replay::new(
        62,
        game.state.first_player,
        three_decks(),
        AiSettings::default(),
    )
    .expect("replay");
    let save = SessionSave::capture(&game, replay, log, false);
    let mut value = serde_json::to_value(&save).expect("save json");
    value["version"] = serde_json::json!(3);
    match SessionSave::from_json(&serde_json::to_string(&value).expect("json")) {
        Err(ReplayError::Version {
            got: 3,
            expected: 4,
        }) => {}
        other => panic!("expected an explicit save-v3 rejection, got {other:?}"),
    }
}

#[test]
fn pending_choice_survives_a_save_round_trip() {
    let decks = vec![deck_a(), deck_b(), deck_a(), deck_b()];
    let (game, log) = Game::new_logged(63, &decks).expect("game");
    assert!(
        matches!(game.state.pending, Some(Pending::Mulligan { player: P0 })),
        "the opening mulligan is a pending choice"
    );
    let replay =
        Replay::new(63, game.state.first_player, decks, AiSettings::default()).expect("replay");
    let save = SessionSave::capture(&game, replay, log, false);
    let parsed = SessionSave::from_json(&save.to_json()).expect("pending save round-trips");
    assert_eq!(parsed.game.pending, game.state.pending);
    assert_eq!(parsed.game().digest(), game.digest());
}

#[test]
fn a_response_resets_the_priority_pass_mask() {
    let mut game = fixture_n(64, 3)
        .hand(P0, &[CardId::Shock])
        .hand(1, &[CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .battlefield(1, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let p0_shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: p0_shock,
            targets: vec![Target::Player(2)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.turn.passed, 0b001);
    let p1_shock = hand_card(&game, 1, CardId::Shock);
    submit_ok(
        &mut game,
        1,
        Action::CastSpell {
            card: p1_shock,
            targets: vec![Target::Player(P0)],
        },
    );
    assert_eq!(game.state.turn.passed, 0, "a response clears prior passes");
    assert_eq!(game.decision_owner(), Some(1));

    submit_ok(&mut game, 1, Action::Pass);
    submit_ok(&mut game, 2, Action::Pass);
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.stack.len(), 1, "only the top item resolved");
    assert_eq!(game.state.player(P0).life, 18);
}

#[test]
fn masked_projection_digest_does_not_depend_on_hidden_state() {
    // Same public board/counts, different hidden P1 hand and library order.
    let a = fixture_n(70, 3)
        .hand(P0, &[CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .hand(1, &[CardId::Forest])
        .library(1, &[CardId::Island])
        .hand(2, &[CardId::Forest])
        .library(2, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let b = fixture_n(70, 3)
        .hand(P0, &[CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .hand(1, &[CardId::Island])
        .library(1, &[CardId::Forest])
        .hand(2, &[CardId::Forest])
        .library(2, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    assert_ne!(a.digest(), b.digest(), "the full states really differ");

    for viewer in [P0, 2] {
        let view_a = a.view_for(viewer, &[]);
        let view_b = b.view_for(viewer, &[]);
        assert_eq!(
            view_a.to_json(),
            view_b.to_json(),
            "P{viewer}'s serialized observation changed with hidden state"
        );
        assert_eq!(
            view_a.digest, view_b.digest,
            "P{viewer}'s view digest is an oracle over hidden state"
        );
    }
}

#[test]
fn save_rejects_a_tampered_embedded_replay() {
    let (mut game, _) = Game::new_logged(71, &three_decks()).expect("game");
    let mut replay = Replay::new(
        71,
        game.state.first_player,
        three_decks(),
        AiSettings::default(),
    )
    .expect("replay");
    let order = game
        .state
        .mulligan
        .as_ref()
        .expect("opening mulligan state")
        .order
        .clone();
    for player in order {
        submit_recorded(
            &mut game,
            &mut replay,
            player,
            Action::Mulligan { keep: true },
        );
    }
    assert!(game.state.pending.is_none(), "all opening hands are kept");
    let save = SessionSave::capture(&game, replay, Vec::new(), false);
    let mut value = serde_json::to_value(&save).expect("save json");
    value["replay"]["steps"][0]["action"] =
        serde_json::to_value(Action::Mulligan { keep: false }).expect("action json");
    let tampered = serde_json::to_string(&value).expect("tampered json");
    assert!(
        SessionSave::from_json(&tampered).is_err(),
        "a save whose embedded replay diverges from its state must be rejected"
    );
}

#[test]
fn all_viewers_ignore_hidden_state_for_2_3_4_seats() {
    for seats in [2usize, 3, 4] {
        let a = hidden_state_game(seats, false);
        let b = hidden_state_game(seats, true);
        assert_ne!(
            a.digest(),
            b.digest(),
            "{seats}-seat full states really differ"
        );
        let owner = a.state.decision_owner().expect("P0 has priority");
        for viewer in 0..seats as PlayerId {
            let view_a = a.view_for(viewer, &[]);
            let view_b = b.view_for(viewer, &[]);
            assert_eq!(
                view_a.to_json(),
                view_b.to_json(),
                "P{viewer} observation changed with hidden state in a {seats}-seat game"
            );
            assert_eq!(
                view_a.digest, view_b.digest,
                "P{viewer} digest is an oracle in a {seats}-seat game"
            );
            assert_eq!(
                view_a.legal.is_empty(),
                viewer != owner,
                "only the decision owner may receive legal actions"
            );
            assert_eq!(view_a.opponents.len(), seats - 1);
        }
        assert_eq!(
            a.state.player(P0).life,
            20,
            "unequal life totals are deliberate"
        );
        assert!(a.state.player(3.min(seats as u8 - 1)).life < 20);
    }
}

#[test]
fn mulligan_declarations_start_with_the_starting_player_and_do_not_redraw_early() {
    for seats in 2..=4usize {
        let decks = vec![deck_a(); seats];
        let (seed, mut game) = (0..200u64)
            .map(|seed| (seed, Game::new(seed, &decks).expect("game")))
            .find(|(_, game)| game.state.first_player != 0)
            .expect("a seed where a non-P0 seat starts");
        let first = game.state.first_player;
        assert_eq!(
            game.decision_owner(),
            Some(first),
            "{seats}-seat seed {seed}: declarations must start with P{first}"
        );
        let before = game.state.player(first).hand.clone();
        let mulligans = game.state.player(first).mulligans;
        submit_ok(&mut game, first, Action::Mulligan { keep: false });
        assert_eq!(
            game.state.player(first).hand,
            before,
            "{seats}-seat seed {seed}: no redraw before every eligible seat declares"
        );
        assert_eq!(
            game.state.player(first).mulligans,
            mulligans,
            "{seats}-seat seed {seed}: mulligan count changes only at batch execution"
        );
        let next = game.state.next_live_seat_after(first).expect("next seat");
        assert_eq!(
            game.decision_owner(),
            Some(next),
            "{seats}-seat seed {seed}: next declaration follows turn order"
        );
    }
}

#[test]
fn mulligan_rounds_wrap_and_lock_kept_hands() {
    let (mut game, _) = Game::new_logged(32, &three_decks()).expect("game");
    let order = game
        .state
        .mulligan
        .as_ref()
        .expect("mulligan state")
        .order
        .clone();
    assert_eq!(order[0], game.state.first_player);
    for window in order.windows(2) {
        assert_eq!(
            game.state.next_live_seat_after(window[0]),
            Some(window[1]),
            "declaration order follows turn order and wraps"
        );
    }
    let kept = order[0];
    let mulliganer = order[1];
    let other = order[2];
    let kept_hand = game.state.player(kept).hand.clone();
    let other_hand = game.state.player(other).hand.clone();
    submit_ok(&mut game, kept, Action::Mulligan { keep: true });
    submit_ok(&mut game, mulliganer, Action::Mulligan { keep: false });
    submit_ok(&mut game, other, Action::Mulligan { keep: true });
    assert_eq!(game.state.player(mulliganer).mulligans, 1);
    assert_eq!(game.state.player(kept).hand, kept_hand);
    assert_eq!(game.state.player(other).hand, other_hand);
    assert_eq!(
        game.state.pending,
        Some(Pending::Mulligan { player: mulliganer })
    );
    submit_ok(&mut game, mulliganer, Action::Mulligan { keep: true });
    assert_eq!(game.state.pending, None);
    assert_eq!(game.state.player(kept).hand, kept_hand);
    assert_eq!(game.state.player(other).hand, other_hand);
}

#[test]
fn wrong_seat_mulligan_is_rejected_without_mutation() {
    let (mut game, _) = Game::new_logged(33, &three_decks()).expect("game");
    let owner = game.decision_owner().expect("opening declarer");
    let wrong = (owner + 1) % 3;
    let digest = game.digest();
    let revision = game.revision();
    let pending = game.state.pending.clone();
    let err = submit_err(&mut game, wrong, Action::Mulligan { keep: true });
    assert!(matches!(err, EngineError::NotYourDecision { .. }), "{err}");
    assert_eq!(game.digest(), digest);
    assert_eq!(game.revision(), revision);
    assert_eq!(game.state.pending, pending);
}

#[test]
fn mulligan_intermediate_and_bottom_states_save_and_replay() {
    let decks = vec![deck_a(), deck_b()];
    let (mut game, _) = Game::new_logged(34, &decks).expect("game");
    let mut replay =
        Replay::new(34, game.state.first_player, decks, AiSettings::default()).expect("replay");
    let order = game
        .state
        .mulligan
        .as_ref()
        .expect("mulligan state")
        .order
        .clone();
    let first = order[0];
    let second = order[1];
    submit_recorded(
        &mut game,
        &mut replay,
        first,
        Action::Mulligan { keep: false },
    );
    submit_recorded(
        &mut game,
        &mut replay,
        second,
        Action::Mulligan { keep: true },
    );
    assert!(matches!(
        game.state.pending,
        Some(Pending::BottomCards { player, .. }) if player == first
    ));
    let save = SessionSave::capture(&game, replay.clone(), Vec::new(), false);
    let parsed = SessionSave::from_json(&save.to_json()).expect("intermediate save round-trips");
    assert_eq!(parsed.game().digest(), game.digest());
    let (replayed, _) = replay.run().expect("intermediate replay reproduces");
    assert_eq!(replayed.state, game.state);

    let bottom = game
        .legal_actions()
        .into_iter()
        .find(|legal| matches!(legal.action, Action::BottomCards { .. }))
        .expect("bottom is offered")
        .action;
    submit_recorded(&mut game, &mut replay, first, bottom);
    assert_eq!(
        game.state.pending,
        Some(Pending::Mulligan { player: first })
    );
    let save = SessionSave::capture(&game, replay.clone(), Vec::new(), false);
    let parsed = SessionSave::from_json(&save.to_json()).expect("next-round save round-trips");
    assert_eq!(parsed.game().digest(), game.digest());
    let (replayed, _) = replay.run().expect("next-round replay reproduces");
    assert_eq!(replayed.state, game.state);
}

#[test]
fn save_rejects_malformed_mulligan_state() {
    let decks = vec![deck_a(), deck_b()];
    let (game, log) = Game::new_logged(35, &decks).expect("game");
    let replay =
        Replay::new(35, game.state.first_player, decks, AiSettings::default()).expect("replay");
    let save = SessionSave::capture(&game, replay, log, false);
    let mut value = serde_json::to_value(&save).expect("save json");
    value["game"]["mulligan"]["cursor"] = serde_json::json!(99);
    let json = serde_json::to_string(&value).expect("json");
    assert!(
        SessionSave::from_json(&json).is_err(),
        "malformed mulligan state must be rejected, not silently accepted"
    );
}

#[test]
fn mulligan_validation_rejects_malformed_first_rounds() {
    let (game, _) = Game::new_logged(301, &three_decks()).expect("game");
    let mut reversed = game.state.clone();
    if let Some(mulligan) = &mut reversed.mulligan {
        mulligan.order.reverse();
    }
    assert!(
        reversed.validate().is_err(),
        "a reversed first-round mulligan order must be rejected"
    );

    let mut missing = game.state.clone();
    if let Some(mulligan) = &mut missing.mulligan {
        mulligan.order.pop();
    }
    assert!(
        missing.validate().is_err(),
        "a first-round order missing a live seat must be rejected"
    );

    let mut round_zero = game.state.clone();
    if let Some(mulligan) = &mut round_zero.mulligan {
        mulligan.round = 0;
    }
    assert!(round_zero.validate().is_err(), "round 0 is invalid");
}

#[test]
fn later_mulligan_round_reversal_is_rejected_independently_of_metadata() {
    let (mut game, _) = Game::new_logged(305, &three_decks()).expect("game");
    let order = game.state.mulligan.as_ref().unwrap().order.clone();
    submit_ok(&mut game, order[0], Action::Mulligan { keep: false });
    submit_ok(&mut game, order[1], Action::Mulligan { keep: false });
    submit_ok(&mut game, order[2], Action::Mulligan { keep: true });
    let mut reversed = game.state.clone();
    if let Some(mulligan) = &mut reversed.mulligan {
        mulligan.eligible.reverse();
        mulligan.previous_mulliganers = mulligan.eligible.clone();
        mulligan.order = mulligan.eligible.clone();
    }
    if let Some(Pending::Mulligan { player }) = &mut reversed.pending {
        *player = reversed.mulligan.as_ref().unwrap().order[0];
    }
    assert!(
        reversed.validate().is_err(),
        "a reversed later-round order must be rejected from the starting-player anchor"
    );
}

#[test]
fn paid_bottom_count_and_hand_shape_are_validated() {
    // Two-seat first mulligans are paid; both seats owe one bottom.
    let (mut game, _) = Game::new_logged(306, &[deck_a(), deck_b()]).expect("game");
    let order = game.state.mulligan.as_ref().unwrap().order.clone();
    for seat in order.clone() {
        submit_ok(&mut game, seat, Action::Mulligan { keep: false });
    }
    match &game.state.pending {
        Some(Pending::BottomCards { count, .. }) => assert_eq!(*count, 1),
        other => panic!("expected a paid bottom, got {other:?}"),
    }

    let mut zero = game.state.clone();
    if let Some(Pending::BottomCards { count, .. }) = &mut zero.pending {
        *count = 0;
    }
    assert!(
        zero.validate().is_err(),
        "BottomCards count 0 must be rejected"
    );

    let mut forged = game.state.clone();
    if let Some(mulligan) = &mut forged.mulligan {
        mulligan.bottomed = vec![order[0]];
        mulligan.bottom_queue = vec![order[1]];
    }
    if let Some(Pending::BottomCards { player, count }) = &mut forged.pending {
        *player = order[1];
        *count = 1;
    }
    assert!(
        forged.validate().is_err(),
        "a bottomed seat with an unbottomed hand must be rejected"
    );
}

#[test]
fn mulligan_round_counter_must_match_eligible_mulligan_counts() {
    let (mut game, _) = Game::new_logged(307, &three_decks()).expect("game");
    let order = game.state.mulligan.as_ref().unwrap().order.clone();
    submit_ok(&mut game, order[0], Action::Mulligan { keep: false });
    submit_ok(&mut game, order[1], Action::Mulligan { keep: false });
    submit_ok(&mut game, order[2], Action::Mulligan { keep: true });
    let mut forged = game.state.clone();
    if let Some(mulligan) = &mut forged.mulligan {
        mulligan.round = 3;
    }
    assert!(
        forged.validate().is_err(),
        "round 3 with eligible mulligan counts of 1 must be rejected"
    );

    // A legitimate mixed round: one seat mulligans again, the other keeps.
    let mut mixed = game;
    let order = mixed.state.mulligan.as_ref().unwrap().order.clone();
    submit_ok(&mut mixed, order[0], Action::Mulligan { keep: false });
    submit_ok(&mut mixed, order[1], Action::Mulligan { keep: true });
    mixed
        .state
        .validate()
        .expect("mixed keep/mulligan round is valid");
}
