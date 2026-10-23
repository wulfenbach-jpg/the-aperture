use serde::{Deserialize, Serialize};

/// Seat index. Seats are ordered and fixed for the whole game; a departed seat
/// keeps its index and is never renumbered (CR 800.4a, 800.4k).
pub type PlayerId = u8;

pub const P0: PlayerId = 0;
pub const P1: PlayerId = 1;

/// Supported seat range for the multiplayer practice kernel.
pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 4;

/// Game-wide object identity. Allocated monotonically and never reused, so an
/// id stays meaningful across zone changes. (Real Magic creates a new object on
/// every zone change; this slice intentionally keeps identity stable, which is
/// documented in HANDOFF.md as a known limitation.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectId(pub u32);

/// Stable identity of one physical commander card. This is an attribute of the
/// card, not a characteristic of the current zone object (CR 903.3), so it
/// survives every zone transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CommanderId(pub u32);

impl std::fmt::Display for CommanderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "commander#{}", self.0)
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Stack,
    Graveyard,
    Exile,
    /// The command zone (CR 903.6, 903.8). Public information.
    Command,
    /// Internal representation of a card that left the game with its owner
    /// (CR 800.4a). This is not a game zone: no view, legal-action generator or
    /// effect may read it. The 800.4a "still controlled objects are exiled"
    /// clause is unreachable here because this slice has no control-changing
    /// effects (controller == owner).
    Removed,
}

impl Zone {
    pub fn name(self) -> &'static str {
        match self {
            Zone::Library => "library",
            Zone::Hand => "hand",
            Zone::Battlefield => "battlefield",
            Zone::Stack => "stack",
            Zone::Graveyard => "graveyard",
            Zone::Exile => "exile",
            Zone::Command => "command zone",
            Zone::Removed => "removed from the game",
        }
    }
}
