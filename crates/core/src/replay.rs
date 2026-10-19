//! Replay, save/load and branching.
//!
//! A replay is the game seed, the ordered per-seat deck snapshots, the rules
//! revision, and the ordered list of accepted actions. Re-running it must
//! reproduce the same canonical state digest. Version 2 is the multi-seat
//! format; version-1 two-seat files are explicitly rejected, never reinterpreted.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::{Action, Submission};
use crate::deck::DeckList;
use crate::engine::Game;
use crate::error::EngineError;
use crate::event::Event;
use crate::ids::{MAX_PLAYERS, MIN_PLAYERS, PlayerId};
use crate::settings::AiSettings;
use crate::state::Outcome;

pub const REPLAY_VERSION: u32 = 2;
pub const SAVE_VERSION: u32 = 3;

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
    pub first_player: PlayerId,
    /// Ordered seat snapshots; index == seat. The length is the seat count.
    pub seats: Vec<DeckList>,
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
    #[error(
        "unsupported replay/save version {got}; this build writes replay {expected}. \
         Older two-seat files are not silently reinterpreted as multi-seat files"
    )]
    Version { got: u32, expected: u32 },
    #[error("unsupported seat count {got}; supported range is {MIN_PLAYERS}..={MAX_PLAYERS}")]
    SeatCount { got: usize },
    #[error("invalid seat layout: {reason}")]
    SeatOrder { reason: String },
    #[error("recorded first player P{expected} does not match the re-created game's P{got}")]
    FirstPlayerMismatch { expected: PlayerId, got: PlayerId },
    #[error("could not construct the recorded game: {source}")]
    Setup {
        #[source]
        source: EngineError,
    },
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
    #[error("save digest {stored} does not match its own state {recomputed}")]
    SaveDigestMismatch {
        stored: String,
        recomputed: String,
    },
    #[error("replay/save record is malformed: {detail}")]
    Malformed { detail: String },
}

#[derive(Deserialize)]
struct VersionProbe {
    version: Option<u32>,
}

fn probe_version(text: &str, expected: u32) -> Result<(), ReplayError> {
    let probe: VersionProbe =
        serde_json::from_str(text).map_err(|e| ReplayError::Malformed {
            detail: e.to_string(),
        })?;
    let got = probe.version.ok_or_else(|| ReplayError::Malformed {
        detail: "missing version field".to_string(),
    })?;
    if got != expected {
        return Err(ReplayError::Version { got, expected });
    }
    Ok(())
}

impl Replay {
    pub fn new(
        seed: u64,
        first_player: PlayerId,
        seats: Vec<DeckList>,
        ai: AiSettings,
    ) -> Result<Self, ReplayError> {
        let replay = Replay {
            version: REPLAY_VERSION,
            seed,
            first_player,
            seats,
            rules_revision: crate::RULES_REVISION.to_string(),
            ai,
            steps: Vec::new(),
            outcome: None,
            truncated: None,
            final_digest: String::new(),
        };
        replay.validate()?;
        Ok(replay)
    }

    fn validate(&self) -> Result<(), ReplayError> {
        let got = self.seats.len();
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&got) {
            return Err(ReplayError::SeatCount { got });
        }
        if self.first_player as usize >= got {
            return Err(ReplayError::SeatOrder {
                reason: format!(
                    "first player P{} is outside the {got}-seat game",
                    self.first_player
                ),
            });
        }
        self.ai
            .validate_for(got)
            .map_err(|reason| ReplayError::SeatOrder { reason })?;
        Ok(())
    }

    pub fn record(&mut self, step: ReplayStep) {
        self.steps.push(step);
    }

    pub fn run(&self) -> Result<(Game, Vec<Event>), ReplayError> {
        if self.version != REPLAY_VERSION {
            return Err(ReplayError::Version {
                got: self.version,
                expected: REPLAY_VERSION,
            });
        }
        self.validate()?;
        let (mut game, mut log) = Game::new_logged(self.seed, &self.seats)
            .map_err(|source| ReplayError::Setup { source })?;
        if game.state.first_player != self.first_player {
            return Err(ReplayError::FirstPlayerMismatch {
                expected: self.first_player,
                got: game.state.first_player,
            });
        }
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
        probe_version(text, REPLAY_VERSION)?;
        let replay: Replay =
            serde_json::from_str(text).map_err(|e| ReplayError::Malformed {
                detail: e.to_string(),
            })?;
        replay.validate()?;
        Ok(replay)
    }
}

/// A live session snapshot: the full engine state plus what produced it. Loading
/// restores pending decisions, RNG position and AI settings exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSave {
    pub version: u32,
    pub game: crate::state::GameState,
    /// Ordered deck snapshots; must match `game.players` and `replay.seats`.
    pub seats: Vec<DeckList>,
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
            seats: replay.seats.clone(),
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
        probe_version(text, SAVE_VERSION)?;
        let save: SessionSave =
            serde_json::from_str(text).map_err(|e| ReplayError::Malformed {
                detail: e.to_string(),
            })?;
        save.validate()?;
        Ok(save)
    }

    fn validate(&self) -> Result<(), ReplayError> {
        let seats = self.seats.len();
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&seats) {
            return Err(ReplayError::SeatCount { got: seats });
        }
        if self.game.players.len() != seats {
            return Err(ReplayError::SeatOrder {
                reason: format!(
                    "save lists {seats} deck snapshot(s) but its state has {} seat(s)",
                    self.game.players.len()
                ),
            });
        }
        self.game
            .validate()
            .map_err(|source| ReplayError::Setup { source })?;
        self.ai
            .validate_for(seats)
            .map_err(|reason| ReplayError::SeatOrder { reason })?;
        if self.replay.version != REPLAY_VERSION {
            return Err(ReplayError::Version {
                got: self.replay.version,
                expected: REPLAY_VERSION,
            });
        }
        if self.replay.seats != self.seats {
            return Err(ReplayError::SeatOrder {
                reason: "save deck snapshots do not match the embedded replay".to_string(),
            });
        }
        if self.replay.seed != self.game.seed {
            return Err(ReplayError::SeatOrder {
                reason: "save seed does not match the embedded replay".to_string(),
            });
        }
        let recomputed = crate::digest::digest(&self.game);
        if recomputed != self.digest {
            return Err(ReplayError::SaveDigestMismatch {
                stored: self.digest.clone(),
                recomputed,
            });
        }
        Ok(())
    }

    pub fn game(&self) -> Game {
        Game {
            state: self.game.clone(),
        }
    }
}
