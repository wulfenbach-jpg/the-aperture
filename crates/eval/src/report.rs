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
    /// One of "terminal", "capped", "failed", "policy_rejected" (a rejected
    /// policy action keeps its own literal status AND counts under `failed`
    /// aggregates; it is never scored).
    pub status: String,
    /// Terminal wins only; None for capped/failed/policy-rejected and draws.
    pub winner: Option<u8>,
    pub reason: String,
    pub steps: usize,
    /// Whole-game wall-clock milliseconds, measured up to (not including)
    /// replay verification. EXCLUDED from the serialized report: wall-clock
    /// timings are the only non-deterministic bytes a run produces, and the
    /// scored report must stay byte-reproducible (repair 3). The measurement
    /// is exported to the sibling `timings.json` sidecar instead
    /// ([`latency_rows`]).
    #[serde(skip)]
    pub wall_ms: u64,
    /// Replay verification milliseconds, separate from `wall_ms`. Excluded
    /// from the serialized report for the same reproducibility reason (see
    /// `wall_ms`).
    #[serde(skip)]
    pub verify_ms: u64,
    /// True when the engine rejected a policy action; such games carry the
    /// literal `policy_rejected` status and are never scored.
    pub policy_rejected: bool,
    /// Cumulative milliseconds spent inside search decisions for this game.
    /// Excluded from the serialized report for the same reproducibility reason
    /// (see `wall_ms`).
    #[serde(skip)]
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
    /// Terminal games whose replay VERIFIED. Terminal records whose replay
    /// did not verify are counted separately in `terminal_unverified`, so a
    /// bare `terminal` count can never be misread as all-scored (repair 3).
    pub terminal: usize,
    /// Terminal games whose replay did NOT verify: never scored, never folded
    /// into `terminal`.
    pub terminal_unverified: usize,
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
    let mut terminal_unverified = 0usize;
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
            // `terminal` counts only replay-VERIFIED terminal games; terminal
            // records whose replay did not verify carry their own label.
            "terminal" if game.replay_verified => terminal += 1,
            "terminal" => terminal_unverified += 1,
            "capped" => capped += 1,
            // A literal policy-rejected status still aggregates under `failed`.
            "policy_rejected" => failed += 1,
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
        // Wording repair (non-blocking review finding): only the interval and
        // the above/below-parity verdict are withheld; verified-only
        // aggregates stay populated for audit. Never overstate.
        "report invalid: at least one replay did not verify against the engine digest; the paired CI and the above/below-parity verdict are WITHHELD, while verified-only aggregates (terminal wins/losses/draws, paired mean diff) remain populated for audit".to_string()
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
        terminal_unverified,
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
    /// Name of the sibling sidecar file (`timings.json`) carrying the raw
    /// per-game wall/verify/search timings, which are deliberately EXCLUDED
    /// from this serialized report so the scored evidence stays
    /// byte-reproducible (repair 3).
    pub latency_sidecar: String,
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
    /// Git commit of the driver source, if a repo was found. This MUST be the
    /// commit whose code actually executed and can reproduce the report;
    /// a stale pin is a false-provenance defect (repair-3 blocking finding).
    pub git_commit: Option<String>,
    /// `git rev-parse HEAD^{tree}` at run time: the exact tracked content
    /// that produced the report.
    pub git_tree: String,
    /// True when the run's tracked working tree was NOT clean (or git could
    /// not prove the provenance at all). Dirty-tree evidence is refused
    /// by default and, with `--allow-dirty`, stamped unusable for any claim.
    pub git_dirty: bool,
    /// Profile the games were built with, as executed.
    pub build_profile: String,
}

/// True when `git status --porcelain` text (captured WITHOUT untracked files)
/// shows any tracked modification. Pure so it can be regression-tested.
pub fn porcelain_status_is_dirty(status: &str) -> bool {
    status.lines().any(|line| !line.trim().is_empty())
}

/// Escape-hatch stamping for evidence produced from a dirty tree, per the
/// repair-3 work order: any escape hatch must stamp the evidence unusable
/// for any claim.
pub fn mark_dirty_evidence(report: &mut EvalReport) {
    let s = &mut report.summary;
    s.headline = "DIRTY TREE - NOT REPRODUCIBLE: this report was produced from a working tree with tracked modifications; NO value in it may support any claim".to_string();
    s.paired_mean_diff_fair_odds = None;
    s.paired_ci95_low_diff_fair_odds = None;
    s.paired_ci95_high_diff_fair_odds = None;
    s.paired_ci_note.push_str(
        "; DIRTY TREE - NOT REPRODUCIBLE: interval suppressed (evidence unusable for any claim)",
    );
    s.wilson_low = None;
    s.wilson_high = None;
    s.wilson_note
        .push_str("; DIRTY TREE - NOT REPRODUCIBLE: evidence unusable for any claim");
}

/// Refuse dirty-tree evidence by default. `git_dirty` is the run-time pin;
/// `allow_dirty` is the explicit `--allow-dirty` escape hatch.
pub fn dirty_tree_gate(git_dirty: bool, allow_dirty: bool) -> Result<(), String> {
    if git_dirty && !allow_dirty {
        Err(
            "refusing to write evidence: the working tree has tracked modifications (or git could not prove clean provenance), so the report could not name the code that produces it. Commit or stash first, or re-run with --allow-dirty to emit a DIRTY TREE-labelled report that is unusable for any claim."
                .to_string(),
        )
    } else {
        Ok(())
    }
}

/// One game's measured latencies, exported to the `timings.json` sidecar
/// instead of `report.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatencyRow {
    pub seed: u64,
    pub rotation: u32,
    /// Whole-game wall-clock milliseconds (excludes replay verification).
    pub wall_ms: u64,
    /// Replay-verification milliseconds, reported separately.
    pub verify_ms: u64,
    /// Cumulative search milliseconds for the game.
    pub search_ms: u64,
}

/// The sidecar payload: self-describing raw latency rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatencySidecar {
    pub note: String,
    pub rows: Vec<LatencyRow>,
}

pub const LATENCY_SIDECAR_FILE: &str = "timings.json";

/// Collect the per-game latency rows that a run must write to
/// [`LATENCY_SIDECAR_FILE`] next to the report.
pub fn latency_rows(games: &[GameRecord]) -> LatencySidecar {
    LatencySidecar {
        note: "Raw per-game latencies, deliberately NOT in report.json so the scored report stays byte-reproducible; wall_ms excludes replay verification.".to_string(),
        rows: games
            .iter()
            .map(|g| LatencyRow {
                seed: g.seed,
                rotation: g.rotation,
                wall_ms: g.wall_ms,
                verify_ms: g.verify_ms,
                search_ms: g.search_ms,
            })
            .collect(),
    }
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

/// Public provenance probe used by the CLI BEFORE any game runs: (commit,
/// tree, tracked-dirty). Commit `None` means unnameable provenance, which the
/// dirty gate refuses by default.
pub fn git_provenance_probe(root: &Path) -> (Option<String>, String, bool) {
    git_pin(root)
}

/// Git provenance probe: (commit, tree, tracked-dirty). Commit `None` (no
/// repo, or git unusable) means provenance is UNNAMEABLE and the caller must
/// treat the tree as dirty: unnameable code cannot be claimed reproducible.
fn git_pin(root: &Path) -> (Option<String>, String, bool) {
    let commit = first_stdout_line(
        std::process::Command::new("git")
            .args(["-C"])
            .arg(root)
            .args(["rev-parse", "HEAD"]),
    );
    match commit {
        Some(commit) => {
            let tree = first_stdout_line(
                std::process::Command::new("git")
                    .args(["-C"])
                    .arg(root)
                    .args(["rev-parse", "HEAD^{tree}"]),
            )
            .unwrap_or_default();
            let dirty = match std::process::Command::new("git")
                .args(["-C"])
                .arg(root)
                .args(["status", "--porcelain", "--untracked-files=no"])
                .output()
            {
                Ok(out) => {
                    if out.status.success() {
                        porcelain_status_is_dirty(&String::from_utf8_lossy(&out.stdout))
                    } else {
                        // Cannot prove clean -> treat as dirty.
                        true
                    }
                }
                Err(_) => true,
            };
            (Some(commit), tree, dirty)
        }
        None => (None, String::new(), true),
    }
}

fn detect_hardware(repo_root: Option<&Path>) -> HardwarePin {
    let (git_commit, git_tree, git_dirty) = match repo_root {
        Some(root) => git_pin(root),
        None => (None, String::new(), true),
    };
    HardwarePin {
        os: std::env::consts::OS.to_string(),
        logical_cpus: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0),
        cpu_model: cpu_model(),
        rustc: first_stdout_line(std::process::Command::new("rustc").arg("--version"))
            .unwrap_or_else(|| "unknown".to_string()),
        git_commit,
        git_tree,
        git_dirty,
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
            // Schema 3: latency sidecar + git tree/dirty pin (repair 3).
            schema: 3,
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
            latency_sidecar: LATENCY_SIDECAR_FILE.to_string(),
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

    /// A literal `policy_rejected` status (repair-3 note on B5) aggregates
    /// under `failed`, is excluded from every score, and cannot count as
    /// `terminal`.
    #[test]
    fn literal_policy_rejected_status_is_failed_and_never_scored() {
        let session = prod_config();
        let mut rejected = terminal_record(101, 0, None, true);
        rejected.status = "policy_rejected".to_string();
        rejected.policy_rejected = true;
        let games = vec![rejected, terminal_record(101, 1, Some(1), true)];
        let summary = summarize(&session, &games);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.policy_rejected, 1);
        assert_eq!(summary.terminal, 1);
        assert_eq!(summary.terminal_unverified, 0);
        assert_eq!(
            summary.terminal_search_wins, 1,
            "the verified terminal win still scores"
        );
        assert!(summary.headline.contains("no strength statement"));
    }

    /// Terminal-but-unverified records are labelled separately from `terminal`
    /// so a bare terminal count can never be read as all-scored (repair 3).
    #[test]
    fn terminal_unverified_is_labelled_separately() {
        let session = prod_config();
        let games = vec![
            terminal_record(101, 0, Some(0), true),
            terminal_record(101, 1, Some(1), false),
        ];
        let summary = summarize(&session, &games);
        assert_eq!(summary.terminal, 1);
        assert_eq!(summary.terminal_unverified, 1);
        assert_eq!(summary.replay_mismatch, 1);
        assert_eq!(summary.terminal_search_wins, 1, "only the verified one");
        assert!(summary.headline.contains("WITHHELD"));
        assert!(
            summary.headline.contains("verified-only aggregates"),
            "the headline must state exactly what is retained: {}",
            summary.headline
        );
    }

    /// Tracked-modification porcelain text parses to dirty=True; empty output
    /// to false. This is the parser used by the run-time dirty gate.
    #[test]
    fn porcelain_status_parser_flags_tracked_changes_only() {
        assert!(!porcelain_status_is_dirty(""));
        assert!(!porcelain_status_is_dirty("\n"));
        assert!(!porcelain_status_is_dirty("   \n"));
        assert!(porcelain_status_is_dirty(" M crates/eval/src/report.rs\n"));
        assert!(porcelain_status_is_dirty("M  src/stats.rs\n"));
        assert!(porcelain_status_is_dirty("M  a.rs\n M b.rs\n"));
    }

    /// A dirty tree can never produce an unlabelled scored report: without the
    /// explicit escape hatch the run is refused outright, and with the hatch
    /// the summary is stamped unusable for any claim with every interval and
    /// mean suppressed.
    #[test]
    fn dirty_tree_is_refused_by_default_and_stamped_unusable_with_hatch() {
        assert!(dirty_tree_gate(true, false).is_err());
        assert!(dirty_tree_gate(true, true).is_ok());
        assert!(dirty_tree_gate(false, false).is_ok());

        let session = prod_config();
        let games = vec![terminal_record(101, 0, Some(0), true)];
        let mut report = EvalReport::build(&session, games, None, None);
        assert_ne!(report.summary.headline, "");
        mark_dirty_evidence(&mut report);
        assert!(report.summary.headline.contains("DIRTY TREE"));
        assert!(report.summary.headline.contains("NOT REPRODUCIBLE"));
        assert!(report.summary.paired_mean_diff_fair_odds.is_none());
        assert!(report.summary.paired_ci95_low_diff_fair_odds.is_none());
        assert!(report.summary.paired_ci95_high_diff_fair_odds.is_none());
        assert!(report.summary.wilson_low.is_none());
        assert!(report.summary.wilson_high.is_none());
        assert!(report.summary.paired_ci_note.contains("DIRTY TREE"));
        assert!(
            report
                .summary
                .paired_ci_note
                .contains("unusable for any claim")
        );
    }

    /// The latency sidecar is self-describing and matches the games' own
    /// (non-serialized) timing fields row for row, in declared order.
    #[test]
    fn latency_rows_export_every_game_in_declared_order() {
        let games = vec![
            terminal_record(101, 0, Some(0), true),
            terminal_record(101, 1, Some(1), true),
        ];
        let sidecar = latency_rows(&games);
        assert_eq!(sidecar.rows.len(), games.len());
        assert_eq!(sidecar.rows[0].seed, 101);
        assert_eq!(sidecar.rows[0].rotation, 0);
        assert_eq!(sidecar.rows[0].wall_ms, games[0].wall_ms);
        assert_eq!(sidecar.rows[0].verify_ms, games[0].verify_ms);
        assert_eq!(sidecar.rows[0].search_ms, games[0].search_ms);
        assert_eq!(sidecar.rows[1].rotation, 1);
        // The sidecar filename is stable and documented on the report itself.
        assert_eq!(LATENCY_SIDECAR_FILE, "timings.json");
    }
}
