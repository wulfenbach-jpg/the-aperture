//! aperture-eval: reproducible paired-seed policy evaluation driver for
//! THE APERTURE. Real engine games, only offered actions, full replays verified
//! against the engine digest, and separately reported terminal/capped/failed
//! outcomes. Reports on practice fixtures are practice/tooling evidence only.

pub mod config;
pub mod report;
pub mod runner;
pub mod stats;

pub use config::EvalConfig;
pub use report::{EvalReport, EvalSummary, GameRecord, OutcomeClass, Strategy};
pub use runner::{SessionConfig, run_game, run_session, search_seat, strategy_for};

use sha2::{Digest, Sha256};

/// Lowercase hex sha256 of `bytes`, the format used for fixture and config
/// digests in reports.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut hex = String::with_capacity(out.len() * 2);
    for b in out {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
