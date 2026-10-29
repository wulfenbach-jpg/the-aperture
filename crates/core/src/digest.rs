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

/// A deck-list snapshot's text with line endings normalized to LF.
///
/// A certificate identifies the *list*, not the checkout that produced it: the
/// same tracked `decks/**/*.txt` file is checked out CRLF on Windows and LF
/// elsewhere, so the raw bytes differ across platforms while the list does not.
/// Every side that hashes a snapshot — the `certify` command and the tests that
/// check its record — must go through this one basis, or a certificate recorded
/// on one platform silently fails to verify on another.
///
/// Only CRLF is folded. A lone CR is preserved, because git's `eol` handling
/// never produces one and treating it as a line ending would change the list
/// text rather than the checkout encoding.
pub fn normalize_snapshot_text(bytes: &[u8]) -> Result<String, std::str::Utf8Error> {
    let text = std::str::from_utf8(bytes)?;
    Ok(text.replace("\r\n", "\n"))
}

/// SHA-256 of the snapshot text under the canonical line-ending basis.
pub fn snapshot_digest(bytes: &[u8]) -> Result<String, std::str::Utf8Error> {
    Ok(digest_bytes(normalize_snapshot_text(bytes)?.as_bytes()))
}
