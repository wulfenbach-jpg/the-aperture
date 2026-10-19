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
    let status = if !replay_matches {
        "failed"
    } else {
        match outcome.status {
            RunStatus::Terminal => "terminal",
            RunStatus::Capped => "capped",
            RunStatus::Failed => "failed",
        }
    };
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

pub fn run(args: Args) -> anyhow::Result<()> {
    if args.seats.is_empty() {
        anyhow::bail!("harness needs at least one --seats value");
    }
    let limits = SelfplayLimits {
        max_steps: args.max_steps,
        max_millis: args.max_millis,
    };
    let mut rows = Vec::new();
    let mut failed = false;
    let mut capped = false;
    for seats in &args.seats {
        if !(aperture_core::MIN_PLAYERS..=aperture_core::MAX_PLAYERS).contains(seats) {
            anyhow::bail!("seat count {seats} is outside 2..=4");
        }
        for game in 0..args.games_per_seat {
            for pair in 0..2u64 {
                let seed = args.seed_base + game as u64 * 2 + pair;
                let row = run_one(seed, *seats, pair == 1, limits)?;
                match row.get("status").and_then(|s| s.as_str()) {
                    Some("failed") => failed = true,
                    Some("capped") => capped = true,
                    _ => {}
                }
                rows.push(row);
            }
        }
    }
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
            "failed": rows.iter().filter(|r| r.get("status").and_then(|s| s.as_str()) == Some("failed")).count(),
            "capped": rows.iter().filter(|r| r.get("status").and_then(|s| s.as_str()) == Some("capped")).count(),
            "terminal": rows.iter().filter(|r| r.get("status").and_then(|s| s.as_str()) == Some("terminal")).count(),
        }
    });
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
    if failed {
        anyhow::bail!("harness recorded failed run(s); see {}", args.out.display());
    }
    if capped {
        std::process::exit(2);
    }
    Ok(())
}
