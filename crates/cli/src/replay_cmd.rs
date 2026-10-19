//! `replay`: verify a saved session or a replay file by re-running it.

use std::path::Path;

use aperture_core::{Replay, SessionSave};

pub fn run(file: &Path, verbose: bool) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(file)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;

    if value.get("game").is_some() {
        let save = SessionSave::from_json(&text).map_err(|e| anyhow::anyhow!("{e}"))?;
        let game = save.game();
        let recomputed = game.digest();
        let integrity = recomputed == save.digest;
        println!(
            "save {} — {} seat(s): {}",
            file.display(),
            save.seats.len(),
            save.seats
                .iter()
                .enumerate()
                .map(|(index, deck)| format!("P{index} '{}'", deck.name))
                .collect::<Vec<_>>()
                .join(" vs ")
        );
        println!(
            "  stored digest {} | recomputed {} | {}",
            &save.digest[..12],
            &recomputed[..12],
            if integrity { "matches" } else { "MISMATCH" }
        );
        println!(
            "  turn {} step {} | pending {:?} | sandbox modified: {}",
            save.game.turn.number,
            save.game.turn.step.name(),
            save.game.pending,
            save.sandbox_modified
        );
        println!("  ai: {}", save.ai.describe());
        if save.replay.steps.is_empty() {
            println!("  no recorded action sequence in this save; nothing to re-run");
        } else {
            match save.replay.run() {
                Ok((replayed, events)) => {
                    let ok = replayed.digest() == save.digest;
                    println!(
                        "  replay of {} action(s) -> digest {} | events {} | {}",
                        save.replay.steps.len(),
                        &replayed.digest()[..12],
                        events.len(),
                        if ok {
                            "reproduces the save"
                        } else {
                            "DIVERGED"
                        }
                    );
                    if verbose {
                        for event in &events {
                            println!(
                                "    {}",
                                aperture_core::view::render_event(&replayed, 0, event)
                            );
                        }
                    }
                    if !ok {
                        anyhow::bail!("replay diverged from the saved state");
                    }
                }
                Err(err) => anyhow::bail!("replay failed: {err}"),
            }
        }
        if !integrity {
            anyhow::bail!("save digest does not match its own state");
        }
        return Ok(());
    }

    let replay = Replay::from_json(&text).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "replay seed {} — {} seat(s), {} action(s), recorded outcome {:?}",
        replay.seed,
        replay.seats.len(),
        replay.steps.len(),
        replay.outcome
    );
    if let Some(reason) = &replay.truncated {
        println!("  recorded as TRUNCATED (not a result): {reason}");
    }
    match replay.run() {
        Ok((game, events)) => {
            println!(
                "  replayed digest {} | matches recorded: {} | events {}",
                &game.digest()[..12],
                game.digest() == replay.final_digest,
                events.len()
            );
            if verbose {
                for event in &events {
                    println!("    {}", aperture_core::view::render_event(&game, 0, event));
                }
            }
            if game.digest() != replay.final_digest {
                anyhow::bail!("replay digest mismatch");
            }
        }
        Err(err) => anyhow::bail!("replay failed: {err}"),
    }
    Ok(())
}
