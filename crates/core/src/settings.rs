use serde::{Deserialize, Serialize};

use crate::ids::PlayerId;

/// Which policy drives each seat, whether the local decision model is allowed,
/// and which seat is the human's. Multiplayer games may have several AI seats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSettings {
    /// Whether AI stepping is active for the configured AI seats.
    pub enabled: bool,
    /// Continuously play AI turns without a per-step click.
    pub autoplay: bool,
    /// The seat presented as the human's seat in the native UI.
    pub human_seat: PlayerId,
    /// Seats driven by a policy when `enabled`.
    pub ai_seats: Vec<PlayerId>,
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
            human_seat: 0,
            ai_seats: vec![1],
            use_decider: false,
            decider_timeout_ms: 4_000,
        }
    }
}

impl AiSettings {
    /// Reject settings that name a seat outside the game.
    pub fn validate_for(&self, seat_count: usize) -> Result<(), String> {
        let in_range = |p: PlayerId| (p as usize) < seat_count;
        if !in_range(self.human_seat) {
            return Err(format!(
                "human seat P{} is outside the {seat_count}-seat game",
                self.human_seat
            ));
        }
        if let Some(seat) = self.ai_seats.iter().find(|seat| !in_range(**seat)) {
            return Err(format!(
                "AI seat P{seat} is outside the {seat_count}-seat game"
            ));
        }
        let mut seen = Vec::new();
        for seat in &self.ai_seats {
            if seen.contains(seat) {
                return Err(format!("AI seat P{seat} is listed twice"));
            }
            seen.push(*seat);
        }
        Ok(())
    }

    pub fn seat_is_ai(&self, seat: PlayerId) -> bool {
        self.enabled && self.ai_seats.contains(&seat)
    }

    pub fn describe(&self) -> String {
        let policy = if self.use_decider {
            "decider-2b (local, opt-in) with heuristic fallback"
        } else {
            "deterministic baseline heuristic"
        };
        let seats = if self.ai_seats.is_empty() {
            "none".to_string()
        } else {
            self.ai_seats
                .iter()
                .map(|seat| format!("P{seat}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "human P{}; AI seats {seats} = {policy}; autoplay {}",
            self.human_seat,
            if self.autoplay { "on" } else { "off" }
        )
    }
}
