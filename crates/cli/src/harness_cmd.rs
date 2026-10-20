//! `harness`: bounded 2/3/4-seat headless regression games with paired seeds,
//! replay-digest verification and explicit terminal/capped/failed accounting.

use std::path::PathBuf;

use aperture_ai::{HeuristicPolicy, Policy, RunStatus, SelfplayLimits, play_to_end};
use aperture_core::{AiSettings, DeckList, Game, Replay};
use serde_json::json;

use crate::decks;

pub struct Args {
    pub seats: Vec<usize>,
    pub seed_base: u64,
    pub games_per_seat: usize,
    pub max_steps: usize,
    pub max_millis: Option<u64>,
    pub out: PathBuf,
}

fn deck(seat: usize, rotated: bool) -> anyhow::Result<DeckList> {
    let index = if rotated { seat + 1 } else { seat };
    let (text, name) = if index % 2 == 0 {
        (decks::BUNDLED_TEMUR, "Temur Beats")
    } else {
        (decks::BUNDLED_IZZET, "Izzet Control")
    };
    DeckList::parse_practice(name, text).map_err(|err| anyhow::anyhow!("bundled {name}: {err}"))
}

fn classify(replay_matches: bool, status: RunStatus) -> &'static str {
    if !replay_matches {
        return "failed";
    }
    match status {
        RunStatus::Terminal => "terminal",
        RunStatus::Capped => "capped",
        RunStatus::Failed => "failed",
    }
}

/// Failure takes precedence over a capped run; both are distinct from success.
fn exit_code(failed: usize, capped: usize) -> i32 {
    if failed > 0 {
        1
    } else if capped > 0 {
        2
    } else {
        0
    }
}

fn run_one(
    seed: u64,
    seats: usize,
    rotated: bool,
    limits: SelfplayLimits,
) -> anyhow::Result<serde_json::Value> {
    let lists: Vec<DeckList> = (0..seats)
        .map(|seat| deck(seat, rotated))
        .collect::<anyhow::Result<_>>()?;
    let ai = AiSettings {
        human_seat: 0,
        ai_seats: (0..seats as u8).collect(),
        ..AiSettings::default()
    };
    let (mut game, mut log) =
        Game::new_logged(seed, &lists).map_err(|err| anyhow::anyhow!("{err}"))?;
    let mut replay = Replay::new(seed, game.state.first_player, lists.clone(), ai)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let mut policies: Vec<Box<dyn Policy>> = (0..seats)
        .map(|_| Box::new(HeuristicPolicy::new()) as Box<dyn Policy>)
        .collect();
    let outcome = play_to_end(&mut game, &mut policies, limits, &mut log, &mut replay);
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();
    let replay_check = replay.run();
    let (replay_matches, replay_error) = match replay_check {
        Ok((replayed, _)) => (replayed.digest() == game.digest(), None),
        Err(err) => (false, Some(err.to_string())),
    };
    let status = classify(replay_matches, outcome.status);
    Ok(json!({
        "seed": seed,
        "seats": seats,
        "rotated_decks": rotated,
        "decks": lists.iter().map(|d| d.name.clone()).collect::<Vec<_>>(),
        "status": status,
        "winner": outcome.winner,
        "reason": outcome.reason,
        "steps": outcome.steps,
        "turns": game.state.turn.number,
        "digest": game.digest(),
        "replay_digest_matches": replay_matches,
        "replay_error": replay_error,
        "truncated": outcome.truncated,
        "decision_sources": outcome.sources,
    }))
}

fn status_count(rows: &[serde_json::Value], status: &str) -> usize {
    rows.iter()
        .filter(|row| row.get("status").and_then(|s| s.as_str()) == Some(status))
        .count()
}

/// Run the harness and return its machine-readable summary plus the real exit
/// code. `run` owns file writing and process exit so tests can exercise the
/// classifier without a hidden production fault flag.
pub(crate) fn execute(args: &Args) -> anyhow::Result<(serde_json::Value, i32)> {
    if args.seats.is_empty() {
        anyhow::bail!("harness needs at least one --seats value");
    }
    let limits = SelfplayLimits {
        max_steps: args.max_steps,
        max_millis: args.max_millis,
    };
    let mut rows = Vec::new();
    for seats in &args.seats {
        if !(aperture_core::MIN_PLAYERS..=aperture_core::MAX_PLAYERS).contains(seats) {
            anyhow::bail!("seat count {seats} is outside 2..=4");
        }
        for game in 0..args.games_per_seat {
            for pair in 0..2u64 {
                let seed = args.seed_base + game as u64 * 2 + pair;
                rows.push(run_one(seed, *seats, pair == 1, limits)?);
            }
        }
    }
    let failed = status_count(&rows, "failed");
    let capped = status_count(&rows, "capped");
    let terminal = status_count(&rows, "terminal");
    let summary = json!({
        "schema": "aperture-harness/1",
        "rules_revision": aperture_core::RULES_REVISION,
        "seed_base": args.seed_base,
        "games_per_seat": args.games_per_seat,
        "max_steps": args.max_steps,
        "max_millis": args.max_millis,
        "rows": rows,
        "counts": {
            "total": rows.len(),
            "failed": failed,
            "capped": capped,
            "terminal": terminal,
        }
    });
    Ok((summary, exit_code(failed, capped)))
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let (summary, code) = execute(&args)?;
    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, serde_json::to_string_pretty(&summary)? + "\n")?;
    println!(
        "harness: {} run(s) — terminal {}, capped {}, failed {} → {}",
        summary["counts"]["total"],
        summary["counts"]["terminal"],
        summary["counts"]["capped"],
        summary["counts"]["failed"],
        args.out.display()
    );
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(seats: Vec<usize>, seed_base: u64, max_steps: usize) -> Args {
        Args {
            seats,
            seed_base,
            games_per_seat: 1,
            max_steps,
            max_millis: None,
            out: PathBuf::from("unused-harness.json"),
        }
    }

    #[test]
    fn classifier_and_exit_code_are_explicit() {
        // Synthetic classifier checks: no legal pool game is forced to fail.
        assert_eq!(classify(true, RunStatus::Terminal), "terminal");
        assert_eq!(classify(true, RunStatus::Capped), "capped");
        assert_eq!(classify(true, RunStatus::Failed), "failed");
        assert_eq!(classify(false, RunStatus::Terminal), "failed");
        assert_eq!(exit_code(1, 0), 1);
        assert_eq!(exit_code(0, 1), 2);
        assert_eq!(exit_code(0, 0), 0);
        assert_eq!(exit_code(1, 1), 1, "failure takes precedence");
    }

    #[test]
    fn capped_harness_emits_paired_rows_and_exit_2() {
        let (summary, code) = execute(&args(vec![2, 3, 4], 9_100, 5)).expect("harness");
        assert_eq!(code, 2);
        assert_eq!(summary["counts"]["total"], 6);
        assert_eq!(summary["counts"]["capped"], 6);
        assert_eq!(summary["counts"]["failed"], 0);
        assert_eq!(summary["counts"]["terminal"], 0);
        let rows = summary["rows"].as_array().expect("rows");
        assert_eq!(rows.len(), 6);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row["seed"], json!(9_100 + (index % 2) as u64));
            assert_eq!(row["rotated_decks"], json!(index % 2 == 1));
            assert_eq!(row["replay_digest_matches"], json!(true));
            assert_eq!(row["status"], json!("capped"));
        }
    }

    #[test]
    fn terminal_harness_emits_rows_and_exit_0() {
        let (summary, code) = execute(&args(vec![2, 3, 4], 9_200, 4_000)).expect("harness");
        assert_eq!(code, 0);
        assert_eq!(summary["counts"]["total"], 6);
        assert_eq!(summary["counts"]["terminal"], 6);
        assert_eq!(summary["counts"]["failed"], 0);
        assert_eq!(summary["counts"]["capped"], 0);
        for row in summary["rows"].as_array().expect("rows") {
            assert_eq!(row["replay_digest_matches"], json!(true));
            assert_eq!(row["status"], json!("terminal"));
        }
    }
}
