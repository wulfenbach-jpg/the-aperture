//! aperture-eval CLI.
//!
//! Usage:
//!   aperture-eval run <config.json> <outdir>            run the declared
//!                                                       paired calibration
//!                                                       and write raw report
//!                                                       + replays + timings
//!   aperture-eval run ... --allow-dirty                 explicit escape hatch:
//!                                                       emit a report from a
//!                                                       dirty tree, stamped
//!                                                       unusable for claims
//!   aperture-eval verify <report.json>                  re-verify every replay
//!                                                       in a stored report
//!                                                       against the engine
//!                                                       digest, audit the
//!                                                       declared seed x rotation
//!                                                       plan (no undeclared /
//!                                                       duplicate / missing
//!                                                       records) and the
//!                                                       summary counters
//!
//! Dirty-tree gate (repair 3): a run REFUSES — before any game executes or any
//! evidence is written — when the working tree carries tracked modifications,
//! because then `hardware.git_commit` could not name the code that reproduces
//! the report. `--allow-dirty` stamps the report "DIRTY TREE - NOT
//! REPRODUCIBLE: NO value may support any claim" and suppresses the interval,
//! mean and Wilson readouts.
//!
//! Nothing is launched beyond local engine games: no services, downloads,
//! weights or network inference. Run from the repository root; fixture paths
//! in the config are relative to the CWD.
//!
//! Deterministic mode: set `search_max_millis: 0` in the config to neutralize
//! the search policy's wall-clock cutoff (passed as u64::MAX), so the search
//! budget is governed only by node/rollout counts and the games are a
//! reproducible function of (seed, config, code).
//!
//! Byte-reproducible scored report: per-game wall/verify/search timings are the
//! only non-deterministic bytes a run produces; they are written to the sibling
//! `timings.json` sidecar and kept OUT of `report.json` by schema design
//! (repair 3), so checking out the pinned commit and re-running reproduces
//! `report.json` byte-for-byte.

use std::fs;
use std::path::Path;

use aperture_eval::config::EvalConfig;
use aperture_eval::report::{LATENCY_SIDECAR_FILE, PRACTICE_DISCLOSURE};
use aperture_eval::{EvalReport, SessionConfig, run_session, sha256_hex};

fn main() {
    if let Err(err) = real_main() {
        eprintln!("aperture-eval: {err:#}");
        std::process::exit(1);
    }
}

fn real_main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, config_path, outdir] if cmd == "run" => run(config_path, outdir, false),
        [cmd, config_path, outdir, flag] if cmd == "run" && flag == "--allow-dirty" => {
            run(config_path, outdir, true)
        }
        [cmd, report_path] if cmd == "verify" => verify(report_path),
        _ => {
            anyhow::bail!(
                "usage: aperture-eval run <config.json> <outdir> [--allow-dirty] \
                 | aperture-eval verify <report.json>"
            )
        }
    }
}

fn run(config_path: &str, outdir: &str, allow_dirty: bool) -> anyhow::Result<()> {
    let config_text = fs::read_to_string(config_path)?;
    let parsed_sha = sha256_hex(config_text.as_bytes());
    let eval: EvalConfig = serde_json::from_str(&config_text)
        .map_err(|e| anyhow::anyhow!("config {config_path}: {e}"))?;
    // The config declares fixture paths relative to the CWD, documented as the
    // repository root.
    let repo_root = std::env::current_dir()?;
    let session = SessionConfig::from_files(eval, &repo_root)?;

    // Dirty-tree gate BEFORE any game runs or any evidence bytes are written:
    // a report whose git_commit cannot name the reproducing code is not
    // evidence (repair-3 blocking finding).
    let (_, _, tracked_dirty) = aperture_eval::report::git_provenance_probe(&repo_root);
    aperture_eval::report::dirty_tree_gate(tracked_dirty, allow_dirty)
        .map_err(anyhow::Error::msg)?;

    let games = run_session(&session)?;
    for game in &games {
        if !game.replay_verified {
            eprintln!(
                "REPLAY MISMATCH seed {} rot {}: {}",
                game.seed,
                game.rotation,
                game.replay_error.clone().unwrap_or_default()
            );
        }
    }
    let mut report = EvalReport::build(&session, games, Some(parsed_sha.clone()), Some(&repo_root));
    let mismatched = report.summary.replay_mismatch;
    if allow_dirty && report.hardware.git_dirty {
        // Explicit escape hatch: the evidence IS emitted but stamped unusable
        // for any claim (interval/mean/Wilson suppressed, headline marked).
        aperture_eval::report::mark_dirty_evidence(&mut report);
    }
    let versioned_config = serde_json::to_string_pretty(&session.eval)?;
    fs::write(Path::new(outdir).join("config.json"), versioned_config)?;
    fs::write(Path::new(outdir).join("config-insha256.txt"), parsed_sha)?;
    // Raw latency sidecar; report.json itself carries NO timing bytes.
    let sidecar = aperture_eval::report::latency_rows(&report.games);
    fs::write(
        Path::new(outdir).join(LATENCY_SIDECAR_FILE),
        serde_json::to_string_pretty(&sidecar)?,
    )?;
    for game in &report.games {
        let path = format!(
            "{}/replays/seed{:-020}-rot{}.json",
            outdir, game.seed, game.rotation
        );
        fs::write(&path, &game.replay_json)?;
    }
    let raw = serde_json::to_string_pretty(&report)?;
    fs::write(Path::new(outdir).join("report.json"), raw.clone())?;
    let hw = &report.hardware;
    let mean = fmt_maybe(report.summary.paired_mean_diff_fair_odds);
    let lo = fmt_maybe(report.summary.paired_ci95_low_diff_fair_odds);
    let hi = fmt_maybe(report.summary.paired_ci95_high_diff_fair_odds);
    let clean_tree = if hw.git_dirty { "no" } else { "yes" };
    let commit_line = hw
        .git_commit
        .clone()
        .unwrap_or_else(|| "n/a (no repository provenance)".to_string());
    let tree_line = if hw.git_tree.is_empty() {
        "n/a".to_string()
    } else {
        hw.git_tree.clone()
    };
    let summary_text = format!(
        "{}\nrun_id: {}\nsource config file sha256: {} (raw input bytes)\ncanonical config sha256: {} (serde_json re-serialization)\n{}\n{}\nhardware: {} logical cpus, cpu {}, os {}, rustc {}, profile {}\ndriver commit: {}\ndriver tree: {} ({})\nclean tracked tree: {}\ninstrumentation: raw per-game wall/verify/search timings are in {} next to this summary; report.json carries no timing bytes and is byte-reproducible from the pinned commit\ninvocation: aperture-eval run {} {} (cwd = repository root)\n\ngames: {} recorded / {} declared | terminal {} (search wins {}, losses {}, draws {}), terminal unverified {}, capped {}, failed {}, policy rejected {}, replay mismatches {}\npaired design (2+ rotations, seat swap): {}\npaired diff vs fair odds: mean {mean}, CI95 [{lo}, {hi}]\n{}\n{}\npolicy note: {}\n",
        PRACTICE_DISCLOSURE,
        report.run_id,
        report.config_input_sha256.clone().unwrap_or_default(),
        report.config_sha256,
        serde_json::to_string_pretty(&report.fixture_hashes)?,
        report.rules_revision,
        hw.logical_cpus,
        hw.cpu_model,
        hw.os,
        hw.rustc,
        hw.build_profile,
        commit_line,
        tree_line,
        clean_tree,
        clean_tree,
        LATENCY_SIDECAR_FILE,
        config_path,
        outdir,
        report.summary.games_recorded,
        report.summary.games_declared,
        report.summary.terminal,
        report.summary.terminal_search_wins,
        report.summary.terminal_search_losses,
        report.summary.terminal_draws,
        report.summary.terminal_unverified,
        report.summary.capped,
        report.summary.failed,
        report.summary.policy_rejected,
        report.summary.replay_mismatch,
        report.summary.paired_design,
        report.summary.paired_ci_note,
        report.summary.headline,
        report.frozen_policy_note,
    );
    fs::write(Path::new(outdir).join("SUMMARY.txt"), summary_text)?;
    println!("{}", report.summary.headline);
    println!(
        "wrote raw report to {outdir}/report.json ({} games), timings sidecar {outdir}/{LATENCY_SIDECAR_FILE}",
        report.summary.games_recorded
    );
    if mismatched > 0 {
        // The report is still written (full evidence retention), but the run
        // FAILS: any replay mismatch invalidates every scored outcome.
        anyhow::bail!("{mismatched} replay mismatch(es): scored outcomes withheld, run failed");
    }
    Ok(())
}

fn fmt_maybe(v: Option<f64>) -> String {
    v.map(|x| format!("{x:+.4}"))
        .unwrap_or_else(|| "n/a".to_string())
}

/// Pure plan audit for `verify`: the declared seed x rotation plan versus the
/// records actually present. Returns human-auditable violations for
/// undeclared, duplicate and missing records (a duplicate previously
/// overwrote the plan map instead of being rejected — repair-3 finding).
fn plan_violations(seeds: &[u64], rotations: u32, records: &[(u64, u32)]) -> Vec<String> {
    let mut declared: std::collections::BTreeSet<(u64, u32)> = std::collections::BTreeSet::new();
    for seed in seeds {
        for rotation in 0..rotations {
            declared.insert((*seed, rotation));
        }
    }
    let mut present: std::collections::BTreeSet<(u64, u32)> = std::collections::BTreeSet::new();
    let mut violations = Vec::new();
    for &(seed, rotation) in records {
        if !declared.contains(&(seed, rotation)) {
            violations.push(format!("undeclared game seed {seed} rot {rotation}"));
            continue;
        }
        if !present.insert((seed, rotation)) {
            violations.push(format!("duplicate record for seed {seed} rot {rotation}"));
        }
    }
    let missing: Vec<String> = declared
        .iter()
        .filter(|key| !present.contains(*key))
        .map(|(s, r)| format!("seed {s} rot {r}"))
        .collect();
    if !missing.is_empty() {
        violations.push(format!(
            "{} declared game(s) missing from the report: {}",
            missing.len(),
            missing.join(", ")
        ));
    }
    violations
}

fn verify(report_path: &str) -> anyhow::Result<()> {
    let text = fs::read_to_string(report_path)?;
    let report: EvalReport =
        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("report {report_path}: {e}"))?;
    let records: Vec<(u64, u32)> = report.games.iter().map(|g| (g.seed, g.rotation)).collect();
    let violations = plan_violations(&report.config.seeds, report.config.rotations, &records);
    if !violations.is_empty() {
        anyhow::bail!("plan violation: {}", violations.join("; "));
    }
    let declared = report.config.seeds.len() * report.config.rotations as usize;
    let mut verified = 0usize;
    let mut mismatched = 0usize;
    // Counters derived exactly like summarize() does, from the records' own
    // verified flags, so summary counters cannot drift from record reality.
    let mut expected_terminal = 0usize;
    let mut expected_unverified = 0usize;
    for game in &report.games {
        if game.status == "terminal" && game.replay_verified {
            expected_terminal += 1;
        } else if game.status == "terminal" {
            expected_unverified += 1;
        }
        match aperture_core::Replay::from_json(&game.replay_json).and_then(|r| r.run()) {
            Ok(_) => verified += 1,
            Err(e) => {
                mismatched += 1;
                eprintln!(
                    "seed {} rot {}: replay verification error: {e}",
                    game.seed, game.rotation
                );
            }
        }
    }
    // The runner's own "verified" flag is the same deterministic
    // replay-through-engine check, so a record whose stored flag and whose
    // actual replay outcome disagree is a forged record: expected_terminal
    // (flag counting) must equal the terminal-status games whose replays
    // actually validated.
    let terminal_replayed_ok = report
        .games
        .iter()
        .filter(|g| g.status == "terminal")
        .filter(|g| {
            aperture_core::Replay::from_json(&g.replay_json)
                .and_then(|r| r.run())
                .is_ok()
        })
        .count();
    if terminal_replayed_ok != expected_terminal {
        anyhow::bail!(
            "plan violation: recorded replay_verified flags disagree with the replay validation ({} terminal-status games flag-verified, {} actually validated)",
            expected_terminal,
            terminal_replayed_ok
        );
    }
    let s = &report.summary;
    if s.terminal != expected_terminal || s.terminal_unverified != expected_unverified {
        anyhow::bail!(
            "plan violation: summary terminal counters ({} verified-terminal, {} unverified-terminal) do not match the game records ({} verified-terminal, {} unverified-terminal)",
            s.terminal,
            s.terminal_unverified,
            expected_terminal,
            expected_unverified
        );
    }
    if s.policy_rejected > s.failed {
        anyhow::bail!(
            "plan violation: policy_rejected {} exceeds failed {}",
            s.policy_rejected,
            s.failed
        );
    }
    if s.games_recorded != report.games.len() {
        anyhow::bail!(
            "plan violation: games_recorded {} != present game records {}",
            s.games_recorded,
            report.games.len()
        );
    }
    println!(
        "verify: {} verified, {} mismatched of {} recorded games ({} declared; schema {}, run_id {})",
        verified,
        mismatched,
        report.games.len(),
        declared,
        report.schema,
        report.run_id
    );
    if mismatched > 0 {
        anyhow::bail!("replay verification failed for {mismatched} game(s)");
    }
    Ok(())
}

#[cfg(test)]
mod plan_audit_tests {
    use super::plan_violations;

    #[test]
    fn a_clean_plan_has_no_violations() {
        let v = plan_violations(&[101, 102], 2, &[(101, 0), (101, 1), (102, 0), (102, 1)]);
        assert!(v.is_empty(), "{v:?}");
    }

    /// Repair-3 finding: a duplicate (seed, rotation) record must be REJECTED,
    /// never silently overwrite the plan map.
    #[test]
    fn duplicate_records_are_rejected() {
        let v = plan_violations(&[101], 2, &[(101, 0), (101, 1), (101, 1), (101, 0)]);
        assert_eq!(v.len(), 2, "{v:?}");
        assert!(v[0].contains("duplicate record for seed 101 rot 1"));
        assert!(v[1].contains("duplicate record for seed 101 rot 0"));
    }

    #[test]
    fn undeclared_records_are_rejected() {
        let v = plan_violations(&[101], 2, &[(101, 0), (101, 1), (999, 0)]);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("undeclared game seed 999 rot 0"), "{v:?}");
    }

    #[test]
    fn missing_records_are_named() {
        let v = plan_violations(&[101, 102], 2, &[(101, 0), (102, 1)]);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(
            v[0].contains("2 declared game(s) missing")
                && v[0].contains("seed 101 rot 1")
                && v[0].contains("seed 102 rot 0"),
            "{v:?}"
        );
    }
}
