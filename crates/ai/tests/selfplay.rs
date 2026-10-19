//! Self-play: the drivers must actually finish games, never fabricate results,
//! and record replayable action sequences.

mod common;

use aperture_ai::*;
use aperture_core::*;
use common::*;

#[test]
fn heuristic_selfplay_reaches_a_terminal_result_and_replays() {
    let mut game = Game::new(4242, &[deck_a(), deck_b()]).expect("game");
    let mut policies: Vec<Box<dyn Policy>> = vec![
        Box::new(HeuristicPolicy::new()),
        Box::new(HeuristicPolicy::new()),
    ];
    let mut replay = Replay::new(
        4242,
        game.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .expect("replay");
    let mut log = Vec::new();
    let outcome = play_to_end(
        &mut game,
        &mut policies,
        SelfplayLimits {
            max_steps: 4_000,
            max_millis: None,
        },
        &mut log,
        &mut replay,
    );
    assert!(outcome.steps > 0, "the policy must act");
    assert!(
        outcome.truncated.is_none(),
        "a 4000-step cap should be enough for this matchup: {:?}",
        outcome.truncated
    );
    assert!(outcome.winner.is_some(), "a terminal result: {outcome:?}");
    assert_eq!(outcome.digest, game.digest());
    assert_eq!(replay.steps.len(), outcome.steps);

    // The AI must do more than pass: it casts and attacks.
    let casts = log
        .iter()
        .filter(|e| matches!(e, Event::Cast { .. }))
        .count();
    let attacks = log
        .iter()
        .filter(
            |e| matches!(e, Event::AttackersDeclared { attackers, .. } if !attackers.is_empty()),
        )
        .count();
    assert!(casts > 0, "selfplay cast nothing");
    assert!(attacks > 0, "selfplay never attacked");

    // The recorded action sequence reproduces the game exactly.
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();
    let (replayed, _) = replay.run().expect("replay reproduces the self-play game");
    assert_eq!(replayed.digest(), game.digest());
}

#[test]
fn identical_seeds_produce_identical_games_and_ai_digests() {
    let mut digests = Vec::new();
    for _ in 0..2 {
        let mut game = Game::new(99, &[deck_a(), deck_b()]).expect("game");
        let mut policies: Vec<Box<dyn Policy>> = vec![
            Box::new(HeuristicPolicy::new()),
            Box::new(HeuristicPolicy::new()),
        ];
        let mut replay = Replay::new(
            99,
            game.state.first_player,
            vec![deck_a(), deck_b()],
            AiSettings::default(),
        )
        .expect("replay");
        let mut log = Vec::new();
        let outcome = play_to_end(
            &mut game,
            &mut policies,
            SelfplayLimits {
                max_steps: 4_000,
                max_millis: None,
            },
            &mut log,
            &mut replay,
        );
        digests.push((outcome.digest, outcome.steps, outcome.winner));
    }
    assert_eq!(digests[0], digests[1], "self-play must be deterministic");
}

#[test]
fn a_run_cap_is_reported_as_truncation_not_a_result() {
    let mut game = Game::new(7, &[deck_a(), deck_b()]).expect("game");
    let mut policies: Vec<Box<dyn Policy>> = vec![
        Box::new(HeuristicPolicy::new()),
        Box::new(HeuristicPolicy::new()),
    ];
    let mut replay = Replay::new(
        7,
        game.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .expect("replay");
    let mut log = Vec::new();
    let outcome = play_to_end(
        &mut game,
        &mut policies,
        SelfplayLimits {
            max_steps: 25,
            max_millis: None,
        },
        &mut log,
        &mut replay,
    );
    assert_eq!(outcome.steps, 25);
    assert!(outcome.winner.is_none());
    let reason = outcome.truncated.clone().unwrap_or_default();
    assert!(
        reason.contains("cap") || reason.contains("repetition"),
        "{reason}"
    );
    assert!(log.iter().any(|e| matches!(e, Event::Truncated { .. })));
}
