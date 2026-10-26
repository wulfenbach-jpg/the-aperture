//! Eval-driver calibration tests: the driver genuinely executes real engine
//! games, separates terminal from capped/failed/policy-rejected, and every
//! emitted replay verifies against the engine's digest. Deliberately tiny
//! budgets — this is a pipeline proof, not the certified benchmark.
//!
//! Deterministic budgets: `search_max_millis: 0` neutralizes the search
//! policy's wall-clock cutoff, so identical configs and seeds must produce
//! identical lines (proven by the reproducibility test below).

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
        // Deterministic mode: no wall-clock cutoff inside search.
        search_max_millis: 0,
        search_max_candidates: 6,
    }
}

fn session_for(eval: EvalConfig) -> SessionConfig {
    // Cargo test runs with cwd = crates/eval; fixtures resolve from repo root.
    let repo = std::env::current_dir().unwrap().join("..").join("..");
    SessionConfig::from_files(eval, &repo).expect("session builds")
}

#[test]
fn calibration_run_down_verifies_replays_and_counters() {
    let session = session_for(tiny_config());
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
    let report = EvalReport::build(&session, games, None, None);
    assert_eq!(report.games.len(), report.summary.games_recorded);
    assert_eq!(report.summary.games_recorded, report.summary.games_declared);
    assert!(report.summary.paired_design);
    // Practice disclosure is always attached verbatim.
    assert!(
        report
            .summary
            .disclosure
            .contains("PRACTICE CALIBRATION ONLY"),
        "summaries must carry the practice-only disclosure"
    );
    // Latency is recorded and search time stays inside game wall time;
    // verification time is disclosed separately from game wall time.
    for game in &report.games {
        assert!(game.search_ms <= game.wall_ms + 5);
    }
}

/// Deterministic search budget => two runs of the same config + seed must
/// produce the exact same recorded line (digest), not just similar ones.
/// This is the regression test for the wall-clock-budget nondeterminism the
/// independent review reproduced (S6 / B4).
#[test]
fn deterministic_budget_reproduces_the_line_exactly() {
    let mut cfg = tiny_config();
    cfg.run_id = "practice-driver-proof-determinism".to_string();
    cfg.seeds = vec![101];
    cfg.max_millis = 0; // no whole-game wall cap either
    let session = session_for(cfg);
    let first = run_session(&session).expect("first run");
    let second = run_session(&session).expect("second run");
    assert_eq!(first.len(), second.len());
    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(a.digest, b.digest, "seed {} rot {}", a.seed, a.rotation);
        assert_eq!(
            a.sources, b.sources,
            "decision-source counts must be identical per game"
        );
    }
}

#[test]
fn paired_stats_refuse_a_single_seed() {
    let sample = aperture_eval::stats::PairedSample {
        per_seed: vec![1.0],
    };
    assert!(sample.t_interval_95().is_err());
}
