//! `replay`: verify a saved session or a replay file by re-running it.

use std::path::{Path, PathBuf};

use aperture_core::{Replay, SessionSave, compare_branches};

/// Load a recorded line from a session save (embedded replay after full
/// validation) or a bare replay file.
fn load_line(
    _file: &Path,
    text: &str,
    value: &serde_json::Value,
) -> anyhow::Result<Option<Replay>> {
    if value.get("game").is_some() {
        let save = SessionSave::from_json(text).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Some(save.replay))
    } else {
        Ok(Some(
            Replay::from_json(text).map_err(|e| anyhow::anyhow!("{e}"))?,
        ))
    }
}

pub fn run(
    file: &Path,
    verbose: bool,
    branch_at: Option<usize>,
    branch_out: Option<PathBuf>,
    compare_with: Option<PathBuf>,
    save_out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(file)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;

    if let Some(other) = compare_with {
        let base = load_line(file, &text, &value)?.ok_or_else(|| {
            anyhow::anyhow!("the file does not contain a recorded line to compare")
        })?;
        let other_text = std::fs::read_to_string(&other)?;
        let other_value: serde_json::Value = serde_json::from_str(&other_text)?;
        let branch = load_line(&other, &other_text, &other_value)?.ok_or_else(|| {
            anyhow::anyhow!(
                "{} does not contain a recorded line to compare",
                other.display()
            )
        })?;
        let diff = compare_branches(&base, &branch)?;
        println!(
            "public accepted-action comparison ({file:?} vs {other:?}) — action kinds only, card names/targets/zone contents are never shown:"
        );
        print!("{}", diff.render());
        return Ok(());
    }

    if value.get("game").is_some() {
        let save = SessionSave::from_json(&text).map_err(|e| anyhow::anyhow!("{e}"))?;
        let replay = save.replay.clone();
        if let Some(keep) = branch_at {
            let branch = replay.branch_at(keep)?;
            let (branched, _) = branch.run_recorded()?;
            println!(
                "branch from {} at action #{keep}: {} seat(s), {} recorded action(s) kept, replays to digest {} under the same seed",
                file.display(),
                branch.seats.len(),
                branch.steps.len(),
                &branched.digest()[..12],
            );
            if let Some(out) = branch_out {
                std::fs::write(&out, branch.to_json())?;
                println!("  branch written to {}", out.display());
            }
            return Ok(());
        }
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
    if let Some(keep) = branch_at {
        let branch = replay.branch_at(keep)?;
        let (branched, _) = branch.run_recorded()?;
        println!(
            "branch from {} at action #{keep}: {} seat(s), {} recorded action(s) kept, replays to digest {} under the same seed",
            file.display(),
            branch.seats.len(),
            branch.steps.len(),
            &branched.digest()[..12],
        );
        if let Some(out) = branch_out {
            std::fs::write(&out, branch.to_json())?;
            println!("  branch written to {}", out.display());
        }
        return Ok(());
    }
    println!(
        "replay seed {} — {} seat(s), {} action(s), recorded outcome {:?}",
        replay.seed,
        replay.seats.len(),
        replay.steps.len(),
        replay.outcome
    );
    if let Some(out) = save_out {
        if replay.outcome.is_none() {
            anyhow::bail!("--save-out needs a replay that actually recorded a game outcome");
        }
        if replay.steps.is_empty() {
            anyhow::bail!("--save-out needs a replay with recorded actions");
        }
        let (game, log) = replay.run()?;
        let save = SessionSave::capture(&game, replay.clone(), log.clone(), false);
        std::fs::write(&out, save.to_json())?;
        println!(
            "  certified session save of the finished game written to {} (digest {})",
            out.display(),
            &save.digest[..12]
        );
    }
    if let Some(reason) = &replay.truncated {
        println!("  recorded as TRUNCATED (not a result): {reason}");
    }
    match replay.run() {
        Ok((game, events)) => {
            println!(
                "  replayed digest {} | {} | events {}",
                &game.digest()[..12],
                if replay.final_digest.is_empty() {
                    "no recorded digest on this line (a branch)".to_string()
                } else if game.digest() == replay.final_digest {
                    "matches recorded".to_string()
                } else {
                    "MISMATCH".to_string()
                },
                events.len()
            );
            if verbose {
                for event in &events {
                    println!("    {}", aperture_core::view::render_event(&game, 0, event));
                }
            }
            if !replay.final_digest.is_empty() && game.digest() != replay.final_digest {
                anyhow::bail!("replay digest mismatch");
            }
        }
        Err(err) => anyhow::bail!("replay failed: {err}"),
    }
    Ok(())
}
