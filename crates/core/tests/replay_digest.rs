//! Determinism, save/load round-trips, London mulligans, and replay.
//! CR 103.5, 401.2 (order comes from the seed, never from stored state).

mod common;

use aperture_core::*;
use common::*;

/// Drive both seats with the fixed scripted policy until a cap is hit.
fn scripted_game(seed: u64, cap: usize) -> (Game, Vec<Event>, Vec<ReplayStep>) {
    let (mut game, mut log) = Game::new_logged(seed, &[deck_a(), deck_b()]).expect("game");
    let mut steps = Vec::new();
    for _ in 0..cap {
        if game.state.is_over() {
            break;
        }
        let Some(owner) = game.decision_owner() else {
            break;
        };
        let action = auto_answer(&game);
        let step = ReplayStep {
            player: owner,
            revision: game.revision(),
            decision: game.decision(),
            action: action.clone(),
        };
        match game.submit_action(owner, action) {
            Ok(events) => {
                log.extend(events);
                steps.push(step);
            }
            Err(err) => panic!("scripted step rejected: {err}"),
        }
    }
    (game, log, steps)
}

#[test]
fn the_same_seed_and_actions_produce_the_same_state_digest() {
    let (a, _, steps_a) = scripted_game(101, 300);
    let (b, _, steps_b) = scripted_game(101, 300);
    assert_eq!(steps_a.len(), steps_b.len());
    assert_eq!(a.digest(), b.digest());
    assert_eq!(steps_a, steps_b, "the same action sequence was produced");

    let (c, _, _) = scripted_game(202, 300);
    assert_ne!(
        a.digest(),
        c.digest(),
        "a different seed shuffles differently"
    );
}

#[test]
fn london_mulligan_bottoms_one_card_per_mulligan() {
    let (mut game, mut log) = Game::new_logged(1, &[deck_a(), deck_b()]).expect("game");
    let order = game
        .state
        .mulligan
        .as_ref()
        .expect("opening mulligan state")
        .order
        .clone();
    let first = order[0];
    let second = order[1];
    assert_eq!(game.decision_owner(), Some(first));
    assert_eq!(game.state.player(first).hand_size(), 7);

    submit_ok(&mut game, first, Action::Mulligan { keep: false });
    assert_eq!(
        game.state.player(first).hand_size(),
        7,
        "no redraw before the other seat declares"
    );
    let events = submit_ok(&mut game, second, Action::Mulligan { keep: true });
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Mulligan {
            player,
            count: 1
        } if *player == first
    )));
    assert_eq!(game.state.player(first).mulligans, 1);
    assert_eq!(
        game.state.player(first).hand_size(),
        7,
        "a fresh seven is drawn at batch execution"
    );
    match &game.state.pending {
        Some(Pending::BottomCards { player, count }) => {
            assert_eq!(*player, first);
            assert_eq!(*count, 1);
        }
        other => panic!("expected a bottoming decision, got {other:?}"),
    }
    let offered = game.legal_actions();
    assert_eq!(offered.len(), 1);
    let chosen = match &offered[0].action {
        Action::BottomCards { cards } => cards.clone(),
        other => panic!("unexpected action {other:?}"),
    };
    submit_ok(&mut game, first, Action::BottomCards { cards: chosen });
    assert_eq!(game.state.player(first).hand_size(), 6);
    assert_eq!(game.state.player(first).library.len(), 34);
    assert_eq!(
        game.state.pending,
        Some(Pending::Mulligan { player: first })
    );

    submit_ok(&mut game, first, Action::Mulligan { keep: true });
    assert_eq!(game.state.pending, None);
    assert_eq!(game.state.player(second).hand_size(), 7);
    assert_eq!(game.state.player(second).library.len(), 33);
    assert!(game.state.turn.priority.is_some(), "turn 1 can begin");
    log.clear();
}

#[test]
fn save_and_load_restore_pending_choices_rng_and_ai_settings() {
    let (game, log) = Game::new_logged(5, &[deck_a(), deck_b()]).expect("game");
    assert!(
        matches!(game.state.pending, Some(Pending::Mulligan { .. })),
        "the opening mulligan is a pending choice with the RNG position"
    );
    let mut replay = Replay::new(
        5,
        game.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .expect("replay");
    let save = SessionSave::capture(&game, replay.clone(), log.clone(), false);
    let json = save.to_json();
    let loaded = SessionSave::from_json(&json).expect("save round-trips");
    assert_eq!(loaded.game, game.state, "state is preserved exactly");
    assert_eq!(loaded.game().digest(), game.digest());
    assert_eq!(loaded.ai, AiSettings::default());
    assert_eq!(loaded.log, log);

    // Both copies advance identically.
    let mut restored = loaded.game();
    let mut a = game;
    let owner = a.decision_owner().expect("a decision is pending");
    let action = auto_answer(&a);
    submit_ok(&mut a, owner, action.clone());
    submit_ok(&mut restored, owner, action);
    assert_eq!(a.digest(), restored.digest());

    // A replay with no digest recorded still round-trips through JSON.
    replay.record(ReplayStep {
        player: owner,
        revision: 0,
        decision: 0,
        action: auto_answer(&a),
    });
    let text = replay.to_json();
    let parsed = Replay::from_json(&text).expect("replay round-trips");
    assert_eq!(parsed.steps.len(), 1);
}

#[test]
fn replaying_a_recorded_game_reproduces_the_digest() {
    let (game, log, steps) = scripted_game(77, 250);
    let mut replay = Replay::new(
        77,
        game.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .expect("replay");
    for step in steps {
        replay.record(step);
    }
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();
    let (replayed, replayed_log) = replay.run().expect("replay reproduces the game");
    assert_eq!(replayed.digest(), game.digest());
    assert_eq!(replayed.state, game.state);
    assert_eq!(replayed_log.len(), log.len(), "the event trace replays too");

    // Truncated runs are recorded as truncation, not as a result.
    assert!(replay.outcome.is_some() || replay.truncated.is_none());
}

#[test]
fn a_tampered_replay_fails_instead_of_inventing_a_result() {
    let (game, _, steps) = scripted_game(88, 40);
    let mut replay = Replay::new(
        88,
        game.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .expect("replay");
    for step in steps {
        replay.record(step);
    }
    replay.final_digest = game.digest();
    assert!(replay.run().is_ok());

    // Replace the first action with a mulligan choice at the wrong moment.
    let mut tampered = replay.clone();
    tampered.steps[0].action = Action::Pass;
    match tampered.run() {
        Err(ReplayError::Step { index, .. }) => assert_eq!(index, 0),
        other => panic!("expected the replay to reject step 0, got {other:?}"),
    }

    // A wrong digest is reported, not papered over.
    let mut wrong = replay.clone();
    wrong.final_digest = "0".repeat(64);
    match wrong.run() {
        Err(ReplayError::DigestMismatch { expected, got }) => {
            assert_eq!(expected, "0".repeat(64));
            assert_ne!(got, expected);
        }
        other => panic!("expected a digest mismatch, got {other:?}"),
    }
}

#[test]
fn library_order_comes_from_the_seed_not_from_stored_state() {
    let a = Game::new(303, &[deck_a(), deck_b()]).expect("game");
    let b = Game::new(303, &[deck_a(), deck_b()]).expect("game");
    assert_eq!(a.state.player(P0).library, b.state.player(P0).library);
    assert_eq!(a.digest(), b.digest());
    let c = Game::new(304, &[deck_a(), deck_b()]).expect("game");
    assert_ne!(a.state.player(P0).library, c.state.player(P0).library);
}
