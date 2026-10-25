//! Focused issue23 core controls: branching a recorded line at an explicit
//! action boundary, deterministic forward play on the branch, and the public
//! accepted-action comparison.

use aperture_core::*;

fn apply(game: &mut Game, replay: &mut Replay, log: &mut Vec<Event>, action: Action) {
    let player = game.decision_owner().unwrap();
    let step = ReplayStep {
        player,
        revision: game.revision(),
        decision: game.decision(),
        action: action.clone(),
    };
    log.extend(game.submit_action(player, action).unwrap());
    replay.record(step);
    replay.final_digest = game.digest();
}

fn start(seed: u64, text: &str) -> (Game, Replay, Vec<Event>) {
    let deck = DeckList::parse_practice("issue23", text).unwrap();
    let decks = vec![deck.clone(), deck];
    let (game, log) = Game::new_logged(seed, &decks).unwrap();
    let replay = Replay::new(seed, game.state.first_player, decks, AiSettings::default()).unwrap();
    (game, replay, log)
}

/// A short deterministic line: opening keeps, land, mana, cast.
fn recorded_line(seed: u64) -> (Game, Replay, Vec<Event>) {
    let (mut game, mut replay, mut log) = start(seed, "20 Mountain\n20 Crack the Earth\n");
    for _ in 0..2 {
        apply(
            &mut game,
            &mut replay,
            &mut log,
            Action::Mulligan { keep: true },
        );
    }
    (game, replay, log)
}

#[test]
fn a_branch_keeps_exactly_the_chosen_prefix_and_replays_it_deterministically() {
    for keep in [0usize, 1, 2] {
        let (game, replay, log) = recorded_line(2301);
        assert_eq!(replay.steps.len(), 2, "setup recorded two guarded keeps");
        let branch = replay.branch_at(keep).expect("branch point");
        assert_eq!(branch.steps.len(), keep);
        assert_eq!(branch.seed, replay.seed);
        assert_eq!(branch.seats, replay.seats);
        assert!(
            branch.final_digest.is_empty()
                && branch.outcome.is_none()
                && branch.truncated.is_none(),
            "the branch must not carry the base line's recorded end"
        );
        let (replayed, replayed_log) = branch.run_recorded().expect("the branch replays");
        if keep == replay.steps.len() {
            assert_eq!(
                replayed.state, game.state,
                "a full-prefix branch must reproduce the base state exactly"
            );
            assert_eq!(replayed_log.len(), log.len());
        } else {
            // A shorter prefix deterministically reaches the same state the
            // base line had after those same actions (same seed, same decks).
            let (base_prefix_game, _) = {
                let mut prefix = replay.clone();
                prefix.steps.truncate(keep);
                prefix.final_digest.clear();
                prefix.run_recorded().expect("prefix replays")
            };
            assert_eq!(replayed.state, base_prefix_game.state);
        }
    }
}

#[test]
fn a_branch_beyond_the_recorded_line_is_refused() {
    let (_, replay, _) = recorded_line(2302);
    assert!(replay.steps.len() == 2);
    let err = replay
        .branch_at(3)
        .expect_err("branching past the line is an invariant violation");
    assert!(
        err.to_string().contains("branch point 3 is beyond the 2"),
        "{err}"
    );
    let full = replay.branch_at(2).expect("the full line branches");
    assert_eq!(full.steps.len(), 2);
}

#[test]
fn the_accepted_action_comparison_shows_only_the_public_action_axis() {
    let (_, replay, _) = recorded_line(2303);
    // Same setup, different second decision: mulligan to five at the same
    // decision boundary instead of keeping.
    let mut branch = replay.branch_at(1).expect("branch at the second action");
    let (mut game, _) = branch.run_recorded().expect("prefix replays");
    let player = game.decision_owner().unwrap();
    let step = ReplayStep {
        player,
        revision: game.revision(),
        decision: game.decision(),
        action: Action::Mulligan { keep: false },
    };
    game.submit_action(player, Action::Mulligan { keep: false })
        .expect("the mulligan choice is legal here");
    branch.record(step);

    let diff = compare_branches(&replay, &branch).expect("same setup compares");
    assert_eq!(diff.diverged_at, Some(1));

    let rendered = diff.render();
    assert!(
        rendered.contains("mulligan choice"),
        "the comparison must show the public action kinds: {rendered}"
    );
    assert!(
        rendered.contains("#1") && rendered.contains("P1"),
        "the compared line must name the public decision axis (step, player): {rendered}"
    );
    for leak in ["Crack the Earth", "Mountain", "card", "target"] {
        // Card names, targets and zone contents must never appear.
        assert!(
            !rendered.contains(leak),
            "the default comparison must not leak private surface '{leak}': {rendered}"
        );
    }

    let mismatch = Replay::new(
        replay.seed + 1,
        replay.first_player,
        replay.seats.clone(),
        AiSettings::default(),
    )
    .expect("other replay");
    let err = compare_branches(&replay, &mismatch).expect_err("different setups are refused");
    assert!(err.to_string().contains("same setup"), "{err}");
}

#[test]
fn the_diff_never_reveals_the_base_tail_beyond_the_branchs_recorded_history() {
    // Base line of THREE recorded steps: the two guarded keeps, then one more
    // accepted action recorded on the base game itself.
    let (mut game, mut replay, mut log) = recorded_line(2305);
    let next = game
        .legal_actions()
        .into_iter()
        .next()
        .expect("the engine offers a legal action at a live decision")
        .action;
    apply(&mut game, &mut replay, &mut log, next);
    assert_eq!(replay.steps.len(), 3);
    let base_tail_kind = replay.steps[2].action.kind();
    assert_ne!(
        base_tail_kind, "mulligan choice",
        "setup: the base tail differs from the divergent kind"
    );

    // A live branch cut at #1 with one divergent mulligan: TWO recorded steps.
    let mut branch = replay.branch_at(1).expect("branch point");
    let (mut bgame, _) = branch.run_recorded().expect("prefix replays");
    let owner = bgame.decision_owner().unwrap();
    let step = ReplayStep {
        player: owner,
        revision: bgame.revision(),
        decision: bgame.decision(),
        action: Action::Mulligan { keep: false },
    };
    bgame
        .submit_action(owner, Action::Mulligan { keep: false })
        .expect("the mulligan choice is legal here");
    branch.record(step);
    assert_eq!(branch.steps.len(), 2, "the branch is mid-divergence");

    let diff = compare_branches(&replay, &branch).expect("same setup compares");
    assert_eq!(diff.diverged_at, Some(1));
    assert!(
        diff.base_tail_unrevealed,
        "the branch is shorter than the base, so its tail must be marked unrevealed"
    );
    assert!(
        diff.lines.iter().all(|line| line.step < branch.steps.len()),
        "no compared line may sit past the branch's live boundary: {:?}",
        diff.lines
    );
    let rendered = diff.render();
    assert!(
        rendered.contains("not revealed"),
        "the unrevealed base tail must be marked, not implied: {rendered}"
    );
    assert!(
        !rendered.contains(base_tail_kind),
        "the base's future action kind '{base_tail_kind}' must never appear on a live branch: {rendered}"
    );
    assert!(
        rendered.contains("mulligan choice"),
        "the compared steps still show the public action axis: {rendered}"
    );
}

#[test]
fn branching_then_playing_forward_stays_deterministic_under_the_same_seed() {
    let (_, replay, _) = recorded_line(2304);
    let branch = replay.branch_at(1).expect("branch");
    // Two independent replay runs of the same branch line agree exactly.
    let (a, log_a) = branch.run_recorded().expect("run A");
    let (b, log_b) = branch.run_recorded().expect("run B");
    assert_eq!(a.state, b.state);
    assert_eq!(log_a, log_b);
    // The branch is version 6 and round-trips through JSON as a bare replay.
    let restored = Replay::from_json(&branch.to_json()).expect("branch serializes");
    assert_eq!(restored, branch);
    // And the restored branch still replays to the same digest.
    let (c, _) = restored.run_recorded().expect("restored replays");
    assert_eq!(c.digest(), a.digest());
}
