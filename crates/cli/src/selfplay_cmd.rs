//! `selfplay`: one deterministic 2–4 seat game under the baseline heuristic
//! (and optionally the local Decider-2B), recorded as a replay and a JSON report.

use std::path::PathBuf;

use aperture_ai::{
    DeciderConfig, DeciderPolicy, HeuristicPolicy, Policy, RunStatus, SelfplayLimits, play_to_end,
};
use aperture_core::{AiSettings, Event, Game, RULES_REVISION, Replay};
use serde_json::json;

use crate::decks;

pub struct Args {
    pub seed: u64,
    /// One path per seat. Empty means the two bundled practice decks.
    pub decks: Vec<PathBuf>,
    pub max_steps: usize,
    pub max_millis: Option<u64>,
    pub decider_seat: Option<u8>,
    pub decider_timeout_ms: u64,
    pub out: Option<PathBuf>,
    pub replay_out: Option<PathBuf>,
    pub verbose: bool,
}

fn bundled_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from("decks/temur-beats.txt"),
        PathBuf::from("decks/izzet-control.txt"),
    ]
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let requested = if args.decks.is_empty() {
        bundled_paths()
    } else {
        args.decks
    };
    if !(aperture_core::MIN_PLAYERS..=aperture_core::MAX_PLAYERS).contains(&requested.len()) {
        anyhow::bail!(
            "selfplay needs 2..=4 --deck paths (got {})",
            requested.len()
        );
    }

    let mut lists = Vec::with_capacity(requested.len());
    let mut sources = Vec::with_capacity(requested.len());
    for (index, path) in requested.iter().enumerate() {
        let (fallback, name) = if index % 2 == 0 {
            (decks::BUNDLED_TEMUR, "Temur Beats")
        } else {
            (decks::BUNDLED_IZZET, "Izzet Control")
        };
        let (deck, source) = decks::load(path, fallback, name)?;
        lists.push(deck);
        sources.push(source);
    }
    let seats = lists.len();

    let ai = AiSettings {
        use_decider: args.decider_seat.is_some(),
        decider_timeout_ms: args.decider_timeout_ms,
        human_seat: 0,
        ai_seats: (0..seats as u8).collect(),
        ..AiSettings::default()
    };

    let mut policies: Vec<Box<dyn Policy>> = (0..seats)
        .map(|_| Box::new(HeuristicPolicy::new()) as Box<dyn Policy>)
        .collect();
    let mut decider_note = None;
    if let Some(seat) = args.decider_seat {
        if seat as usize >= seats {
            decider_note = Some(format!(
                "decider seat {seat} is outside this {seats}-seat game; all seats use the heuristic"
            ));
        } else {
            let config = DeciderConfig {
                timeout_ms: args.decider_timeout_ms,
                ..DeciderConfig::default()
            };
            match DeciderPolicy::new(config) {
                Ok(policy) => {
                    policies[seat as usize] = Box::new(policy);
                    decider_note = Some(format!(
                        "seat {seat} uses decider-2b with heuristic fallback"
                    ));
                }
                Err(err) => {
                    decider_note = Some(format!(
                        "decider request rejected ({err}); all seats use the heuristic"
                    ));
                }
            }
        }
    }

    let (mut game, mut log) =
        Game::new_logged(args.seed, &lists).map_err(|err| anyhow::anyhow!("{err}"))?;
    let mut replay = Replay::new(
        args.seed,
        game.state.first_player,
        lists.clone(),
        ai.clone(),
    )
    .map_err(|err| anyhow::anyhow!("{err}"))?;
    let limits = SelfplayLimits {
        max_steps: args.max_steps,
        max_millis: args.max_millis,
    };
    let outcome = play_to_end(&mut game, &mut policies, limits, &mut log, &mut replay);
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();

    if args.verbose {
        for event in &log {
            println!("{}", aperture_core::view::render_event(&game, 0, event));
        }
    }

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
    let blocks = log
        .iter()
        .filter(|e| matches!(e, Event::BlockersDeclared { blocks, .. } if !blocks.is_empty()))
        .count();
    let damage = log
        .iter()
        .filter(|e| matches!(e, Event::Damage { .. }))
        .count();
    let counters = log
        .iter()
        .filter(|e| matches!(e, Event::Countered { .. }))
        .count();
    let turns = game.state.turn.number;

    let winner = outcome.winner;
    let winner_label = match winner {
        Some(seat) => format!("P{seat}"),
        None => "none".to_string(),
    };
    let policy_label = policies
        .iter()
        .map(|p| p.name().to_string())
        .collect::<Vec<_>>()
        .join(" vs ");
    let status = match outcome.status {
        RunStatus::Terminal => "terminal",
        RunStatus::Capped => "capped",
        RunStatus::Failed => "failed",
    };

    // Verify the replay against the live digest before reporting success.
    let replay_check = replay.run();
    let (replay_matches, replay_error) = match replay_check {
        Ok((replayed, _)) => (replayed.digest() == game.digest(), None),
        Err(err) => (false, Some(err.to_string())),
    };

    let seat_json: Vec<_> = lists
        .iter()
        .enumerate()
        .map(|(index, deck)| {
            json!({
                "seat": index,
                "name": deck.name,
                "source": sources[index],
                "cards": deck.size(),
                "life": game.state.player(index as u8).life,
                "library": game.state.player(index as u8).library.len(),
            })
        })
        .collect();

    let report = json!({
        "schema": "aperture-selfplay/2",
        "rules_revision": RULES_REVISION,
        "seed": args.seed,
        "seat_count": seats,
        "first_player": game.state.first_player,
        "seats": seat_json,
        "policies": policy_label,
        "decider": decider_note,
        "status": status,
        "steps": outcome.steps,
        "turns": turns,
        "winner": winner.map(|w| w as u32),
        "winner_label": winner_label,
        "reason": outcome.reason,
        "truncated": outcome.truncated,
        "decision_sources": outcome.sources,
        "policy_notes": outcome.fallback_notes,
        "final_digest": outcome.digest,
        "replay_steps": replay.steps.len(),
        "replay_digest_matches": replay_matches,
        "replay_error": replay_error,
        "counts": {
            "events": log.len(),
            "casts": casts,
            "attacks": attacks,
            "blocks": blocks,
            "damage_events": damage,
            "counterspells": counters,
        },
        "pool_size": aperture_core::catalog::pool_size(),
    });

    println!(
        "selfplay seed={} seats={} {} -> status {} winner {} in {} steps ({turns} turns){}",
        args.seed,
        seats,
        policy_label,
        status,
        winner_label,
        outcome.steps,
        match &outcome.truncated {
            Some(reason) => format!(" TRUNCATED: {reason}"),
            None => String::new(),
        }
    );
    println!(
        "  digest {} | replay {} | casts {} attacks {} blocks {} counters {}",
        &game.digest()[..12],
        if replay_matches {
            "verified"
        } else {
            "MISMATCH"
        },
        casts,
        attacks,
        blocks,
        counters
    );

    if let Some(path) = &args.replay_out {
        std::fs::write(path, replay.to_json())?;
        println!("  replay written to {}", path.display());
    }
    if let Some(path) = &args.out {
        std::fs::write(path, serde_json::to_string_pretty(&report)? + "\n")?;
        println!("  report written to {}", path.display());
    }
    if !replay_matches {
        anyhow::bail!("replay verification failed: {replay_error:?}");
    }
    if outcome.status == RunStatus::Failed {
        anyhow::bail!("selfplay failed: {}", outcome.reason);
    }
    Ok(())
}
