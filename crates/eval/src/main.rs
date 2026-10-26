//! aperture-eval CLI.
//!
//! Usage:
//!   aperture-eval run <config.json> <outdir>     run the declared paired
//!                                                calibration and write raw
//!                                                report + replays
//!   aperture-eval verify <report.json>           re-verify every replay in a
//!                                                stored report against the
//!                                                engine digest, cross-check
//!                                                the declared seed x rotation
//!                                                plan and the summary counters
//!
//! Nothing is launched beyond local engine games: no services, downloads,
//! weights or network inference. Run from the repository root; fixture paths
//! in the config are relative to the CWD.
//!
//! Deterministic mode: set `search_max_millis: 0` in the config to neutralize
//! the search policy's wall-clock cutoff (passed as u64::MAX), so the search
//! budget is governed only by node/rollout counts and the games are a
//! reproducible function of (seed, config, code).

use std::fs;
use std::path::Path;

use aperture_eval::config::EvalConfig;
use aperture_eval::report::PRACTICE_DISCLOSURE;
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
        [cmd, config_path, outdir] if cmd == "run" => run(config_path, outdir),
        [cmd, report_path] if cmd == "verify" => verify(report_path),
        _ => {
            anyhow::bail!(
                "usage: aperture-eval run <config.json> <outdir> | aperture-eval verify <report.json>"
            )
        }
    }
}

fn run(config_path: &str, outdir: &str) -> anyhow::Result<()> {
    let config_text = fs::read_to_string(config_path)?;
    let parsed_sha = sha256_hex(config_text.as_bytes());
    let eval: EvalConfig = serde_json::from_str(&config_text)
        .map_err(|e| anyhow::anyhow!("config {config_path}: {e}"))?;
    // The config declares fixture paths relative to the CWD, documented as the
    // repository root.
    let repo_root = std::env::current_dir()?;
    let session = SessionConfig::from_files(eval, &repo_root)?;

    fs::create_dir_all(outdir)?;
    fs::create_dir_all(Path::new(outdir).join("replays"))?;

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
    let report = EvalReport::build(&session, games, Some(parsed_sha.clone()), Some(&repo_root));
    let mismatched = report.summary.replay_mismatch;
    let versioned_config = serde_json::to_string_pretty(&session.eval)?;
    fs::write(Path::new(outdir).join("config.json"), versioned_config)?;
    fs::write(Path::new(outdir).join("config-insha256.txt"), parsed_sha)?;
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
    let summary_text = format!(
        "{}\nrun_id: {}\nsource config file sha256: {} (raw input bytes)\ncanonical config sha256: {} (serde_json re-serialization)\n{}\n{}\nhardware: {} logical cpus, cpu {}, os {}, rustc {}, profile {}\ndriver commit: {}\ninvocation: aperture-eval run {} {} (cwd = repository root)\n\ngames: {} recorded / {} declared | terminal {} (search wins {}, losses {}, draws {}), capped {}, failed {}, policy rejected {}, replay mismatches {}\npaired design (2+ rotations, seat swap): {}\npaired diff vs fair odds: mean {mean}, CI95 [{lo}, {hi}]\n{}\n{}\npolicy note: {}\n",
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
        hw.git_commit.clone().unwrap_or_else(|| "n/a".to_string()),
        config_path,
        outdir,
        report.summary.games_recorded,
        report.summary.games_declared,
        report.summary.terminal,
        report.summary.terminal_search_wins,
        report.summary.terminal_search_losses,
        report.summary.terminal_draws,
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
        "wrote raw report to {outdir}/report.json ({} games)",
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

fn verify(report_path: &str) -> anyhow::Result<()> {
    let text = fs::read_to_string(report_path)?;
    let report: EvalReport =
        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("report {report_path}: {e}"))?;
    let mut verified = 0usize;
    let mut mismatched = 0usize;
    let mut declared: std::collections::BTreeMap<(u64, u32), bool> =
        std::collections::BTreeMap::new();
    for seed in &report.config.seeds {
        for rotation in 0..report.config.rotations {
            declared.insert((*seed, rotation), false);
        }
    }
    for game in &report.games {
        let key = (game.seed, game.rotation);
        if declared.insert(key, true).is_none() {
            anyhow::bail!(
                "plan violation: report contains undeclared game seed {} rot {}",
                game.seed,
                game.rotation
            );
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
    let missing: Vec<_> = declared
        .iter()
        .filter(|(_, present)| !**present)
        .map(|((s, r), _)| format!("seed {s} rot {r}"))
        .collect();
    if !missing.is_empty() {
        anyhow::bail!(
            "plan violation: {} declared game(s) missing from the report: {}",
            missing.len(),
            missing.join(", ")
        );
    }
    let s = &report.summary;
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
        declared.len(),
        report.schema,
        report.run_id
    );
    if mismatched > 0 {
        anyhow::bail!("replay verification failed for {mismatched} game(s)");
    }
    Ok(())
}
