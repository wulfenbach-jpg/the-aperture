//! aperture-eval CLI.
//!
//! Usage:
//!   aperture-eval run <config.json> <outdir>     run the declared paired
//!                                                calibration and write raw
//!                                                report + replays
//!   aperture-eval verify <report.json>           re-verify every replay in a
//!                                                stored report against the
//!                                                engine digest
//!
//! Nothing is launched beyond local engine games: no services, downloads,
//! weights or network inference.

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
    // Repo root = two levels above the config? No: the config declares fixture
    // paths relative to the CWD, which is the repo root (documented).
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
    let report = EvalReport::build(&session, games);
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
    let summary_text = format!(
        "{}\nrun_id: {}\nsource config sha256: {}\n{}\n{}\nhardware: {} logical cpus, os {}, profile {}\n\ngames: {} recorded / {} declared | terminal {} (search wins {}, losses {}, draws {}), capped {}, failed {}, replay mismatches {}\npaired: {} units; {} \n{}\npolicy note: {}\n",
        PRACTICE_DISCLOSURE,
        report.run_id,
        report.config_sha256,
        serde_json::to_string_pretty(&report.fixture_hashes)?,
        report.rules_revision,
        report.hardware.logical_cpus,
        report.hardware.os,
        report.hardware.build_profile,
        report.summary.games_recorded,
        report.summary.games_declared,
        report.summary.terminal,
        report.summary.terminal_search_wins,
        report.summary.terminal_search_losses,
        report.summary.terminal_draws,
        report.summary.capped,
        report.summary.failed,
        report.summary.replay_mismatch,
        report.summary.paired_units_seeds,
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
    Ok(())
}

fn verify(report_path: &str) -> anyhow::Result<()> {
    let text = fs::read_to_string(report_path)?;
    let report: EvalReport =
        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("report {report_path}: {e}"))?;
    let mut verified = 0usize;
    let mut mismatched = 0usize;
    for game in &report.games {
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
    println!(
        "verify: {} verified, {} mismatched of {} recorded games (schema {}, run_id {})",
        verified,
        mismatched,
        report.games.len(),
        report.schema,
        report.run_id
    );
    if mismatched > 0 {
        anyhow::bail!("replay verification failed for {mismatched} game(s)");
    }
    Ok(())
}
