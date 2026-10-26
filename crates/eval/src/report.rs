//! Raw report shape and disclosed-confidence summary for the paired-seed
//! driver. Every declared seed/rotation is retained: no omissions, no silent
//! retries, no cherry-picking. The summary separates terminal outcomes from
//! capped/failed/replay-mismatch ones and discloses sample sizes and the
//! documented confidence method.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::config::EvalConfig;
use crate::sha256_hex;
use crate::stats::{Go, PairedSample, wilson_95};

/// Which declared strategy played a seat in a given game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// The frozen baseline heuristic. It is never altered to improve a result.
    Heuristic,
    /// Budgeted information-set search; pending/unavailable points fall back to
    /// the same frozen heuristic and that is labelled per seat.
    Search,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameRecord {
    pub seed: u64,
    pub rotation: u32,
    pub seat0: Strategy,
    pub seat1: Strategy,
    /// Always `Some` in this driver (which seat search occupied).
    pub search_seat: Option<u8>,
    /// One of "terminal", "capped", "failed".
    pub status: String,
    /// Terminal wins only; None for capped/failed and draws.
    pub winner: Option<u8>,
    pub reason: String,
    pub steps: usize,
    /// Whole-game wall-clock milliseconds.
    pub wall_ms: u64,
    /// Cumulative milliseconds spent inside search decisions for this game.
    pub search_ms: u64,
    pub search_decisions: u64,
    /// Decision-source counts (search / heuristic fallbacks / forced), auditable.
    pub sources: BTreeMap<String, usize>,
    /// Typed refusal/fallback notes (deduped, capped at five).
    pub fallback_notes: Vec<String>,
    pub digest: String,
    /// True only when the replayed line reproduces the engine's exact final
    /// digest through the core replay validator.
    pub replay_verified: bool,
    pub replay_error: Option<String>,
    /// Set exactly when status != terminal.
    pub truncated: Option<String>,
    /// Full replay payload; also written to its own file on disk.
    pub replay_json: String,
}

/// Search's outcome relative to one opponent game, as an aggregated score. Draw
/// is a half win (disclosed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeClass {
    SearchWin,
    SearchLoss,
    SearchDraw,
    /// Not a result: capped, failed or replay mismatch. Never folded into wins.
    NotAResult,
}

fn outcome_for(configs: (Strategy, Strategy), status: &str, winner: Option<u8>) -> OutcomeClass {
    if status != "terminal" {
        return OutcomeClass::NotAResult;
    }
    match winner {
        None => OutcomeClass::SearchDraw, // terminal draw between the two seats
        Some(w) => {
            if configs.0 as u8 == Strategy::Search as u8 {
                if w == 0 {
                    OutcomeClass::SearchWin
                } else {
                    OutcomeClass::SearchLoss
                }
            } else if configs.1 as u8 == Strategy::Search as u8 {
                if w == 1 {
                    OutcomeClass::SearchWin
                } else {
                    OutcomeClass::SearchLoss
                }
            } else {
                OutcomeClass::NotAResult
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalSummary {
    pub games_declared: usize,
    pub games_recorded: usize,
    pub terminal: usize,
    pub capped: usize,
    pub failed: usize,
    /// Recorded games whose replay did NOT verify against the engine digest.
    pub replay_mismatch: usize,
    /// Search wins / losses / draws among TERMINAL games.
    pub terminal_search_wins: usize,
    pub terminal_search_losses: usize,
    pub terminal_draws: usize,
    /// Seat wins split by which strategy held first/served on seat 0, so a
    /// first-player advantage cannot hide inside the aggregate.
    pub wins_by_seat0_strategy: BTreeMap<String, usize>,
    /// Documented choice: rotations of one seed are correlated units.
    pub paired_units_seeds: usize,
    pub paired_instances: usize,
    pub paired_mean: f64,
    pub paired_ci95_low: f64,
    pub paired_ci95_high: f64,
    pub paired_ci_note: String,
    /// Secondary readout over terminal games only (Wilson). Correlation across
    /// seeds/rotations is disclosed and NOT modeled here.
    pub wilson_note: String,
    pub wilson_low: Option<f64>,
    pub wilson_high: Option<f64>,
    /// Honest one-liner released with the report; never a strength claim
    /// unless the paired interval exceeds parity (lower bound > 0).
    pub headline: String,
    pub disclosure: String,
}

pub fn summarize(session: &crate::runner::SessionConfig, games: &[GameRecord]) -> EvalSummary {
    let mut terminal = 0usize;
    let mut capped = 0usize;
    let mut failed = 0usize;
    let mut replay_mismatch = 0usize;
    let mut search_wins = 0usize;
    let mut search_losses = 0usize;
    let mut draws = 0usize;
    let mut wins_by_seat0: BTreeMap<String, usize> = BTreeMap::new();
    // Per-seed correlated unit: the mean of that seed's rotation results
    // (rotation games that did not reach a terminal outcome contribute no
    // score; the denominator is disclosed with the units).
    let mut scores_by_seed: BTreeMap<u64, Vec<f64>> = BTreeMap::new();

    for game in games {
        match game.status.as_str() {
            "terminal" => terminal += 1,
            "capped" => capped += 1,
            _ => failed += 1,
        }
        if !game.replay_verified {
            replay_mismatch += 1;
        }
        if game.winner == Some(0) {
            *wins_by_seat0
                .entry(format!("{:?}", game.seat0))
                .or_insert(0) += 1;
        }
        let score = match outcome_for((game.seat0, game.seat1), &game.status, game.winner) {
            OutcomeClass::SearchWin => Some(Go::Win.score()),
            OutcomeClass::SearchDraw => Some(Go::Draw.score()),
            OutcomeClass::SearchLoss => Some(Go::Loss.score()),
            OutcomeClass::NotAResult => None,
        };
        if let Some(score) = score {
            scores_by_seed.entry(game.seed).or_default().push(score);
            if score == Go::Win.score() {
                search_wins += 1;
            } else if score == Go::Draw.score() {
                draws += 1;
            } else {
                search_losses += 1;
            }
        }
    }
    let n_units = scores_by_seed.len();
    let units: Vec<f64> = scores_by_seed
        .values()
        .map(|scores| scores.iter().sum::<f64>() / scores.len() as f64)
        .collect();
    let paired = PairedSample { per_seed: units };
    let (ci_note, lo, hi) = if n_units >= 2 {
        match paired.t_interval_95() {
            Ok((l, h, note)) => (note, Some(l), Some(h)),
            Err(e) => (e, None, None),
        }
    } else {
        (
            "interval withheld: fewer than 2 correlated units (seeds)".to_string(),
            None,
            None,
        )
    };

    let instances = games.len();
    let wilsons = if terminal > 0 {
        Some(wilson_95(search_wins, terminal))
    } else {
        None
    };
    let (wil_note, wlo, whi) = match wilsons {
        Some(Ok((l, h))) => (
            format!(
                "Wilson 95% over {search_wins} wins in {terminal} terminal games; seeds/rotations are correlated, so this readout does not model that correlation"
            ),
            Some(l),
            Some(h),
        ),
        Some(Err(e)) => (e, None, None),
        None => ("no terminal games".to_string(), None, None),
    };

    let above_parity = lo.map(|l| l > 0.0).unwrap_or(false);
    let headline = if terminal == 0 {
        "no terminal games; no strength claim is possible from this calibration".to_string()
    } else if above_parity {
        format!(
            "small practice calibration: paired interval lower bound above parity; NOT a certified all-matchup strength claim ({n} calibration units only)",
            n = n_units
        )
    } else {
        format!(
            "practice calibration: search did not exceed parity within disclosed uncertainty ({n} calibration units); no strength claim",
            n = n_units
        )
    };

    EvalSummary {
        games_declared: session.eval.seeds.len() * session.eval.rotations as usize,
        games_recorded: games.len(),
        terminal,
        capped,
        failed,
        replay_mismatch,
        terminal_search_wins: search_wins,
        terminal_search_losses: search_losses,
        terminal_draws: draws,
        wins_by_seat0_strategy: wins_by_seat0,
        paired_units_seeds: n_units,
        paired_instances: instances,
        paired_mean: paired.mean(),
        paired_ci95_low: lo.unwrap_or(f64::NAN),
        paired_ci95_high: hi.unwrap_or(f64::NAN),
        paired_ci_note: format!("{ci_note}; {}", paired.denominator_note(instances)),
        wilson_note: wil_note,
        wilson_low: wlo,
        wilson_high: whi,
        headline,
        disclosure: PRACTICE_DISCLOSURE.to_string(),
    }
}

pub const PRACTICE_DISCLOSURE: &str = "PRACTICE CALIBRATION ONLY: fixtures are declared practice decks, not the eight \
certified #17 starters. This report proves the driver/replay/report pipeline and must \
not be reported as 1000 certified games; issue #26 full acceptance still requires the \
certified corpus run with hardware/budgets pinned.";

/// The full durable artifact written to disk per run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalReport {
    pub schema: u32,
    pub run_id: String,
    /// Rules revision the engine itself reported.
    pub rules_revision: String,
    /// Config SHA-256 (the exact serialized config, versioned next to the report).
    pub config_sha256: String,
    /// Declared fixture path -> sha256 of the deck file used.
    pub fixture_hashes: Vec<(String, String)>,
    /// Kernel/policy provenance.
    pub engine: String,
    pub policies: String,
    /// Freeze note for the heuristic.
    pub frozen_policy_note: String,
    /// Pinned hardware/software for this run (declared, not guessed).
    pub hardware: HardwarePin,
    pub config: EvalConfig,
    pub games: Vec<GameRecord>,
    pub summary: EvalSummary,
}

/// Pinned hardware/software provenance recorded at report build time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwarePin {
    pub os: String,
    pub logical_cpus: usize,
    /// Profile the games were built with, as executed.
    pub build_profile: String,
}

fn detect_hardware() -> HardwarePin {
    HardwarePin {
        os: std::env::consts::OS.to_string(),
        logical_cpus: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0),
        build_profile: if cfg!(debug_assertions) {
            "dev".to_string()
        } else {
            "release".to_string()
        },
    }
}

impl EvalReport {
    pub fn build(session: &crate::runner::SessionConfig, games: Vec<GameRecord>) -> Self {
        let summary = summarize(session, &games);
        let config_json = serde_json::to_string(&session.eval).expect("config serializes");
        EvalReport {
            schema: 1,
            run_id: session.eval.run_id.clone(),
            rules_revision: aperture_core::RULES_REVISION.to_string(),
            config_sha256: sha256_hex(config_json.as_bytes()),
            fixture_hashes: session
                .eval
                .decks
                .iter()
                .cloned()
                .map(|f| f.path)
                .zip(session.fixture_hashes.clone())
                .collect(),
            engine: format!("aperture-core {}", env!("CARGO_PKG_VERSION")),
            policies: "frozen HeuristicPolicy (unchanged) vs budgeted information-set search (unchanged algorithm; budgets pinned in config)".to_string(),
            frozen_policy_note: "The baseline heuristic was not altered for this run; search-budget only differences are pinned in the versioned config.".to_string(),
            hardware: detect_hardware(),
            config: session.eval.clone(),
            games,
            summary,
        }
    }
}
