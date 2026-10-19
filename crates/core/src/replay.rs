//! Replay, save/load and branching.
//!
//! A replay is the game seed, both deck snapshots, the rules revision, and the
//! ordered list of accepted actions. Re-running it must reproduce the same
//! canonical state digest.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::{Action, Submission};
use crate::deck::DeckList;
use crate::engine::Game;
use crate::error::EngineError;
use crate::event::Event;
use crate::ids::PlayerId;
use crate::settings::AiSettings;
use crate::state::Outcome;

pub const REPLAY_VERSION: u32 = 1;
pub const SAVE_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayStep {
    pub player: PlayerId,
    pub revision: u64,
    pub decision: u64,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    pub version: u32,
    pub seed: u64,
    pub deck_a: DeckList,
    pub deck_b: DeckList,
    pub rules_revision: String,
    pub ai: AiSettings,
    pub steps: Vec<ReplayStep>,
    /// Outcome recorded when the actions were first played, if the game ended.
    pub outcome: Option<Outcome>,
    /// Set when the original run was cut off by a run cap. Not a result.
    pub truncated: Option<String>,
    pub final_digest: String,
}

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error("unsupported replay version {got} (this build writes {REPLAY_VERSION})")]
    Version { got: u32 },
    #[error("step {index} (P{player}, {kind}) was rejected: {source}")]
    Step {
        index: usize,
        player: PlayerId,
        kind: &'static str,
        #[source]
        source: EngineError,
    },
    #[error("replay ended early: digest {got} != recorded {expected}")]
    DigestMismatch { expected: String, got: String },
    #[error("replay error: {0}")]
    Other(String),
}

impl Replay {
    pub fn new(seed: u64, deck_a: DeckList, deck_b: DeckList, ai: AiSettings) -> Self {
        Replay {
            version: REPLAY_VERSION,
            seed,
            deck_a,
            deck_b,
            rules_revision: crate::RULES_REVISION.to_string(),
            ai,
            steps: Vec::new(),
            outcome: None,
            truncated: None,
            final_digest: String::new(),
        }
    }

    pub fn record(&mut self, step: ReplayStep) {
        self.steps.push(step);
    }

    pub fn run(&self) -> Result<(Game, Vec<Event>), ReplayError> {
        if self.version != REPLAY_VERSION {
            return Err(ReplayError::Version { got: self.version });
        }
        let (mut game, mut log) = Game::new_logged(self.seed, &self.deck_a, &self.deck_b);
        for (index, step) in self.steps.iter().enumerate() {
            let submission = Submission {
                player: step.player,
                revision: step.revision,
                decision: step.decision,
                action: step.action.clone(),
            };
            match game.submit(&submission) {
                Ok(events) => log.extend(events),
                Err(source) => {
                    return Err(ReplayError::Step {
                        index,
                        player: step.player,
                        kind: step.action.kind(),
                        source,
                    });
                }
            }
        }
        if !self.final_digest.is_empty() {
            let got = game.digest();
            if got != self.final_digest {
                return Err(ReplayError::DigestMismatch {
                    expected: self.final_digest.clone(),
                    got,
                });
            }
        }
        Ok((game, log))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("replay serializes")
    }

    pub fn from_json(text: &str) -> Result<Self, ReplayError> {
        serde_json::from_str(text).map_err(|e| ReplayError::Other(e.to_string()))
    }
}

/// A live session snapshot: the full engine state plus what produced it. Loading
/// restores pending decisions, RNG position and AI settings exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSave {
    pub version: u32,
    pub game: crate::state::GameState,
    pub deck_a: DeckList,
    pub deck_b: DeckList,
    pub ai: AiSettings,
    pub replay: Replay,
    pub log: Vec<Event>,
    /// True once a sandbox tool has been used; certified analysis is void from
    /// that point (the UI labels this).
    pub sandbox_modified: bool,
    pub digest: String,
}

impl SessionSave {
    pub fn capture(game: &Game, replay: Replay, log: Vec<Event>, sandbox_modified: bool) -> Self {
        SessionSave {
            version: SAVE_VERSION,
            game: game.state.clone(),
            deck_a: replay.deck_a.clone(),
            deck_b: replay.deck_b.clone(),
            ai: replay.ai.clone(),
            replay,
            log,
            sandbox_modified,
            digest: game.digest(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("session serializes")
    }

    pub fn from_json(text: &str) -> Result<Self, ReplayError> {
        let save: SessionSave =
            serde_json::from_str(text).map_err(|e| ReplayError::Other(e.to_string()))?;
        if save.version != SAVE_VERSION {
            return Err(ReplayError::Other(format!(
                "unsupported save version {}",
                save.version
            )));
        }
        Ok(save)
    }

    pub fn game(&self) -> Game {
        Game {
            state: self.game.clone(),
        }
    }
}
