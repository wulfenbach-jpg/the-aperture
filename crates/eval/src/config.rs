//! Versioned evaluation configuration for the paired-seed driver.
//!
//! Only existing public practice fixtures are supported here. The certified
//! corpus (#17) is a separate, later gate that this crate deliberately does not
//! touch.

use serde::{Deserialize, Serialize};

/// One practice deck fixture. `name` is the deck list's displayed name; `path`
/// is relative to the repository root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckFixture {
    pub name: String,
    pub path: String,
}

/// Budgets and limits, mirrored from the policies' own config shapes so a
/// report can pin them exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalConfig {
    /// Free-form, human-readable run identity. Must state its nature plainly,
    /// e.g. "practice-calibration-v1".
    pub run_id: String,
    /// Exactly two practice decks; seat 0 uses the first, seat 1 the second.
    pub decks: Vec<DeckFixture>,
    /// Declared seed list. Every seed must still appear in the raw report, even
    /// if its games were capped or failed: no silent retries or omissions.
    pub seeds: Vec<u64>,
    /// Rotations per seed. 1 = fixed seats, 2 = seats swapped for the second
    /// game (the paired design used for seat-bias disclosure).
    pub rotations: u32,
    /// Step cap for each game; a game that hits it is Capped, never a result.
    pub max_steps: usize,
    /// Whole-game wall-clock bound in milliseconds (0 = no wall-clock bill).
    pub max_millis: u64,
    /// Information-set search budget per decision (mirrors
    /// `aperture_ai::SearchConfig`, pinned per run).
    pub search_seeds: usize,
    pub search_node_budget: usize,
    pub search_max_rollout_nodes: usize,
    pub search_max_millis: u64,
    pub search_max_candidates: usize,
}

impl EvalConfig {
    /// Structural sanity check with actionable errors.
    pub fn validate(&self) -> Result<(), String> {
        if self.decks.len() != 2 {
            return Err(format!(
                "exactly two practice decks are required, got {}",
                self.decks.len()
            ));
        }
        if self.seeds.is_empty() {
            return Err("at least one seed is required".to_string());
        }
        if self.rotations == 0 || self.rotations > 2 {
            return Err(format!(
                "rotations must be 1 or 2 (paired seats), got {}",
                self.rotations
            ));
        }
        if self.max_steps == 0 {
            return Err("max_steps must be positive".to_string());
        }
        let no_white = |v: usize, what: &str| {
            if v == 0 {
                Err(format!("{what} must be positive"))
            } else {
                Ok(())
            }
        };
        no_white(self.search_seeds, "search_seeds")?;
        no_white(self.search_node_budget, "search_node_budget")?;
        no_white(self.search_max_rollout_nodes, "search_max_rollout_nodes")?;
        if self.search_max_millis == 0 {
            return Err("search_max_millis must be positive".to_string());
        }
        no_white(self.search_max_candidates, "search_max_candidates")?;
        Ok(())
    }
}
