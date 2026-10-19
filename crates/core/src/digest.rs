//! Canonical state digest. Any two states with equal digests are identical for
//! replay purposes: the digest is SHA-256 over the JSON serialization of the
//! state, which contains no maps with nondeterministic iteration order and no
//! floating point.

use sha2::{Digest, Sha256};

use crate::state::GameState;

pub fn canonical_json(state: &GameState) -> String {
    serde_json::to_string(state).expect("GameState serializes")
}

pub fn digest(state: &GameState) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_json(state).as_bytes());
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

/// Short form for logs and the UI.
pub fn short_digest(state: &GameState) -> String {
    digest(state).chars().take(12).collect()
}

pub fn digest_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}
