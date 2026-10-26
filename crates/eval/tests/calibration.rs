//! Eval-driver calibration tests: the driver genuinely executes real engine
//! games, separates terminal from capped/failed, and every emitted replay
//! verifies against the engine's digest. Deliberately tiny budgets — this is a
//! pipeline proof, not the certified benchmark.

use aperture_eval::config::{DeckFixture, EvalConfig};
use aperture_eval::{EvalReport, SessionConfig, run_session};

fn tiny_config() -> EvalConfig {
    EvalConfig {
        run_id: "practice-driver-proof-tiny".to_string(),
        decks: vec![
            DeckFixture {
                name: "Temur Beats".to_string(),
                path: "decks/temur-beats.txt".to_string(),
            },
            DeckFixture {
                name: "Izzet Control".to_string(),
                path: "decks/izzet-control.txt".to_string(),
            },
        ],
        seeds: vec![77, 78],
        rotations: 2,
        max_steps: 4000,
        max_millis: 60_000,
        search_seeds: 1,
        search_node_budget: 400,
        search_max_rollout_nodes: 100,
        search_max_millis: 80,
        search_max_candidates: 6,
    }
}

#[test]
fn calibration_run_down_verifies_replays_and_counters() {
    let eval_root = std::env::current_dir().unwrap();
    // Cargo test runs with cwd = crates/eval; fixtures resolve from repo root.
    let repo = eval_root.join("..").join("..");
    let session = SessionConfig::from_files(tiny_config(), &repo).expect("session builds");
    // Fixture prompts are hashed for the report.
    assert_eq!(session.fixture_hashes.len(), 2);
    assert!(session.fixture_hashes.iter().all(|h| h.len() == 64));

    let games = run_session(&session).expect("driver runs");
    for game in &games {
        assert!(
            game.replay_verified,
            "replay must verify: {:?}",
            game.replay_error
        );
        assert_eq!(
            game.replay_error, None,
            "replay error, not a mismatch claim"
        );
        // A game shows status terminal/capped/failed and its step count is
        // counted, never silently omitted.
        assert!(
            game.status == "terminal" || game.status == "capped" || game.status == "failed",
            "unknown status {}",
            game.status
        );
        // When a search seat chose an action, either search decided it or an
        // honest labelled fallback did; neither may be silent.
        assert!(
            game.sources.contains_key("search")
                || game
                    .sources
                    .contains_key("heuristic (search-seat fallback)"),
            "search seat production must be auditable: {:?}",
            game.sources
        );
    }
    let report = EvalReport::build(&session, games);
    assert_eq!(report.games.len(), report.summary.games_recorded);
    assert_eq!(report.summary.games_recorded, report.summary.games_declared);
    // Practice disclosure is always attached verbatim.
    assert!(
        report
            .summary
            .disclosure
            .contains("PRACTICE CALIBRATION ONLY"),
        "summaries must carry the practice-only disclosure"
    );
    // Latency is recorded, >= 0, and search time never exceeds wall time.
    for game in &report.games {
        assert!(game.search_ms <= game.wall_ms + 5);
    }
}

#[test]
fn paired_stats_refuse_a_single_seed() {
    let sample = aperture_eval::stats::PairedSample {
        per_seed: vec![1.0],
    };
    assert!(sample.t_interval_95().is_err());
}
