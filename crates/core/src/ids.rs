use serde::{Deserialize, Serialize};

/// Seat index. Two seats are exercised; the state model is a fixed array.
pub type PlayerId = u8;

pub const P0: PlayerId = 0;
pub const P1: PlayerId = 1;
pub const NUM_PLAYERS: usize = 2;

/// The next player in turn order. Correct only for two seats.
pub fn opponent(p: PlayerId) -> PlayerId {
    1 - p
}

/// Game-wide object identity. Allocated monotonically and never reused, so an
/// id stays meaningful across zone changes. (Real Magic creates a new object on
/// every zone change; this slice intentionally keeps identity stable, which is
/// documented in HANDOFF.md as a known limitation.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectId(pub u32);

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
        }
    }
}
