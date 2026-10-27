//! Run-path regression for the dirty-tree/provenance gate (repair 4).
//!
//! The gate is applied inside the CLI's `run()` (`src/main.rs`), BEFORE any
//! game executes or any evidence byte is written. Until this file existed the
//! only coverage was the helper test in `src/report.rs` (`dirty_tree_gate`)
//! plus `plan_violations`: deleting the two gate statements in `run()` left
//! the whole suite green. These tests drive the REAL binary
//! (`env!("CARGO_BIN_EXE_aperture-eval")`) in throwaway git repositories so
//! removing the gate application breaks them:
//!
//! - Case A asserts the run EXITS NONZERO and that `outdir_A` does not exist;
//!   if `dirty_tree_gate`/`git_provenance_probe` are no longer called from
//!   `run()`, the run completes instead and BOTH the exit-status assertion and
//!   the no-output-directory assertion fail.
//! - Case B pins the `--allow-dirty` escape hatch end to end on the run path:
//!   report written, `hardware.git_dirty: true`, DIRTY TREE headline, mean /
//!   confidence-interval / Wilson readouts all null.
//! - Case C covers a directory that is not a git repository at all
//!   (unnameable provenance is treated as dirty and refused).
//!
//! Nothing here touches the reviewed worktree: every case runs in its own
//! uniquely named directory under the system temp dir, with the bundled
//! practice decks copied in. The raw report for Case B comes from ONE tiny
//! game (single seed, single rotation, 60-step cap) — the game's outcome
//! (capped/terminal) is deliberately not asserted, because the gate contract
//! is about provenance, not play.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIG_ONE_GAME: &str = r#"{
  "run_id": "run-gate-regression-tiny",
  "decks": [
    { "name": "Temur Beats", "path": "decks/temur-beats.txt" },
    { "name": "Izzet Control", "path": "decks/izzet-control.txt" }
  ],
  "seeds": [1],
  "rotations": 1,
  "max_steps": 60,
  "max_millis": 60000,
  "search_seeds": 1,
  "search_node_budget": 200,
  "search_max_rollout_nodes": 50,
  "search_max_millis": 0,
  "search_max_candidates": 6
}"#;

/// Run `git <args> -C <dir>`; must succeed (failures abort the test).
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        status.status.success(),
        "git {:?} failed in {}: {}",
        args,
        dir.display(),
        String::from_utf8_lossy(&status.stderr)
    );
}

/// A fresh, uniquely named directory that does not collide across re-runs.
fn fresh_temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .subsec_nanos();
    let dir = std::env::temp_dir().join(format!(
        "aperture-eval-run-gate-{}-{}-{}",
        std::process::id(),
        nanos,
        tag
    ));
    fs::create_dir_all(&dir).expect("temp dir created");
    dir
}

/// Copy the bundled practice deck fixtures and the one-game config into
/// `dir` (fixture paths are relative to the CLI's CWD). Used for both git and
/// non-git cases, so committing is left to the caller.
fn stage_cases(dir: &Path) {
    let decks = dir.join("decks");
    fs::create_dir_all(&decks).expect("decks dir");
    let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("decks");
    for fixture in ["temur-beats.txt", "izzet-control.txt"] {
        fs::copy(bundled.join(fixture), decks.join(fixture)).expect("deck fixture copied");
    }
    fs::write(dir.join("config.json"), CONFIG_ONE_GAME).expect("config written");
}

/// Track and commit everything in `dir` so the tree starts clean.
fn commit_all(dir: &Path) {
    git(dir, &["init", "-q", "."]);
    git(dir, &["add", "-A"]);
    git(
        dir,
        &[
            "-c",
            "user.email=eval-test@example.invalid",
            "-c",
            "user.name=eval-test",
            "commit",
            "-q",
            "-m",
            "run-gate test fixture",
        ],
    );
}

/// Drive the REAL CLI binary from inside `cwd`.
fn eval_run(cwd: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_aperture-eval"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("aperture-eval binary runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// Dirty one tracked file so the committed repo carries a tracked
/// modification (untracked files never trip the gate).
fn dirty_a_tracked_file(repo: &Path) {
    let deck = repo.join("decks").join("temur-beats.txt");
    let mut text = fs::read_to_string(&deck).expect("tracked deck read");
    text.push_str("\n// run-gate regression: tracked modification\n");
    fs::write(&deck, text).expect("tracked deck modified");
}

#[test]
fn run_path_dirty_gate_refuses_before_writing_any_evidence_and_stamps_the_escape_hatch() {
    // ------------------------------------------------------------------
    // One shared dirty git repository (copies of the bundled practice decks
    // are tracked) for cases A and B.
    // ------------------------------------------------------------------
    let repo = fresh_temp_dir("dirty-repo");
    stage_cases(&repo);
    commit_all(&repo);
    dirty_a_tracked_file(&repo);

    // ------------------------------------------------------------------
    // Case A — dirty tracked tree WITHOUT the flag: the run path itself must
    // refuse BEFORE any game runs and before any output directory or report
    // is created. If the `dirty_tree_gate` call were removed from `run()`,
    // the run would complete and BOTH the exit-status check and the
    // no-output-directory check below would fail.
    // ------------------------------------------------------------------
    let outdir_a = repo.join("outdir-A");
    let (ok_a, _stdout_a, stderr_a) = eval_run(&repo, &["run", "config.json", "outdir-A"]);
    assert!(
        !ok_a,
        "a dirty tracked tree must make the run path exit nonzero"
    );
    assert!(
        !outdir_a.exists(),
        "no output directory may be created for a refused dirty-tree run"
    );
    assert!(
        stderr_a.contains("refusing to write evidence"),
        "the refusal must come from the dirty-tree gate, got: {stderr_a}"
    );

    // ------------------------------------------------------------------
    // Case B — same dirty tree WITH `--allow-dirty`: evidence IS written, but
    // stamped unusable for any claim on the run path: git_dirty pin, DIRTY
    // TREE headline, and null mean / confidence interval / Wilson readouts.
    // ------------------------------------------------------------------
    let outdir_b = repo.join("outdir-B");
    let (ok_b, stdout_b, _stderr_b) =
        eval_run(&repo, &["run", "config.json", "outdir-B", "--allow-dirty"]);
    assert!(
        ok_b,
        "the escape hatch must still emit evidence: {stdout_b}"
    );
    let report_text = fs::read_to_string(outdir_b.join("report.json"))
        .expect("escape-hatch run must write a raw report");
    let report: serde_json::Value = serde_json::from_str(&report_text).expect("report parses");
    assert_eq!(
        report["hardware"]["git_dirty"], true,
        "the escape-hatch report must pin git_dirty: true"
    );
    let headline = report["summary"]["headline"].as_str().expect("headline");
    assert!(
        headline.contains("DIRTY TREE"),
        "the escape-hatch headline must carry the DIRTY TREE stamp: {headline}"
    );
    assert!(
        report_text.contains("DIRTY TREE"),
        "stamped headline stored"
    );
    for field in [
        "paired_mean_diff_fair_odds",
        "paired_ci95_low_diff_fair_odds",
        "paired_ci95_high_diff_fair_odds",
        "wilson_low",
        "wilson_high",
    ] {
        assert_eq!(
            report["summary"][field],
            serde_json::Value::Null,
            "dirty-tree evidence must suppress the {field} readout"
        );
    }

    // ------------------------------------------------------------------
    // Case C — a plain directory that is not a git repository: provenance is
    // unnameable, which the gate treats as dirty and refuses. Same coupling as
    // Case A: with the gate removed from `run()`, this run would complete and
    // the exit-status / no-output-directory checks would fail.
    // ------------------------------------------------------------------
    let plain = fresh_temp_dir("not-a-repo");
    stage_cases(&plain);
    let outdir_c = plain.join("outdir-C");
    let (ok_c, _stdout_c, stderr_c) = eval_run(&plain, &["run", "config.json", "outdir-C"]);
    assert!(
        !ok_c,
        "unnameable provenance (not a git repo) must make the run path exit nonzero"
    );
    assert!(
        !outdir_c.exists(),
        "no output directory may be created for a refused non-git run"
    );
    assert!(
        stderr_c.contains("refusing to write evidence"),
        "the refusal must come from the dirty-tree gate, got: {stderr_c}"
    );
}
