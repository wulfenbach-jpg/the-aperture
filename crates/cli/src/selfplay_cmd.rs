//! `selfplay`: one deterministic game under the baseline heuristic (and
//! optionally the local Decider-2B), recorded as a replay and a JSON report.

use std::path::PathBuf;

use aperture_ai::{DeciderConfig, DeciderPolicy, HeuristicPolicy, Policy, play_to_end};
use aperture_core::{AiSettings, Event, Game, RULES_REVISION, Replay};
use serde_json::json;

use crate::decks;

pub struct Args {
    pub seed: u64,
    pub deck_a: PathBuf,
    pub deck_b: PathBuf,
    pub max_steps: usize,
    pub decider_seat: Option<u8>,
    pub decider_timeout_ms: u64,
    pub out: Option<PathBuf>,
    pub replay_out: Option<PathBuf>,
    pub verbose: bool,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let (deck_a, source_a) = decks::load(&args.deck_a, decks::BUNDLED_TEMUR, "Temur Beats")?;
    let (deck_b, source_b) = decks::load(&args.deck_b, decks::BUNDLED_IZZET, "Izzet Control")?;

    let ai = AiSettings {
        use_decider: args.decider_seat.is_some(),
        decider_timeout_ms: args.decider_timeout_ms,
        ai_seat: args.decider_seat.map(|seat| seat.min(1)).unwrap_or(1),
        ..AiSettings::default()
    };

    let mut policies: [Box<dyn Policy>; 2] = [
        Box::new(HeuristicPolicy::new()),
        Box::new(HeuristicPolicy::new()),
    ];
    let mut decider_note = None;
    if let Some(seat) = args.decider_seat {
        let seat = seat.min(1);
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
                    "decider request rejected ({err}); both seats use the heuristic"
                ));
            }
        }
    }

    let mut game = Game::new(args.seed, &deck_a, &deck_b);
    let mut replay = Replay::new(args.seed, deck_a.clone(), deck_b.clone(), ai.clone());
    let mut log: Vec<Event> = Vec::new();
    let outcome = play_to_end(
        &mut game,
        &mut policies,
        args.max_steps,
        &mut log,
        &mut replay,
    );
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
        Some(aperture_core::P0) => "P0 (deck A)",
        Some(_) => "P1 (deck B)",
        None => "none (truncated)",
    };
    let policy_label = policies
        .iter()
        .map(|p| p.name().to_string())
        .collect::<Vec<_>>()
        .join(" vs ");

    // Verify the replay against the live digest before reporting success.
    let replay_check = replay.run();
    let (replay_matches, replay_error) = match replay_check {
        Ok((replayed, _)) => (replayed.digest() == game.digest(), None),
        Err(err) => (false, Some(err.to_string())),
    };

    let report = json!({
        "schema": "aperture-selfplay/1",
        "rules_revision": RULES_REVISION,
        "seed": args.seed,
        "deck_a": {"name": deck_a.name, "source": source_a, "cards": deck_a.size()},
        "deck_b": {"name": deck_b.name, "source": source_b, "cards": deck_b.size()},
        "policies": policy_label,
        "decider": decider_note,
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
        "life": {"P0": game.state.player(aperture_core::P0).life, "P1": game.state.player(aperture_core::P1).life},
        "libraries": {"P0": game.state.player(aperture_core::P0).library.len(), "P1": game.state.player(aperture_core::P1).library.len()},
        "pool_size": aperture_core::catalog::pool_size(),
    });

    println!(
        "selfplay seed={} {} -> winner {} in {} steps ({turns} turns){}",
        args.seed,
        policy_label,
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
    Ok(())
}
