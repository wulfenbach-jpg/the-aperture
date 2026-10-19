//! Self-play driver: runs a full game with policies, records every accepted
//! action into a replay, and reports truncation instead of a result when a cap
//! or a repetition guard stops it. Multi-seat games use one policy per seat.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use aperture_core::{Event, Game, Outcome, PlayerId, Replay, ReplayStep, Submission};

use crate::Policy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfplayLimits {
    pub max_steps: usize,
    /// Wall-clock bound for the whole run; `None` runs until the step bound.
    pub max_millis: Option<u64>,
}

impl Default for SelfplayLimits {
    fn default() -> Self {
        SelfplayLimits {
            max_steps: 4_000,
            max_millis: None,
        }
    }
}

/// Machine-readable classification: a capped run is not a result and a failed
/// run is an engine/driver inconsistency, never a fabricated winner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RunStatus {
    Terminal,
    Capped,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfplayOutcome {
    pub seat_count: usize,
    pub status: RunStatus,
    pub winner: Option<PlayerId>,
    pub reason: String,
    pub steps: usize,
    /// Set for a capped or failed run. A truncated run is not a result.
    pub truncated: Option<String>,
    pub digest: String,
    /// How many decisions each policy source produced (auditable, so a model
    /// that never answers cannot be reported as if it played).
    pub sources: BTreeMap<String, usize>,
    /// Reasons the model was not used, when any.
    pub fallback_notes: Vec<String>,
}

/// How many times the exact same state may recur before the run is treated as
/// non-terminating instead of looping forever.
const REPEAT_LIMIT: u8 = 3;

pub fn play_to_end(
    game: &mut Game,
    policies: &mut [Box<dyn Policy>],
    limits: SelfplayLimits,
    log: &mut Vec<Event>,
    replay: &mut Replay,
) -> SelfplayOutcome {
    let mut steps = 0usize;
    let mut repeats: BTreeMap<String, u8> = BTreeMap::new();
    let mut truncated = None;
    let mut failed = false;
    let mut sources: BTreeMap<String, usize> = BTreeMap::new();
    let mut fallback_notes: Vec<String> = Vec::new();
    let started = Instant::now();

    let seat_count = game.state.seat_count();
    if policies.len() != seat_count {
        failed = true;
        truncated = Some(format!(
            "driver supplied {} policy/policies for a {seat_count}-seat game",
            policies.len()
        ));
    }

    while !failed && truncated.is_none() && steps < limits.max_steps {
        if game.state.is_over() {
            break;
        }
        if let Some(max_millis) = limits.max_millis
            && started.elapsed() >= Duration::from_millis(max_millis)
        {
            truncated = Some(format!(
                "wall-clock cap of {max_millis} ms reached before the game ended"
            ));
            break;
        }
        let Some(owner) = game.decision_owner() else {
            failed = true;
            truncated =
                Some("no decision point was available although the game is not over".to_string());
            break;
        };
        let view = game.view_for(owner, log);
        let decision = policies[owner as usize].decide(&view);
        *sources
            .entry(decision.source.label().to_string())
            .or_insert(0) += 1;
        if let Some(note) = decision.note.clone()
            && fallback_notes.len() < 5
            && !fallback_notes.contains(&note)
        {
            fallback_notes.push(note);
        }
        let revision = game.revision();
        let decision_id = game.decision();
        let submission = Submission {
            player: owner,
            revision,
            decision: decision_id,
            action: decision.action.clone(),
        };
        match game.submit(&submission) {
            Ok(events) => {
                log.extend(events);
                replay.record(ReplayStep {
                    player: owner,
                    revision,
                    decision: decision_id,
                    action: decision.action,
                });
                steps += 1;
            }
            Err(err) => {
                // The policy produced something the engine refuses. Fall back to
                // passing and record why; never mutate the state silently.
                log.push(Event::Truncated {
                    reason: format!("P{owner} policy action was rejected ({err}); passing instead"),
                });
                let pass = Submission {
                    player: owner,
                    revision: game.revision(),
                    decision: game.decision(),
                    action: aperture_core::Action::Pass,
                };
                match game.submit(&pass) {
                    Ok(events) => {
                        log.extend(events);
                        replay.record(ReplayStep {
                            player: owner,
                            revision: pass.revision,
                            decision: pass.decision,
                            action: pass.action,
                        });
                        steps += 1;
                    }
                    Err(second) => {
                        failed = true;
                        truncated =
                            Some(format!("no legal action available for P{owner}: {second}"));
                        break;
                    }
                }
            }
        }
        let digest = game.digest();
        let seen = repeats.entry(digest).or_insert(0);
        *seen += 1;
        if *seen >= REPEAT_LIMIT {
            truncated = Some(format!(
                "state repetition detected at step {steps} (digest {}); stopping instead of looping",
                &game.digest()[..12]
            ));
            break;
        }
    }

    if !failed && truncated.is_none() && !game.state.is_over() {
        truncated = Some(format!(
            "step cap of {} reached before the game ended",
            limits.max_steps
        ));
    }
    if let Some(reason) = &truncated {
        log.push(Event::Truncated {
            reason: reason.clone(),
        });
        replay.truncated = Some(reason.clone());
    }

    let (winner, reason) = match &game.state.outcome {
        Some(Outcome::Win { winner, reason }) => (Some(*winner), reason.clone()),
        Some(Outcome::Draw { reason }) => (None, reason.clone()),
        None => (None, String::new()),
    };
    let status = if failed {
        RunStatus::Failed
    } else if game.state.is_over() {
        RunStatus::Terminal
    } else {
        RunStatus::Capped
    };

    SelfplayOutcome {
        seat_count,
        status,
        winner,
        reason,
        steps,
        truncated,
        digest: game.digest(),
        sources,
        fallback_notes,
    }
}
