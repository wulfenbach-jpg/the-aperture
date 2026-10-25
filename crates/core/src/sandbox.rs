//! Tabletop sandbox controls.
//!
//! These are the manual table controls a player would have with real cardboard
//! in front of them: draw a card, untap, mill, look at the top of a library,
//! reveal, edit a life total, edit counters. They are deliberately NOT
//! legal-match actions: they bypass the turn structure and the rules, so using
//! one **taints the session's competitive record** for the rest of that session
//! (see [`SandboxRecord`]). The engine applies each one through a deterministic
//! path with no random draws, emits a typed viewer-masked trace event, and
//! bumps the revision/decision serial so any outstanding submission is stale.
//!
//! Rules revision: Comprehensive Rules effective 2026-09-25. The scry/surveil
//! controls model the *choice* a player makes after looking (keep on top, put on
//! the bottom, or put into the graveyard) so a single click is deterministic;
//! the real turn-based scry flow remains the pending-choice path in the engine.

use serde::{Deserialize, Serialize};

use crate::ids::{ObjectId, PlayerId};

/// One manual table control. Every variant is an override, never a legal action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxOp {
    /// Draw `count` cards for `seat` from the top of its library.
    Draw { seat: PlayerId, count: u8 },
    /// Untap every permanent `seat` controls.
    Untap { seat: PlayerId },
    /// Put the top `count` cards of `seat`'s library into its graveyard.
    Mill { seat: PlayerId, count: u8 },
    /// Look at the top `count` cards and keep them on top (`to_top`) or put
    /// them on the bottom of the library in the same relative order.
    Scry {
        seat: PlayerId,
        count: u8,
        to_top: bool,
    },
    /// Look at the top `count` cards and keep them on top (`to_top`) or put
    /// them into the graveyard.
    Surveil {
        seat: PlayerId,
        count: u8,
        to_top: bool,
    },
    /// Reveal the top `count` cards of `seat`'s library to every seat. This is
    /// public information by construction, so its names are not private.
    Reveal { seat: PlayerId, count: u8 },
    /// Set the +1/+1 counter count on a permanent on the battlefield.
    SetCounters { object: ObjectId, count: u32 },
    /// Add (or remove, with a negative delta) +1/+1 counters on a permanent.
    AddCounters { object: ObjectId, delta: i32 },
    /// Set a seat's life total. Never ends the game: 0 or less is refused so a
    /// sandbox edit cannot masquerade as a match result.
    SetLife { seat: PlayerId, life: i32 },
}

impl SandboxOp {
    pub fn kind(self) -> SandboxKind {
        match self {
            SandboxOp::Draw { .. } => SandboxKind::Draw,
            SandboxOp::Untap { .. } => SandboxKind::Untap,
            SandboxOp::Mill { .. } => SandboxKind::Mill,
            SandboxOp::Scry { .. } => SandboxKind::Scry,
            SandboxOp::Surveil { .. } => SandboxKind::Surveil,
            SandboxOp::Reveal { .. } => SandboxKind::Reveal,
            SandboxOp::SetCounters { .. } | SandboxOp::AddCounters { .. } => SandboxKind::Counters,
            SandboxOp::SetLife { .. } => SandboxKind::Life,
        }
    }

    /// The seat this control is recorded against. Counter edits are recorded
    /// against the permanent's controller, so the caller supplies that.
    pub fn seat(self, controller: PlayerId) -> PlayerId {
        match self {
            SandboxOp::Draw { seat, .. }
            | SandboxOp::Untap { seat }
            | SandboxOp::Mill { seat, .. }
            | SandboxOp::Scry { seat, .. }
            | SandboxOp::Surveil { seat, .. }
            | SandboxOp::Reveal { seat, .. }
            | SandboxOp::SetLife { seat, .. } => seat,
            SandboxOp::SetCounters { .. } | SandboxOp::AddCounters { .. } => controller,
        }
    }
}

/// Typed label for a sandbox override. Serialized inside the event trace and
/// the session's taint record so neither stores free-form prose as the only
/// account of what happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SandboxKind {
    Draw,
    Untap,
    Mill,
    Scry,
    Surveil,
    Reveal,
    Counters,
    Life,
}

impl SandboxKind {
    pub fn label(self) -> &'static str {
        match self {
            SandboxKind::Draw => "draw",
            SandboxKind::Untap => "untap",
            SandboxKind::Mill => "mill",
            SandboxKind::Scry => "scry",
            SandboxKind::Surveil => "surveil",
            SandboxKind::Reveal => "reveal",
            SandboxKind::Counters => "counters",
            SandboxKind::Life => "life",
        }
    }
}

/// The result of one applied sandbox control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxOutcome {
    /// Trace events to append to the session log, in order.
    pub events: Vec<crate::event::Event>,
    /// The durability record entry, written into the session save.
    pub edit: SandboxEdit,
}

/// One override, recorded in the session save so the taint survives save/load
/// and can never be presented as a legal match result afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxEdit {
    pub kind: SandboxKind,
    pub seat: PlayerId,
    pub turn: u32,
    pub step: String,
    /// Public description (no hidden card names).
    pub detail: String,
}

/// Ordered taint record. Empty means "no override has been used".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxRecord {
    #[serde(default)]
    pub edits: Vec<SandboxEdit>,
}

impl SandboxRecord {
    pub fn is_tainted(&self) -> bool {
        !self.edits.is_empty()
    }

    pub fn push(&mut self, edit: SandboxEdit) {
        self.edits.push(edit);
    }

    pub fn len(&self) -> usize {
        self.edits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Distinct kinds used, in first-use order.
    pub fn kinds(&self) -> Vec<SandboxKind> {
        let mut kinds: Vec<SandboxKind> = Vec::new();
        for edit in &self.edits {
            if !kinds.contains(&edit.kind) {
                kinds.push(edit.kind);
            }
        }
        kinds
    }

    pub fn summary(&self) -> String {
        if self.edits.is_empty() {
            return "no sandbox overrides".to_string();
        }
        let kinds: Vec<&str> = self.kinds().iter().map(|kind| kind.label()).collect();
        format!(
            "{} sandbox override(s) [{}]",
            self.edits.len(),
            kinds.join(", ")
        )
    }

    /// The sentence every surface that could be mistaken for a result uses.
    pub fn record_line(&self, outcome: Option<&crate::state::Outcome>) -> String {
        let result = match outcome {
            Some(crate::state::Outcome::Win { winner, reason }) => {
                format!("P{winner} won this sandbox game ({reason})")
            }
            Some(crate::state::Outcome::Draw { reason }) => {
                format!("this sandbox game was a draw ({reason})")
            }
            None => "game still in progress".to_string(),
        };
        if self.edits.is_empty() {
            format!("{result} — no sandbox override was used")
        } else {
            format!(
                "SANDBOX-MODIFIED — {result}, but this is NOT a legal match result ({}). The taint lasts for the rest of this session.",
                self.summary()
            )
        }
    }
}
