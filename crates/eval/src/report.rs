//! Raw report shape and disclosed-confidence summary for the paired-seed
//! driver. Every declared seed/rotation is retained: no omissions, no silent
//! retries, no cherry-picking. The summary separates terminal outcomes from
//! capped/failed/replay-mismatch ones and discloses sample sizes and the
//! documented confidence method.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

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
    /// Whole-game wall-clock milliseconds, measured up to (not including)
    /// replay verification.
    pub wall_ms: u64,
    /// Replay verification milliseconds, separate from `wall_ms`.
    pub verify_ms: u64,
    /// True when the engine rejected a policy action; such games are `failed`
    /// and are never scored.
    pub policy_rejected: bool,
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

fn outcome_for(
    configs: (Strategy, Strategy),
    status: &str,
    winner: Option<u8>,
    replay_verified: bool,
) -> OutcomeClass {
    if status != "terminal" || !replay_verified {
        // A terminal game whose replay did not verify is NEVER a scored
        // result: the rule/line identity is unproven, so it can protect
        // neither side of the comparison.
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
    /// Games declared failed because the seat's policy produced an action the
    /// engine rejected (an illegal action is a policy failure, not a pass).
    pub policy_rejected: usize,
    /// Search wins / losses / draws among TERMINAL, replay-VERIFIED games.
    pub terminal_search_wins: usize,
    pub terminal_search_losses: usize,
    pub terminal_draws: usize,
    /// Seat wins split by which strategy held first/served on seat 0, so a
    /// first-player advantage cannot hide inside the aggregate.
    pub wins_by_seat0_strategy: BTreeMap<String, usize>,
    /// False when rotations == 1: the seat swap never happens, so the reported
    /// interval is NOT a paired design and the report says so.
    pub paired_design: bool,
    /// Documented choice: rotations of one seed are correlated units.
    pub paired_units_seeds: usize,
    /// Scored terminal instances (wins+losses+draws of verified games); NOT all
    /// recorded games.
    pub paired_scored_instances: usize,
    /// Declared seeds that contributed no scored game (named, not dropped
    /// silently).
    pub excluded_seeds: Vec<u64>,
    /// Per-seed mean of (search score - 0.5 fair odds), averaged over seeds.
    pub paired_mean_diff_fair_odds: Option<f64>,
    pub paired_ci95_low_diff_fair_odds: Option<f64>,
    pub paired_ci95_high_diff_fair_odds: Option<f64>,
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
    let mut policy_rejected = 0usize;
    let mut search_wins = 0usize;
    let mut search_losses = 0usize;
    let mut draws = 0usize;
    let mut wins_by_seat0: BTreeMap<String, usize> = BTreeMap::new();
    // Per-seed correlated unit: the mean of that seed's rotation DIFFERENCES
    // from fair odds (score - 0.5 per game). Rotation games that are not
    // terminal replay-verified results contribute no score; the scored
    // denominator and excluded seeds are disclosed, never dropped silently.
    let mut diffs_by_seed: BTreeMap<u64, Vec<f64>> = BTreeMap::new();

    for game in games {
        if game.policy_rejected {
            policy_rejected += 1;
        }
        match game.status.as_str() {
            "terminal" => terminal += 1,
            "capped" => capped += 1,
            _ => failed += 1,
        }
        if !game.replay_verified {
            replay_mismatch += 1;
        }
        if game.replay_verified && game.winner == Some(0) {
            *wins_by_seat0
                .entry(format!("{:?}", game.seat0))
                .or_insert(0) += 1;
        }
        let outcome = outcome_for(
            (game.seat0, game.seat1),
            &game.status,
            game.winner,
            game.replay_verified,
        );
        let score = match outcome {
            OutcomeClass::SearchWin => Some(Go::Win.score()),
            OutcomeClass::SearchDraw => Some(Go::Draw.score()),
            OutcomeClass::SearchLoss => Some(Go::Loss.score()),
            OutcomeClass::NotAResult => None,
        };
        if let Some(score) = score {
            diffs_by_seed
                .entry(game.seed)
                .or_default()
                .push(score - FAIR_ODDS);
            if score == Go::Win.score() {
                search_wins += 1;
            } else if score == Go::Draw.score() {
                draws += 1;
            } else {
                search_losses += 1;
            }
        }
    }
    let n_units = diffs_by_seed.len();
    let scored_instances = search_wins + search_losses + draws;
    let excluded_seeds: Vec<u64> = session
        .eval
        .seeds
        .iter()
        .filter(|s| !diffs_by_seed.contains_key(s))
        .copied()
        .collect();
    let units: Vec<f64> = diffs_by_seed
        .values()
        .map(|scores| scores.iter().sum::<f64>() / scores.len() as f64)
        .collect();
    let paired = PairedSample {
        per_seed: units.clone(),
    };
    let (mut ci_note, lo, hi) = if n_units >= 2 && paired_design(session) {
        match paired.t_interval_95() {
            Ok((l, h, note)) => (note, Some(l), Some(h)),
            Err(e) => (e, None, None),
        }
    } else if !paired_design(session) {
        (
            "interval withheld: rotations=1 fixes the seats, so this is NOT a paired design; declare 2 rotations for seat-swap pairing".to_string(),
            None,
            None,
        )
    } else {
        (
            "interval withheld: fewer than 2 correlated units (seeds)".to_string(),
            None,
            None,
        )
    };
    // A run with any replay mismatch has no verified result identity: the CI
    // and headline are withheld outright.
    let (lo, hi) = if replay_mismatch > 0 {
        (None, None)
    } else {
        (lo, hi)
    };

    let scored = search_wins + search_losses + draws;
    let wilsons = if scored > 0 {
        Some(wilson_95(search_wins, scored))
    } else {
        None
    };
    let (wil_note, wlo, whi) = match wilsons {
        Some(Ok((l, h))) => (
            format!(
                "Wilson 95% over {search_wins} wins in {scored} scored terminal games; seeds/rotations are correlated, so this readout does not model that correlation"
            ),
            Some(l),
            Some(h),
        ),
        Some(Err(e)) => (e, None, None),
        None => ("no scored terminal games".to_string(), None, None),
    };

    let headline = if replay_mismatch > 0 {
        "report invalid: at least one replay did not verify against the engine digest; every scored outcome and the confidence interval are WITHHELD".to_string()
    } else if policy_rejected > 0 {
        format!(
            "no strength statement: {policy_rejected} game(s) failed on a rejected policy action and are excluded from all scores"
        )
    } else if scored == 0 {
        "no scored terminal games; no strength claim is possible from this calibration".to_string()
    } else if lo.map(|l| l > 0.0).unwrap_or(false) {
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
    if replay_mismatch > 0 {
        ci_note.push_str("; WITHHELD: replay mismatches present");
    }

    EvalSummary {
        games_declared: session.eval.seeds.len() * session.eval.rotations as usize,
        games_recorded: games.len(),
        terminal,
        capped,
        failed,
        replay_mismatch,
        policy_rejected,
        terminal_search_wins: search_wins,
        terminal_search_losses: search_losses,
        terminal_draws: draws,
        wins_by_seat0_strategy: wins_by_seat0,
        paired_design: paired_design(session),
        paired_units_seeds: n_units,
        paired_scored_instances: scored_instances,
        excluded_seeds: excluded_seeds.clone(),
        paired_mean_diff_fair_odds: paired.mean_opt(),
        paired_ci95_low_diff_fair_odds: lo,
        paired_ci95_high_diff_fair_odds: hi,
        paired_ci_note: format!(
            "{ci_note}; {}",
            paired.denominator_note(scored_instances, &excluded_seeds)
        ),
        wilson_note: wil_note,
        wilson_low: wlo,
        wilson_high: whi,
        headline,
        disclosure: PRACTICE_DISCLOSURE.to_string(),
    }
}

/// Two rotations are required for the seat-swap paired design; one rotation
/// must be flagged as not paired.
fn paired_design(session: &crate::runner::SessionConfig) -> bool {
    session.eval.rotations >= 2
}

pub const FAIR_ODDS: f64 = 0.5;

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
    /// Config SHA-256 over the CANONICAL re-serialization of the parsed config
    /// (`serde_json::to_string`, the shape compared across runs).
    pub config_sha256: String,
    /// SHA-256 of the exact config file bytes that were fed to the CLI.
    pub config_input_sha256: Option<String>,
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
    /// CPU model string from the OS (empty when unobtainable).
    pub cpu_model: String,
    /// `rustc --version` of the compiler that built this driver.
    pub rustc: String,
    /// Git commit of the driver source, if a repo was found.
    pub git_commit: Option<String>,
    /// Profile the games were built with, as executed.
    pub build_profile: String,
}

fn first_stdout_line(cmd: &mut std::process::Command) -> Option<String> {
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().unwrap_or("").trim().to_string();
    if line.is_empty() { None } else { Some(line) }
}

fn cpu_model() -> String {
    if let Some(v) = first_stdout_line(std::process::Command::new("powershell").args([
        "-NoProfile",
        "-Command",
        "(Get-CimInstance Win32_Processor).Name",
    ])) {
        return v;
    }
    first_stdout_line(
        std::process::Command::new("wmic")
            .args(["cpu", "get", "name"])
            .args(["/value"]),
    )
    .and_then(|l| l.strip_prefix("Name=").map(|s| s.trim().to_string()))
    .unwrap_or_default()
}

fn detect_hardware(repo_root: Option<&Path>) -> HardwarePin {
    HardwarePin {
        os: std::env::consts::OS.to_string(),
        logical_cpus: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0),
        cpu_model: cpu_model(),
        rustc: first_stdout_line(std::process::Command::new("rustc").arg("--version"))
            .unwrap_or_else(|| "unknown".to_string()),
        git_commit: repo_root.and_then(|root| {
            first_stdout_line(
                std::process::Command::new("git")
                    .args(["-C"])
                    .arg(root)
                    .arg("rev-parse")
                    .arg("HEAD"),
            )
        }),
        build_profile: if cfg!(debug_assertions) {
            "dev".to_string()
        } else {
            "release".to_string()
        },
    }
}

impl EvalReport {
    pub fn build(
        session: &crate::runner::SessionConfig,
        games: Vec<GameRecord>,
        config_input_sha256: Option<String>,
        repo_root: Option<&Path>,
    ) -> Self {
        let summary = summarize(session, &games);
        let config_json = serde_json::to_string(&session.eval).expect("config serializes");
        EvalReport {
            schema: 2,
            run_id: session.eval.run_id.clone(),
            rules_revision: aperture_core::RULES_REVISION.to_string(),
            config_sha256: sha256_hex(config_json.as_bytes()),
            config_input_sha256,
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
            hardware: detect_hardware(repo_root),
            config: session.eval.clone(),
            games,
            summary,
        }
    }
}

#[cfg(test)]
mod review_regression_tests {
    use super::*;
    use crate::config::{DeckFixture, EvalConfig};

    fn prod_config() -> crate::runner::SessionConfig {
        // A degenerate-but-valid session: only the structure summarize() sees
        // matters (fixture list/paths are never re-read here).
        let eval = EvalConfig {
            run_id: "regression".to_string(),
            decks: vec![
                DeckFixture {
                    name: "A".into(),
                    path: "decks/a.txt".into(),
                },
                DeckFixture {
                    name: "B".into(),
                    path: "decks/b.txt".into(),
                },
            ],
            seeds: vec![101, 102, 103, 104, 105],
            rotations: 2,
            max_steps: 1,
            max_millis: 0,
            search_seeds: 1,
            search_node_budget: 1,
            search_max_rollout_nodes: 1,
            search_max_millis: 0,
            search_max_candidates: 1,
        };
        crate::runner::SessionConfig {
            eval,
            decks: Vec::new(),
            fixture_hashes: Vec::new(),
        }
    }

    fn terminal_record(seed: u64, rotation: u32, winner: Option<u8>, verified: bool) -> GameRecord {
        GameRecord {
            seed,
            rotation,
            seat0: crate::runner::strategy_for(0, rotation),
            seat1: crate::runner::strategy_for(1, rotation),
            search_seat: Some(crate::runner::search_seat(rotation)),
            status: "terminal".to_string(),
            winner,
            reason: "rule".to_string(),
            steps: 10,
            wall_ms: 1,
            verify_ms: 1,
            policy_rejected: false,
            search_ms: 1,
            search_decisions: 1,
            sources: BTreeMap::new(),
            fallback_notes: Vec::new(),
            digest: "d".repeat(64),
            replay_verified: verified,
            replay_error: (!verified).then(|| "fabricated".to_string()),
            truncated: None,
            replay_json: String::new(),
        }
    }

    /// The review's B2 scenario: raw per-game score 0.30 (below fair odds 0.5)
    /// must NOT be called "above parity" under the fair-odds diff statistic.
    /// Rotation 0 searches on seat 0; rotation 1 searches on seat 1.
    #[test]
    fn a_raw_score_of_point_three_is_never_above_parity() {
        let session = prod_config();
        let mut games = Vec::new();
        // 3 wins in 10 rotation games = raw score 0.30 < 0.5 fair odds.
        // rot0: winner 0 = search win, winner 1 = search loss.
        // rot1: winner 1 = search win, winner 0 = search loss.
        let schedule = [
            (101u64, 0u32, Some(0u8)), // search win
            (101, 1, Some(0u8)),       // search loss
            (102, 0, Some(0u8)),       // win
            (102, 1, Some(0u8)),       // loss
            (103, 0, Some(0u8)),       // win
            (103, 1, Some(0u8)),       // loss
            (104, 0, Some(1u8)),       // loss
            (104, 1, Some(0u8)),       // loss
            (105, 0, Some(1u8)),       // loss
            (105, 1, Some(0u8)),       // loss
        ];
        for (seed, rotation, winner) in schedule {
            games.push(terminal_record(seed, rotation, winner, true));
        }
        let summary = summarize(&session, &games);
        assert_eq!(summary.terminal_search_wins, 3);
        assert_eq!(summary.terminal_search_losses, 7);
        assert_eq!(summary.paired_units_seeds, 5);
        let mean = summary.paired_mean_diff_fair_odds.unwrap();
        let lo = summary.paired_ci95_low_diff_fair_odds.unwrap();
        let hi = summary.paired_ci95_high_diff_fair_odds.unwrap();
        assert!(
            mean < 0.0,
            "raw 0.30 playing is a NEGATIVE fair-odds diff, got {mean}"
        );
        assert!(
            lo <= 0.0,
            "lower bound {lo} must not stand above parity for raw score 0.30"
        );
        assert!(hi >= mean);
        assert!(
            !summary.headline.contains("above parity"),
            "a raw 0.30 result must never be released as above parity"
        );
    }

    /// A fabricated terminal record whose replay did not verify is excluded
    /// from ALL scored outcomes and from the seed units (B3). Rotation 1
    /// searches on seat 1, so "winner seat 1" there is a search WIN.
    #[test]
    fn unverified_terminal_replays_never_score() {
        let session = prod_config();
        let games = vec![
            terminal_record(101, 0, Some(0), true), // search win, verified
            terminal_record(101, 1, Some(1), true), // search win, verified
            // Forged search-win records for seed 102 whose replay does NOT
            // verify: they must not lift the win count or seed units.
            terminal_record(102, 0, Some(0), false),
            terminal_record(102, 1, Some(1), false),
        ];
        let summary = summarize(&session, &games);
        assert_eq!(summary.replay_mismatch, 2);
        assert_eq!(summary.terminal_search_wins, 2);
        assert_eq!(summary.terminal_search_losses, 0);
        assert_eq!(summary.terminal_draws, 0);
        assert_eq!(
            summary.paired_units_seeds, 1,
            "unverified seed 102 excluded"
        );
        assert_eq!(summary.excluded_seeds, vec![102, 103, 104, 105]);
        assert_eq!(summary.paired_scored_instances, 2);
        assert!(
            summary.paired_ci95_low_diff_fair_odds.is_none(),
            "CI withheld when mismatches > 0"
        );
        assert!(summary.headline.contains("WITHHELD"));
    }

    /// A policy-rejected game is within `failed` and contributes no score (B5).
    #[test]
    fn policy_rejected_games_are_failed_and_never_scored() {
        let session = prod_config();
        let mut rejected = terminal_record(101, 0, Some(0), true);
        rejected.status = "failed".to_string();
        rejected.winner = None;
        rejected.policy_rejected = true;
        rejected.replay_verified = true; // replay of the truncated line still verifies
        let games = vec![rejected, terminal_record(101, 1, Some(1), true)];
        let summary = summarize(&session, &games);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.policy_rejected, 1);
        assert_eq!(summary.terminal, 1);
        assert_eq!(
            summary.terminal_search_wins, 1,
            "the verified terminal win still scores"
        );
        assert_eq!(summary.paired_units_seeds, 1);
        assert!(summary.headline.contains("no strength statement"));
    }

    /// Single-rotation configs are explicitly marked not-paired and the CI is
    /// withheld (review non-blocking finding on rotations=1).
    #[test]
    fn single_rotation_is_not_a_paired_design() {
        let mut session = prod_config();
        session.eval.rotations = 1;
        let games = vec![terminal_record(101, 0, Some(0), true)];
        let summary = summarize(&session, &games);
        assert!(!summary.paired_design);
        assert!(summary.paired_ci_note.contains("NOT a paired design"));
        assert!(summary.paired_ci95_low_diff_fair_odds.is_none());
    }
}
