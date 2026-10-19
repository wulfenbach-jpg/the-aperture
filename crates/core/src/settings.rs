use serde::{Deserialize, Serialize};

use crate::ids::PlayerId;

/// Which policy drives a seat, and whether the local decision model is allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSettings {
    /// Whether AI stepping is active for the AI seat.
    pub enabled: bool,
    /// Continuously play AI turns without a per-step click.
    pub autoplay: bool,
    /// Seat driven by the policy when `enabled`.
    pub ai_seat: PlayerId,
    /// Opt-in: ask the local Decider service (`http://127.0.0.1:8765`).
    pub use_decider: bool,
    /// Timeout for one Decider request, milliseconds.
    pub decider_timeout_ms: u64,
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            enabled: true,
            autoplay: false,
            ai_seat: 1,
            use_decider: false,
            decider_timeout_ms: 4_000,
        }
    }
}

impl AiSettings {
    pub fn describe(&self) -> String {
        let policy = if self.use_decider {
            "decider-2b (local, opt-in) with heuristic fallback"
        } else {
            "deterministic baseline heuristic"
        };
        format!(
            "P{} = {policy}; autoplay {}",
            self.ai_seat,
            if self.autoplay { "on" } else { "off" }
        )
    }
}
