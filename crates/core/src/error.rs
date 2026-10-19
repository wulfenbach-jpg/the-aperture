use thiserror::Error;

use crate::action::Action;
use crate::card::{CardId, ManaColor};
use crate::ids::PlayerId;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    #[error("the game is already over")]
    GameOver,

    #[error("no player is entitled to act right now")]
    NoDecisionPoint,

    #[error("it is not P{expected}'s decision (submitted for P{got})")]
    NotYourDecision { expected: PlayerId, got: PlayerId },

    #[error("stale submission: expected revision {expected}, got {got}")]
    StaleRevision { expected: u64, got: u64 },

    #[error("stale decision: expected decision {expected}, got {got}")]
    StaleDecision { expected: u64, got: u64 },

    #[error("action {action} is not legal here: {reason}")]
    Illegal {
        action: &'static str,
        reason: String,
    },

    #[error("object does not exist")]
    UnknownObject,

    #[error("{name} is not in the expected zone ({zone})")]
    WrongZone {
        name: &'static str,
        zone: &'static str,
    },

    #[error("cannot pay {cost}; available {available}")]
    CannotPay { cost: String, available: String },

    #[error("{name} cannot be cast right now: {reason}")]
    BadTiming { name: &'static str, reason: String },

    #[error("target is not legal for {card}: {reason}")]
    BadTarget { card: CardId, reason: String },

    #[error("{card} requires a target of the wrong kind")]
    TargetKind { card: CardId },

    #[error("only basic lands have a mana ability in this slice")]
    NotAManaSource,
}

impl EngineError {
    /// Short reason text suitable for the UI and for coaching.
    pub fn reason(&self) -> String {
        self.to_string()
    }

    pub fn illegal(action: &'static str, reason: impl Into<String>) -> Self {
        EngineError::Illegal {
            action,
            reason: reason.into(),
        }
    }
}

/// Convenience for validating a sub-submission.
pub fn rejected(action: &Action, reason: impl Into<String>) -> EngineError {
    EngineError::illegal(action.kind(), reason)
}

pub fn color_name(c: ManaColor) -> &'static str {
    c.name()
}
