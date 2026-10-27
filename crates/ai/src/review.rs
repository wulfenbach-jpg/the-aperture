//! Ranking layer for move review: bounded search results turned into a ranked,
//! traceable, verdict-free payload.
//!
//! Contract: `docs/design/move-review-spec.md` (spec 1.1). Where this module and
//! that note disagree, the note wins. It implements the *ranking* half of #28:
//! §4 (payload, ordering, uncertainty, lines, reason registry, evidence) and the
//! budget record of §3.4.
//!
//! What this module is, precisely:
//!
//! - **Pure data plus pure derivation.** [`rank`] takes the masked
//!   `PlayerView`, one completed `SearchAnalysis` and the `SearchConfig` that
//!   produced it. It performs no engine call, no I/O, no clock read and no
//!   model call. Every legality statement in the payload is a citation of the
//!   engine's own offer (`LegalAction::rules`); this module encodes no rule.
//! - **Nothing is invented.** The score is the mean practice outcome over
//!   scored rollouts (win `+1`, draw `0`, loss `−1`) — never a probability, and
//!   there is no percentage, odds, equity, centipawn or confidence field
//!   anywhere in the payload (§2.2, D-3, D-4). A line is the recording of a
//!   rollout that reached a result (`search::RecordedLine`) — never a
//!   synthesised variation (§4.4, D-6). A reason is a closed-enum tag with
//!   required evidence — never a free-hand sentence (§4.6, D-2).
//! - **Uncertainty is explicit.** Rows are marked `Ordered`, `EffectivelyTied`
//!   (inseparable at the budget's lattice resolution), `SingleSample` or
//!   `Unscored`; the ordering is only claimed where the counts support it
//!   (§4.3, §4.5, D-8).
//! - **No depth or ply metric.** This search is a rollout ensemble, not a
//!   fixed-depth tree search, so no depth figure exists to report and none is
//!   reported (§4.1 "Depth and per-candidate cost", D-11, AC-17). The real
//!   per-candidate cost is the rollout breakdown; the global cost is
//!   `decisions_used` out of `node_budget`.
//!
//! Documented resolutions of two places where the note is internally
//! inconsistent, both recorded in the task handoff:
//!
//! 1. `RankedReview::preference` is `Option<Preference>`: AC-9 requires
//!    `preference: None` for a refused decision point, which the §4.1 sketch's
//!    bare `Preference` cannot express. Present exactly when an ordering could
//!    be claimed.
//! 2. `spec`, `offer_rules` and rule numbers are `Cow<'static, str>` rather
//!    than `&'static str`. §4.1 also requires `Deserialize`, and a borrow can
//!    never be produced from JSON; `Cow` keeps the static slice where the value
//!    already is one and owns it after a round-trip. The serialised shape is
//!    identical (a JSON string).
//!
//! Two further scoping notes:
//!
//! - `BudgetRecord::rendered` is a pure function of every budget field except
//!   `fidelity`, which §5.2 requires the panel to print *verbatim* on its own
//!   line; `rendered` would otherwise duplicate it.
//! - [`ReasonTag::EngineRuleApplies`] exists in the closed registry but is not
//!   emitted by [`rank`], which receives no `Explanation` set: rule text has
//!   exactly one producer, the rules-section constructor (§5.1, D-9). Its
//!   formatter emits the topic and CR citation; the engine's own sentence is
//!   appended by that constructor, which holds the explanation.

use std::borrow::Cow;

use aperture_core::Action;
use aperture_core::PlayerId;
use aperture_core::coach::ExplanationKind;
use aperture_core::view::PlayerView;
use serde::{Deserialize, Serialize};

use crate::search::{
    CandidateScore, RecordedLine, SearchAnalysis, SearchConfig, SearchOutcome, SearchUnavailable,
    StopReason,
};

/// Payload schema id. Bump with the spec, never silently.
pub const SPEC_ID: &str = "move-review/1.2";

/// One sample cannot establish an order (§4.5).
pub const MIN_ORDERING_SAMPLES: usize = 2;

/// `f32` rounding slack in the lattice comparison (§4.5).
pub const LATTICE_EPSILON: f32 = 1e-6;

/// Measured review budget defaults (§3.1). `max_millis` is the interactive
/// bound; anything recorded uses [`recorded_config`], where it is `u64::MAX`.
pub const REVIEW_SEEDS: usize = 2;
pub const REVIEW_NODE_BUDGET: usize = 30_000;
pub const REVIEW_MAX_ROLLOUT_NODES: usize = 6_000;
pub const REVIEW_MAX_MILLIS: u64 = 8_000;
pub const REVIEW_MAX_CANDIDATES: usize = 6;

/// The measured default review budget (§3.1).
pub fn review_config() -> SearchConfig {
    SearchConfig {
        seeds: REVIEW_SEEDS,
        node_budget: REVIEW_NODE_BUDGET,
        max_rollout_nodes: REVIEW_MAX_ROLLOUT_NODES,
        max_millis: REVIEW_MAX_MILLIS,
        max_candidates: REVIEW_MAX_CANDIDATES,
    }
}

/// The same budget with the wall clock disabled: the only budget a recorded or
/// corpus run may use (§3.3), so only node and round bounds can stop it.
pub fn recorded_config() -> SearchConfig {
    SearchConfig {
        max_millis: u64::MAX,
        ..review_config()
    }
}

/// Which game state a review is about (§4.1, §5.5). A result whose binding no
/// longer matches the live session is stale and must be discarded, not shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewBinding {
    pub seat: PlayerId,
    /// Session generation.
    pub generation: u64,
    /// Engine revision.
    pub revision: u64,
    /// Engine decision serial.
    pub decision: u64,
    /// Masked-projection digest the analysis ran on.
    pub view_digest: String,
    pub rules_revision: String,
}

impl ReviewBinding {
    /// Bind to the seat and masked projection `view` was built for.
    pub fn for_view(view: &PlayerView, generation: u64, revision: u64, decision: u64) -> Self {
        ReviewBinding {
            seat: view.viewer,
            generation,
            revision,
            decision,
            view_digest: view.digest.clone(),
            rules_revision: view.rules_revision.clone(),
        }
    }
}

/// The budget actually used, recorded with every review (§3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetRecord {
    pub seeds: usize,
    pub node_budget: usize,
    pub max_rollout_nodes: usize,
    /// `u64::MAX` serialises as the string `"unbounded"`.
    #[serde(with = "millis_repr")]
    pub max_millis: u64,
    pub max_candidates: usize,
    /// Engine decision points actually consumed.
    pub decisions_used: usize,
    pub stop: StopReason,
    pub cancelled: bool,
    /// `stop ∈ {DeterminizationsExhausted, NodeBudget}` and not cancelled.
    pub reproducible: bool,
    /// The search's own determinization disclosure, verbatim.
    pub fidelity: String,
}

impl BudgetRecord {
    /// The one-line rendering of §3.4, generated from the record and never
    /// hand-written. Pure: every field below changes the line, and the line
    /// never contains a percentage.
    ///
    /// `fidelity` is the documented exception: §5.2 prints it verbatim on its
    /// own line, so it is not folded in here.
    pub fn rendered(&self) -> String {
        let clock = if self.max_millis == u64::MAX {
            "unbounded".to_string()
        } else {
            format!("{} ms", self.max_millis)
        };
        let mut line = format!(
            "budget: {} determinization seed(s) · {} engine decisions total · \
             {} decisions per rollout · {} wall clock · at most {} engine-offered \
             candidates · used {} · stopped: {} · reproducible: {}",
            self.seeds,
            self.node_budget,
            self.max_rollout_nodes,
            clock,
            self.max_candidates,
            self.decisions_used,
            self.stop.label(),
            if self.reproducible { "yes" } else { "no" },
        );
        if self.cancelled {
            line.push_str(" · cancellation observed");
        }
        line
    }
}

/// `u64::MAX` round-trips as the string `"unbounded"` (§3.4).
mod millis_repr {
    use serde::{Deserialize, Deserializer, Serializer};

    pub const UNBOUNDED: &str = "unbounded";

    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        if *value == u64::MAX {
            serializer.serialize_str(UNBOUNDED)
        } else {
            serializer.serialize_u64(*value)
        }
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        Number(u64),
        Text(String),
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        match Repr::deserialize(deserializer)? {
            Repr::Number(value) => Ok(value),
            Repr::Text(text) if text == UNBOUNDED => Ok(u64::MAX),
            Repr::Text(text) => text.parse().map_err(serde::de::Error::custom),
        }
    }
}

/// Which world a line happened in (§4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineScope {
    /// The reconstructed determinization the search played, never the real
    /// game's hidden cards.
    SearchGame,
}

/// One engine-submitted action of a recorded rollout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineStep {
    /// Decision ordinal inside that rollout.
    pub decision: usize,
    pub seat: PlayerId,
    /// The engine's label for the action it accepted.
    pub label: String,
}

/// A candidate's line: a recorded rollout, not a search artefact invented
/// afterwards (§4.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateLine {
    /// Engine-submitted actions, root first.
    pub steps: Vec<LineStep>,
    /// The rollout went further than `search::MAX_LINE_PLIES`; `steps` is a
    /// prefix.
    pub truncated: bool,
    /// Determinization seed index that produced this line.
    pub seed_index: u64,
    /// The recorded rollout's outcome value (`+1`/`0`/`−1`).
    pub outcome: i8,
    pub scope: LineScope,
}

/// How firmly this row's position is claimed (§4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Uncertainty {
    /// Two or more samples and the mean is at a lattice distance from every
    /// other scored row. Ordering is claimed.
    Ordered,
    /// At this budget's resolution this row is not separable from the rows it
    /// is compared with. Ordering is **not** claimed. Symmetric by
    /// construction; values are `offer_index`es.
    EffectivelyTied { with: Vec<usize> },
    /// Fewer than [`MIN_ORDERING_SAMPLES`] scored samples: one sample cannot
    /// establish an order.
    SingleSample,
    /// No rollout reached a result. Not ranked at all.
    Unscored,
}

/// What the search itself preferred, and how weakly (§4.1, §5.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preference {
    /// One scored offer is strictly the highest at this budget's resolution.
    Unique { offer_index: usize },
    /// Two or more offers are level at this budget's resolution. No order
    /// among them is claimed; `chosen` is the one the search submitted.
    AmongEquals {
        chosen: usize,
        offer_indices: Vec<usize>,
    },
    /// No ordering is claimed: the leading row has a single sample. `chosen`
    /// is the search's own selection when it made one.
    NoOrder { chosen: Option<usize> },
}

/// The closed reason registry (§4.6). Adding a variant is a spec change plus a
/// guardrail fixture (with the matching row in §4.6).
///
/// Positional vocabulary — hanging piece, tempo, card advantage, board
/// development, mana efficiency — is deliberately **absent**: no engine trace
/// establishes those, so no tag may claim them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasonTag {
    /// Highest mean practice outcome among the offered alternatives.
    HighestPracticeOutcome,
    /// Level with another offered alternative at this sample count.
    EqualPracticeOutcome,
    /// No scored sample: not ranked.
    NoScoredSample,
    /// Some rollouts ended without a result.
    IncompleteRollouts,
    /// The search's own world refused this root action as illegal.
    RefusedBySearchWorld,
    /// One sample cannot establish an order.
    LowSample,
    /// The analysis stopped at the wall clock or a cancellation.
    BudgetStopped,
    /// The engine offers exactly one action at this decision point.
    SoleLegalOffer,
    /// An engine rule statement backs this row. Not emitted by [`rank`]; see
    /// the module note on D-9.
    EngineRuleApplies {
        rule: Cow<'static, str>,
        topic: String,
    },
}

/// What a sentence is (§4.8). The UI maps this to a section and a badge; the
/// ranking layer never decides presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextKind {
    /// Engine rule or simplification statement, CR-cited.
    Rule,
    /// Search preference / practice-outcome statement.
    Policy,
    /// Sample size, tie, truncation statements.
    Uncertainty,
    /// Budget, binding, fidelity, read-only statement.
    Provenance,
}

/// Where a claim's facts come from. Every reference resolves inside the same
/// payload (§4.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceRef {
    /// An index into the engine's offer (`PlayerView::legal`).
    EngineOffer { index: usize },
    /// The engine's own label for that offered action.
    EngineLabel { index: usize },
    /// The search counts this row reports. They must equal the row's fields.
    SearchCount {
        offer_index: usize,
        scored: usize,
        without_result: usize,
        refused: usize,
    },
    /// An engine explanation, by position, kind and topic.
    CoachExplanation {
        index: usize,
        kind: ExplanationKind,
        topic: String,
    },
    /// A Comprehensive Rules citation the engine itself makes.
    RuleCitation { number: Cow<'static, str> },
    /// A budget field, by name.
    Budget { field: Cow<'static, str> },
    /// A step of a candidate's recorded line.
    LineStep { offer_index: usize, decision: usize },
    /// The masked-projection digest the review is bound to.
    ViewDigest,
}

/// A laballed sentence with the evidence that backs it (§4.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewText {
    pub kind: TextKind,
    pub body: String,
    pub evidence: Vec<EvidenceRef>,
}

/// One ranked (or deliberately unranked) engine-offered alternative (§4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankedCandidate {
    /// Index into the engine's offer order. Together with `move_label` this is
    /// the candidate's identity; it is never a re-derived guess.
    pub offer_index: usize,
    /// The engine's own label for the offered action.
    pub move_label: String,
    /// `LegalAction::rules`, verbatim.
    pub offer_rules: Vec<Cow<'static, str>>,
    /// Mean practice outcome over scored rollouts, rounded to two decimals.
    /// `None` when no rollout reached a result.
    pub score: Option<f32>,
    pub scored_samples: usize,
    /// Rollouts that reached no result.
    pub without_result: usize,
    /// How often the search world refused this root action.
    pub refused_times: usize,
    /// `scored_samples + without_result + refused_times`, derived.
    pub rollouts_attempted: usize,
    pub line: Option<CandidateLine>,
    pub reasons: Vec<ReasonTag>,
    pub uncertainty: Uncertainty,
    /// The search's own selection. A selection, never a verdict.
    pub chosen: bool,
}

/// The ranking payload (§4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankedReview {
    pub spec: Cow<'static, str>,
    pub binding: ReviewBinding,
    pub budget: BudgetRecord,
    /// Scored rows in ranking order, then unscored rows in `offer_index` order.
    /// Empty exactly when `refusal` is `Some`.
    pub candidates: Vec<RankedCandidate>,
    /// Present exactly when an ordering could be claimed.
    pub preference: Option<Preference>,
    /// The search's typed reason, verbatim. A refusal ranks nothing.
    pub refusal: Option<String>,
    /// Provenance and uncertainty sentences, kind-labelled.
    pub notes: Vec<ReviewText>,
}

/// Why an analysis row's counts are what they are, in one place, so a text and
/// its evidence can never disagree.
fn search_count(offer_index: usize, row: &RankedCandidate) -> EvidenceRef {
    EvidenceRef::SearchCount {
        offer_index,
        scored: row.scored_samples,
        without_result: row.without_result,
        refused: row.refused_times,
    }
}

/// Round a mean to the two decimals the payload reports, normalising `-0.0` so
/// two legitimate runs cannot differ in a sign.
fn round_score(mean: f32) -> f32 {
    let rounded = (mean * 100.0).round() / 100.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}

/// Two scored rows are separable iff both have at least
/// [`MIN_ORDERING_SAMPLES`] samples and their means differ by at least one
/// lattice step at the smaller sample count (§4.5).
fn separable(a: f32, a_samples: usize, b: f32, b_samples: usize) -> bool {
    let min_samples = a_samples.min(b_samples);
    if min_samples < MIN_ORDERING_SAMPLES {
        return false;
    }
    (a - b).abs() >= 1.0 / min_samples as f32 - LATTICE_EPSILON
}

/// A pending row: the payload fields plus what the cross-row derivation needs.
struct Pending {
    offer_index: usize,
    move_label: String,
    offer_rules: Vec<Cow<'static, str>>,
    score: Option<f32>,
    scored_samples: usize,
    without_result: usize,
    refused_times: usize,
    line: Option<CandidateLine>,
    chosen: bool,
}

fn pending_from(score: &CandidateScore, offer_index: usize) -> Pending {
    // A line is only ever the recording of a rollout that reached a result.
    let line = if score.scored_samples == 0 {
        None
    } else {
        score.line.as_ref().map(candidate_line)
    };
    let scored = score.mean.map(round_score);
    Pending {
        offer_index,
        move_label: score.label.clone(),
        offer_rules: Vec::new(),
        score: scored,
        scored_samples: score.scored_samples,
        without_result: score.rollouts_without_result,
        refused_times: score.refused_times,
        line,
        chosen: false,
    }
}

fn candidate_line(recorded: &RecordedLine) -> CandidateLine {
    CandidateLine {
        steps: recorded
            .steps
            .iter()
            .map(|step| LineStep {
                decision: step.decision,
                seat: step.seat,
                label: step.label.clone(),
            })
            .collect(),
        truncated: recorded.truncated,
        seed_index: recorded.seed_index,
        outcome: recorded.outcome,
        scope: LineScope::SearchGame,
    }
}

/// The §4.2 order: score descending, `chosen` first, `scored_samples`
/// descending, `offer_index` ascending — then the unscored rows, separate and
/// in `offer_index` order, never interleaved.
fn ranked_order(rows: &[Pending]) -> Vec<usize> {
    let mut scored: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.score.is_some())
        .map(|(index, _)| index)
        .collect();
    scored.sort_by(|left, right| {
        let a = &rows[*left];
        let b = &rows[*right];
        b.score
            .unwrap_or_default()
            .total_cmp(&a.score.unwrap_or_default())
            .then_with(|| b.chosen.cmp(&a.chosen))
            .then_with(|| b.scored_samples.cmp(&a.scored_samples))
            .then_with(|| a.offer_index.cmp(&b.offer_index))
    });
    let mut unscored: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.score.is_none())
        .map(|(index, _)| index)
        .collect();
    unscored.sort_by_key(|index| rows[*index].offer_index);
    scored.extend(unscored);
    scored
}

fn uncertainty_of(rows: &[Pending], index: usize) -> Uncertainty {
    let row = &rows[index];
    let Some(score) = row.score else {
        return Uncertainty::Unscored;
    };
    if row.scored_samples < MIN_ORDERING_SAMPLES {
        return Uncertainty::SingleSample;
    }
    let mut with: Vec<usize> = Vec::new();
    for (other_index, other) in rows.iter().enumerate() {
        if other_index == index || other.scored_samples < MIN_ORDERING_SAMPLES {
            continue;
        }
        let Some(other_score) = other.score else {
            continue;
        };
        if !separable(score, row.scored_samples, other_score, other.scored_samples) {
            with.push(other.offer_index);
        }
    }
    with.sort_unstable();
    with.dedup();
    if with.is_empty() {
        Uncertainty::Ordered
    } else {
        Uncertainty::EffectivelyTied { with }
    }
}

/// Build the review from one completed analysis (§4).
///
/// `view` supplies the engine's offer and its rule citations; `analysis` is the
/// finished search; `config` is the budget that produced it (the payload
/// reports the budget actually used, and the config's bounds beside it).
pub fn rank(
    view: &PlayerView,
    analysis: &SearchAnalysis,
    config: &SearchConfig,
    binding: ReviewBinding,
) -> RankedReview {
    let budget = BudgetRecord {
        seeds: config.seeds,
        node_budget: config.node_budget,
        max_rollout_nodes: config.max_rollout_nodes,
        max_millis: config.max_millis,
        max_candidates: config.max_candidates,
        decisions_used: analysis.decisions_used,
        stop: analysis.stop,
        cancelled: analysis.cancelled,
        reproducible: analysis.reproducible,
        fidelity: analysis.fidelity.clone(),
    };

    // Candidate identity is the offer index plus the engine's own label. An
    // analysis row that is not in this view's offer did not come from this
    // view; that is a typed refusal, never a silent default.
    let mut rows: Vec<Pending> = Vec::with_capacity(analysis.candidates.len());
    for (analysis_index, score) in analysis.candidates.iter().enumerate() {
        let Some(offer_index) = view
            .legal
            .iter()
            .position(|legal| legal.action == score.action)
        else {
            return refused(
                view,
                config,
                binding,
                SearchUnavailable::Inconsistent {
                    detail: format!(
                        "analysis row {analysis_index} (\"{}\") is not in this view's offer",
                        score.label
                    ),
                }
                .label(),
            );
        };
        let mut pending = pending_from(score, offer_index);
        pending.offer_rules = view.legal[offer_index]
            .rules
            .iter()
            .map(|rule| Cow::Borrowed(*rule))
            .collect();
        pending.chosen = analysis.chosen == Some(analysis_index);
        rows.push(pending);
    }

    let order = ranked_order(&rows);
    let sole_offer = deduplicated_offer_len(view) == 1;
    let orderable = |row: &Pending| row.scored_samples >= MIN_ORDERING_SAMPLES;
    // The highest score any offered alternative reached, whatever its sample
    // count: "highest of the offered alternatives" must not be claimed for a row
    // that a single-sampled row beats.
    let top_scored = rows
        .iter()
        .filter_map(|row| row.score)
        .max_by(f32::total_cmp);

    let mut candidates: Vec<RankedCandidate> = Vec::with_capacity(rows.len());
    for index in &order {
        let row = &rows[*index];
        let uncertainty = uncertainty_of(&rows, *index);
        let mut reasons: Vec<ReasonTag> = Vec::new();
        if sole_offer {
            reasons.push(ReasonTag::SoleLegalOffer);
        }
        match (&uncertainty, row.score) {
            (Uncertainty::Ordered, Some(score)) if orderable(row) && Some(score) == top_scored => {
                reasons.push(ReasonTag::HighestPracticeOutcome);
            }
            (Uncertainty::EffectivelyTied { .. }, Some(_)) => {
                reasons.push(ReasonTag::EqualPracticeOutcome);
            }
            (Uncertainty::SingleSample, Some(_)) => reasons.push(ReasonTag::LowSample),
            (Uncertainty::Unscored, _) => reasons.push(ReasonTag::NoScoredSample),
            _ => {}
        }
        if row.without_result > 0 {
            reasons.push(ReasonTag::IncompleteRollouts);
        }
        if row.refused_times > 0 {
            reasons.push(ReasonTag::RefusedBySearchWorld);
        }
        if matches!(budget.stop, StopReason::WallClock | StopReason::Cancelled) {
            reasons.push(ReasonTag::BudgetStopped);
        }
        candidates.push(RankedCandidate {
            offer_index: row.offer_index,
            move_label: row.move_label.clone(),
            offer_rules: row.offer_rules.clone(),
            score: row.score,
            scored_samples: row.scored_samples,
            without_result: row.without_result,
            refused_times: row.refused_times,
            rollouts_attempted: row.scored_samples + row.without_result + row.refused_times,
            line: row.line.clone(),
            reasons,
            uncertainty,
            chosen: row.chosen,
        });
    }

    let preference = preference_of(&candidates);
    let notes = notes_of(&candidates, &budget, &binding, &preference);

    RankedReview {
        spec: Cow::Borrowed(SPEC_ID),
        binding,
        budget,
        candidates,
        preference,
        refusal: None,
        notes,
    }
}

/// Convenience entry point for the whole path: a search result or its typed
/// refusal becomes a review either way.
pub fn rank_outcome(
    view: &PlayerView,
    outcome: &Result<SearchOutcome, SearchUnavailable>,
    config: &SearchConfig,
    binding: ReviewBinding,
) -> RankedReview {
    match outcome {
        Ok(outcome) => rank(view, &outcome.analysis, config, binding),
        Err(unavailable) => refused(view, config, binding, unavailable.label()),
    }
}

/// A review of a decision point the search refused (§5.4, AC-9): no candidates,
/// no ordering, the typed reason verbatim. Rule availability does not depend on
/// search availability, so the offer is still described by `view`.
pub fn refused(
    view: &PlayerView,
    config: &SearchConfig,
    binding: ReviewBinding,
    reason: String,
) -> RankedReview {
    let budget = BudgetRecord {
        seeds: config.seeds,
        node_budget: config.node_budget,
        max_rollout_nodes: config.max_rollout_nodes,
        max_millis: config.max_millis,
        max_candidates: config.max_candidates,
        decisions_used: 0,
        stop: StopReason::NotStarted,
        cancelled: false,
        reproducible: false,
        // A refusal draws no determinization, so there is no fidelity line to
        // quote. Saying so is the honest value; a silent empty string is not.
        fidelity: "not applicable: the search refused this decision point before any \
                   determinization was drawn"
            .to_string(),
    };
    let notes = vec![
        ReviewText {
            kind: TextKind::Provenance,
            body: budget.rendered(),
            evidence: vec![
                EvidenceRef::Budget {
                    field: Cow::Borrowed("stop"),
                },
                EvidenceRef::Budget {
                    field: Cow::Borrowed("max_millis"),
                },
            ],
        },
        ReviewText {
            kind: TextKind::Uncertainty,
            body: format!(
                "no alternative is ranked: the search did not run for this decision point ({reason}). \
                 The engine still offered {} action(s) at it.",
                deduplicated_offer_len(view)
            ),
            evidence: vec![if view.legal.is_empty() {
                EvidenceRef::ViewDigest
            } else {
                EvidenceRef::EngineOffer { index: 0 }
            }],
        },
        ReviewText {
            kind: TextKind::Provenance,
            body: read_only_sentence(),
            evidence: vec![EvidenceRef::ViewDigest],
        },
    ];
    RankedReview {
        spec: Cow::Borrowed(SPEC_ID),
        binding,
        budget,
        candidates: Vec::new(),
        preference: None,
        refusal: Some(reason),
        notes,
    }
}

fn read_only_sentence() -> String {
    "this review is read-only: producing it submits no action, grants no priority and \
     changes no engine state"
        .to_string()
}

/// The engine's offer length after action-equality deduplication — the same
/// candidate set the search takes its roots from.
fn deduplicated_offer_len(view: &PlayerView) -> usize {
    let mut seen: Vec<&Action> = Vec::new();
    for legal in &view.legal {
        if !seen.iter().any(|action| **action == legal.action) {
            seen.push(&legal.action);
        }
    }
    seen.len()
}

/// The search's preference, stated only as firmly as the counts allow (§4.2,
/// §4.3, AC-8).
fn preference_of(candidates: &[RankedCandidate]) -> Option<Preference> {
    let chosen = candidates
        .iter()
        .find(|candidate| candidate.chosen)
        .map(|candidate| candidate.offer_index);
    // The ranking order is the single source of truth here: whatever the first
    // row's uncertainty is, the preference says exactly that much — so the
    // preference line can never contradict the order above it.
    let first = candidates.first()?;
    Some(match &first.uncertainty {
        Uncertainty::Unscored => return None,
        Uncertainty::SingleSample => Preference::NoOrder { chosen },
        Uncertainty::Ordered => Preference::Unique {
            offer_index: first.offer_index,
        },
        Uncertainty::EffectivelyTied { with } => {
            let mut group: Vec<usize> = with.clone();
            group.push(first.offer_index);
            group.sort_unstable();
            group.dedup();
            Preference::AmongEquals {
                chosen: chosen.unwrap_or(first.offer_index),
                offer_indices: group,
            }
        }
    })
}

fn notes_of(
    candidates: &[RankedCandidate],
    budget: &BudgetRecord,
    binding: &ReviewBinding,
    preference: &Option<Preference>,
) -> Vec<ReviewText> {
    let mut notes = vec![
        ReviewText {
            kind: TextKind::Provenance,
            body: budget.rendered(),
            evidence: vec![
                EvidenceRef::Budget {
                    field: Cow::Borrowed("decisions_used"),
                },
                EvidenceRef::Budget {
                    field: Cow::Borrowed("stop"),
                },
            ],
        },
        ReviewText {
            kind: TextKind::Provenance,
            body: format!("determinization fidelity (verbatim): {}", budget.fidelity),
            evidence: vec![EvidenceRef::Budget {
                field: Cow::Borrowed("fidelity"),
            }],
        },
        ReviewText {
            kind: TextKind::Provenance,
            body: format!(
                "bound to seat P{} · generation {} · revision {} · decision {} · rules {} · \
                 masked view digest {}",
                binding.seat,
                binding.generation,
                binding.revision,
                binding.decision,
                binding.rules_revision,
                binding.view_digest
            ),
            evidence: vec![EvidenceRef::ViewDigest],
        },
        ReviewText {
            kind: TextKind::Provenance,
            body: read_only_sentence(),
            evidence: vec![EvidenceRef::ViewDigest],
        },
        ReviewText {
            kind: TextKind::Provenance,
            body: "no depth or plies figure is reported: this analysis is a bounded rollout \
                   ensemble over determinizations, and only the rollout counts and the decision \
                   budget are measurements it actually made"
                .to_string(),
            evidence: vec![EvidenceRef::Budget {
                field: Cow::Borrowed("node_budget"),
            }],
        },
    ];

    let sample_evidence: Vec<EvidenceRef> = candidates
        .iter()
        .map(|candidate| search_count(candidate.offer_index, candidate))
        .collect();
    notes.push(ReviewText {
        kind: TextKind::Uncertainty,
        body: format!(
            "sample counts: {} scored sample(s) over {} engine-offered alternative(s), \
             {} rollout(s) without a result, {} root refusal(s) by the search world",
            candidates.iter().map(|c| c.scored_samples).sum::<usize>(),
            candidates.len(),
            candidates.iter().map(|c| c.without_result).sum::<usize>(),
            candidates.iter().map(|c| c.refused_times).sum::<usize>(),
        ),
        evidence: sample_evidence,
    });

    if let Some(group) = tied_group(candidates) {
        notes.push(ReviewText {
            kind: TextKind::Uncertainty,
            body: format!(
                "effectively tied at this budget's resolution, not ordered against each other: {}",
                group
                    .iter()
                    .map(|candidate| format!(
                        "\"{}\" ({:+.2} over {} scored sample(s))",
                        candidate.move_label,
                        candidate.score.unwrap_or_default(),
                        candidate.scored_samples
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            evidence: group
                .iter()
                .map(|candidate| search_count(candidate.offer_index, candidate))
                .collect(),
        });
    }

    let unscored: Vec<&RankedCandidate> = candidates
        .iter()
        .filter(|candidate| candidate.score.is_none())
        .collect();
    if !unscored.is_empty() {
        notes.push(ReviewText {
            kind: TextKind::Uncertainty,
            body: format!(
                "{} engine-offered alternative(s) reached no result and are not ranked: {}",
                unscored.len(),
                unscored
                    .iter()
                    .map(|candidate| format!("\"{}\"", candidate.move_label))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            evidence: unscored
                .iter()
                .map(|candidate| search_count(candidate.offer_index, candidate))
                .collect(),
        });
    }

    if !budget.reproducible {
        notes.push(ReviewText {
            kind: TextKind::Uncertainty,
            body: format!(
                "the analysis stopped at the {}, so it is not reproducible and must not be \
                 recorded as evidence; the counts above are what the truncated run really did",
                budget.stop.label()
            ),
            evidence: vec![
                EvidenceRef::Budget {
                    field: Cow::Borrowed("reproducible"),
                },
                EvidenceRef::Budget {
                    field: Cow::Borrowed("stop"),
                },
            ],
        });
    }

    if let Some(preference) = preference {
        let target = match preference {
            Preference::Unique { offer_index } => Some(*offer_index),
            Preference::AmongEquals { chosen, .. } => Some(*chosen),
            Preference::NoOrder { chosen } => *chosen,
        };
        let evidence =
            match target.and_then(|index| candidates.iter().find(|c| c.offer_index == index)) {
                Some(candidate) => search_count(candidate.offer_index, candidate),
                None => EvidenceRef::SearchCount {
                    offer_index: 0,
                    scored: 0,
                    without_result: 0,
                    refused: 0,
                },
            };
        let body = match preference {
            Preference::Unique { offer_index } => format!(
                "the search's own choice is offer {offer_index}{}, reported as a practice \
                 outcome and not as a probability of anything",
                label_suffix(candidates, *offer_index)
            ),
            Preference::AmongEquals {
                chosen,
                offer_indices,
            } => format!(
                "the search's own choice among equally scored offers is offer {chosen}{}; \
                 {} offers are level at this sample count and are not ordered",
                label_suffix(candidates, *chosen),
                offer_indices.len()
            ),
            Preference::NoOrder { chosen } => match chosen {
                Some(index) => format!(
                    "no order is claimed: the leading offer has a single scored sample; the \
                     search's own choice is offer {index}{}",
                    label_suffix(candidates, *index)
                ),
                None => "no order is claimed: one scored sample cannot establish one".to_string(),
            },
        };
        notes.push(ReviewText {
            kind: TextKind::Policy,
            body,
            evidence: vec![evidence],
        });
    }

    notes
}

fn label_suffix(candidates: &[RankedCandidate], offer_index: usize) -> String {
    candidates
        .iter()
        .find(|candidate| candidate.offer_index == offer_index)
        .map(|candidate| format!(" (\"{}\")", candidate.move_label))
        .unwrap_or_default()
}

/// The first tie group in ranking order, members in ranking order.
fn tied_group(candidates: &[RankedCandidate]) -> Option<Vec<&RankedCandidate>> {
    for candidate in candidates {
        if let Uncertainty::EffectivelyTied { with } = &candidate.uncertainty {
            let mut group: Vec<&RankedCandidate> = candidates
                .iter()
                .filter(|other| {
                    other.offer_index == candidate.offer_index || with.contains(&other.offer_index)
                })
                .collect();
            group.sort_by_key(|other| {
                candidates
                    .iter()
                    .position(|c| c.offer_index == other.offer_index)
                    .unwrap_or(usize::MAX)
            });
            return Some(group);
        }
    }
    None
}

/// The single reason formatter (§4.6, D-2): tag plus counts plus evidence, never
/// a free-hand sentence. The presentation layer renders its result; nothing else
/// may produce reason text.
pub fn reason_text(
    tag: &ReasonTag,
    candidate: &RankedCandidate,
    budget: &BudgetRecord,
) -> ReviewText {
    let counts = search_count(candidate.offer_index, candidate);
    let score = candidate.score.unwrap_or_default();
    match tag {
        ReasonTag::HighestPracticeOutcome => ReviewText {
            kind: TextKind::Policy,
            body: format!(
                "highest mean practice outcome of the offered alternatives ({score:+.2} over {} \
                 scored sample(s))",
                candidate.scored_samples
            ),
            evidence: vec![
                counts,
                EvidenceRef::EngineOffer {
                    index: candidate.offer_index,
                },
            ],
        },
        ReasonTag::EqualPracticeOutcome => ReviewText {
            kind: TextKind::Uncertainty,
            body: format!(
                "level with another offered alternative at this sample count ({score:+.2})"
            ),
            evidence: vec![counts],
        },
        ReasonTag::NoScoredSample => ReviewText {
            kind: TextKind::Uncertainty,
            body: "no scored sample: no rollout reached a result, so this alternative is not \
                   ranked"
                .to_string(),
            evidence: vec![counts],
        },
        ReasonTag::IncompleteRollouts => ReviewText {
            kind: TextKind::Uncertainty,
            body: format!(
                "{} rollout(s) ended without a result and are counted as neither a win nor a loss",
                candidate.without_result
            ),
            evidence: vec![counts],
        },
        ReasonTag::RefusedBySearchWorld => ReviewText {
            kind: TextKind::Policy,
            body: format!(
                "the search's own reconstruction refused this action {} time(s) as illegal",
                candidate.refused_times
            ),
            evidence: vec![counts],
        },
        ReasonTag::LowSample => ReviewText {
            kind: TextKind::Uncertainty,
            body: "one sample cannot establish an order".to_string(),
            evidence: vec![counts],
        },
        ReasonTag::BudgetStopped => ReviewText {
            kind: TextKind::Uncertainty,
            body: format!(
                "the analysis stopped at the {}, so this sample is incomplete",
                budget.stop.label()
            ),
            evidence: vec![
                counts,
                EvidenceRef::Budget {
                    field: Cow::Borrowed("stop"),
                },
            ],
        },
        ReasonTag::SoleLegalOffer => ReviewText {
            // Policy-side text, not rule text: it cites no comprehensive-rule
            // number, and §5.1 keeps the rules section's single producer for
            // engine explanations (D-9).
            kind: TextKind::Policy,
            body: "the engine offers exactly one action at this decision point".to_string(),
            evidence: vec![EvidenceRef::EngineOffer {
                index: candidate.offer_index,
            }],
        },
        ReasonTag::EngineRuleApplies { rule, topic } => ReviewText {
            kind: TextKind::Rule,
            body: format!("[RULE] {topic} (CR {rule})"),
            evidence: vec![EvidenceRef::RuleCitation {
                number: rule.clone(),
            }],
        },
    }
}
