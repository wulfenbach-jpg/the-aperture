//! Move review payload — the closed data contract for issue #28.
//!
//! Normative source: `docs/design/move-review-spec.md` (spec 1.0, §3.4 and §4). This module holds
//! **data only**: no ranking derivation, no presentation, no engine access. The ranking layer
//! (`review::rank`, owned by the ranking task) fills these types; the UI renders them; the
//! honesty guardrails in [`honesty`] audit them.
//!
//! Why the payload is data first:
//!
//! - Reason text is never typed free-hand. A candidate carries a closed [`ReasonTag`]; the
//!   sentence is produced by the single formatter in [`honesty::format_reason`] from
//!   `(tag, counts, evidence)`. A payload that stores prose instead of a tag cannot be audited,
//!   so prose is not part of the contract (D-2).
//! - Every sentence a human reads carries at least one [`EvidenceRef`] that resolves inside the
//!   same payload (AC-4). "Resolves" is defined in [`honesty::audit_review_against`].
//! - Nothing in this payload is a probability. The score is a mean practice outcome over scored
//!   determinizations (`+1` win / `0` draw / `-1` loss), printed with its sample count or not
//!   printed at all (D-3, AC-7).
//!
//! Deviations from the spec text, recorded here so a reviewer does not have to hunt for them:
//!
//! - `RankedReview::preference` is `Option<Preference>`. §4.1 writes the field as `Preference`
//!   but never defines the type, while AC-9 requires `preference: None` for a refusal, which is
//!   only expressible as an `Option`. [`Preference`] therefore carries exactly the two states
//!   §5.3 needs (a unique maximum, or a real tie) and `None` means "no preference is claimed".
//! - Rule citations (CR numbers, budget fields) are stored as `String` rather than the spec's
//!   `&'static str`. §4.1 requires every payload type to implement `Deserialize`, and a borrowed
//!   `'static` field cannot be deserialised without leaking the fixture's strings. The property
//!   the audit actually needs — that a citation is the engine's own rule number, not assembled
//!   prose — is enforced instead by resolving it against `coach::rule_note` and the source
//!   `LegalAction::rules`/`Explanation::rules`, which is checked in
//!   [`honesty::audit_review_against`]. Nothing in the payload is a percentage or a probability
//!   either way.
//! - The payload does not carry the reviewed offer itself. Evidence that quotes engine
//!   explanations is resolved by [`honesty::audit_review_against`] from the caller's
//!   `LegalAction`/`Explanation` slices, which keeps the ranking payload free of a second copy of
//!   the offer.

use aperture_core::coach::ExplanationKind;
use aperture_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

pub mod honesty;

/// Schema id carried by every review payload (§4.1).
pub const SPEC_ID: &str = "move-review/1.0";

/// Recorded rollout steps kept per candidate line (§4.4).
pub const MAX_LINE_PLIES: usize = 12;

/// Below this many scored samples a row is not ordered against any other row (§4.5).
pub const MIN_ORDERING_SAMPLES: usize = 2;

/// Serialises `u64::MAX` as the string `"unbounded"` (§3.4).
mod unbounded_u64 {
    use serde::de::{Error, Unexpected};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &u64, ser: S) -> Result<S::Ok, S::Error> {
        if *value == u64::MAX {
            ser.serialize_str("unbounded")
        } else {
            ser.serialize_u64(*value)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<u64, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Bound(u64),
            Word(String),
        }
        match Wire::deserialize(de)? {
            Wire::Bound(value) => Ok(value),
            Wire::Word(word) if word == "unbounded" => Ok(u64::MAX),
            Wire::Word(word) => Err(D::Error::invalid_value(
                Unexpected::Str(&word),
                &"\"unbounded\" or an integer millisecond bound",
            )),
        }
    }
}

/// Why the analysis stopped (§3.3). Recorded, never collapsed into one boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Every determinization round ran for every candidate.
    DeterminizationsExhausted,
    /// The node budget was reached.
    NodeBudget,
    /// The wall clock was reached.
    WallClock,
    /// A `SearchCancellation` was observed.
    Cancelled,
    /// The decision point was refused before any rollout ran.
    NotStarted,
}

impl StopReason {
    /// Plain-words label for the budget line. No judgement, no percentage.
    pub fn label(self) -> &'static str {
        match self {
            StopReason::DeterminizationsExhausted => "determinizations exhausted",
            StopReason::NodeBudget => "node budget",
            StopReason::WallClock => "wall clock",
            StopReason::Cancelled => "cancelled",
            StopReason::NotStarted => "not started (decision point refused)",
        }
    }

    /// §3.3: reproducible only when a node/round bound stopped the work and nothing cancelled it.
    pub fn is_reproducible(self, cancelled: bool) -> bool {
        !cancelled
            && matches!(
                self,
                StopReason::DeterminizationsExhausted | StopReason::NodeBudget
            )
    }
}

/// The budget this review was produced under (§3.4). Serialised with the payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetRecord {
    pub seeds: usize,
    pub node_budget: usize,
    pub max_rollout_nodes: usize,
    /// `u64::MAX` is serialised as the string `"unbounded"`.
    #[serde(with = "unbounded_u64")]
    pub max_millis: u64,
    pub max_candidates: usize,
    /// Engine decision points actually consumed.
    pub decisions_used: usize,
    pub stop: StopReason,
    pub cancelled: bool,
    pub reproducible: bool,
    /// The search's own determinization disclosure, verbatim.
    pub fidelity: String,
}

impl BudgetRecord {
    /// True when the stored `reproducible` flag agrees with §3.3 for this record.
    ///
    /// A payload may not claim reproducibility the stop reason contradicts; the honesty audit
    /// checks this through [`honesty::audit_review`].
    pub fn reproducible_flag_is_consistent(&self) -> bool {
        self.reproducible == self.stop.is_reproducible(self.cancelled)
    }
}

/// The five-field identity a review is bound to (§5.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewBinding {
    pub seat: PlayerId,
    pub generation: u64,
    pub revision: u64,
    pub decision: u64,
    /// Masked-projection digest the analysis actually ran on.
    pub view_digest: String,
    pub rules_revision: String,
}

/// What the search preferred, if anything (§5.3, AC-9).
///
/// Deliberately not a ranking position and not a verdict: it says which offered action the
/// bounded search selected, and whether that selection was unique or one of a real tie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preference {
    /// Exactly one candidate is the highest-scoring row.
    Unique { offer_index: usize },
    /// Two or more candidates are `EffectivelyTied` at the top; none is ordered above the rest.
    Tied { offer_indexes: Vec<usize> },
}

impl Preference {
    /// Every offer index this preference names.
    pub fn offer_indexes(&self) -> Vec<usize> {
        match self {
            Preference::Unique { offer_index } => vec![*offer_index],
            Preference::Tied { offer_indexes } => offer_indexes.clone(),
        }
    }
}

/// Uncertainty as a statement about samples, not a fitted model (§4.3, §4.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Uncertainty {
    /// At least two samples and one lattice step clear of every other scored row.
    Ordered,
    /// Not separable from these rows at this sample count. Ordering is **not** claimed.
    EffectivelyTied { with: Vec<usize> },
    /// Fewer than [`MIN_ORDERING_SAMPLES`] scored samples.
    SingleSample,
    /// No rollout reached a result; the row is not ranked at all.
    Unscored,
}

/// A line happened in the reconstructed search world, never in the real game (§4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineScope {
    SearchGame,
}

/// One engine-submitted action inside a recorded rollout (§4.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineStep {
    /// Decision ordinal inside that rollout.
    pub decision: usize,
    pub seat: PlayerId,
    /// The label of the action the engine accepted.
    pub label: String,
}

/// A recorded rollout, never a search artefact invented afterwards (§4.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateLine {
    /// Engine-submitted actions from the recorded rollout, root first.
    pub steps: Vec<LineStep>,
    /// The rollout went further than [`MAX_LINE_PLIES`]; the line is a prefix.
    pub truncated: bool,
    /// Determinization seed index that produced this line.
    pub seed_index: u64,
    /// The recorded rollout's outcome value (`+1`/`0`/`-1`).
    pub outcome: i8,
    pub scope: LineScope,
}

/// A closed reason tag (§4.6). Text is derived from the tag, never stored as prose.
///
/// Adding a variant is a spec change: see the module header of [`honesty`] for the required
/// commit shape (registry row, formatter arm, evidence requirement, positive and negative
/// fixture). A variant that is added without those pieces fails the guardrail suite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonTag {
    HighestPracticeOutcome,
    EqualPracticeOutcome,
    NoScoredSample,
    IncompleteRollouts,
    RefusedBySearchWorld,
    LowSample,
    BudgetStopped,
    SoleLegalOffer,
    EngineRuleApplies {
        /// Comprehensive Rules number, verbatim from `Explanation::rules`.
        rule: String,
        /// The engine explanation's own topic. Positional vocabulary is not permitted.
        topic: String,
    },
}

/// A reference into the payload itself (§4.7). Every printed sentence carries at least one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRef {
    /// The engine offered an action at this offer index.
    EngineOffer { index: usize },
    /// The engine's own label for the offered action at this offer index.
    EngineLabel { index: usize },
    /// The sample counts this sentence is derived from.
    SearchCount {
        offer_index: usize,
        scored: usize,
        without_result: usize,
        refused: usize,
    },
    /// An engine rule/trace explanation, quoted by kind and topic.
    CoachExplanation {
        index: usize,
        kind: ExplanationKind,
        topic: String,
    },
    /// A pinned Comprehensive Rules citation.
    RuleCitation { number: String },
    /// A field of the recorded budget.
    Budget { field: String },
    /// A step inside a candidate's recorded line.
    LineStep { offer_index: usize, decision: usize },
    /// The masked view digest the analysis ran on.
    ViewDigest,
}

/// Which section a sentence belongs to (§4.8). The mapping is total and one-to-one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextKind {
    /// Engine rule or simplification statement, CR-cited.
    Rule,
    /// Search preference / practice outcome statement.
    Policy,
    /// Sample size, tie, truncation statement.
    Uncertainty,
    /// Budget, binding, fidelity, read-only statement.
    Provenance,
}

/// The two presentation sections (§5.1). Rule text and policy text never share one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Rules,
    Policy,
}

impl TextKind {
    /// Total, one-to-one mapping from text kind to section (§4.8).
    pub fn section(self) -> Section {
        match self {
            TextKind::Rule => Section::Rules,
            TextKind::Policy | TextKind::Uncertainty | TextKind::Provenance => Section::Policy,
        }
    }
}

/// One labelled sentence with the evidence it rests on (§4.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewText {
    pub kind: TextKind,
    pub body: String,
    pub evidence: Vec<EvidenceRef>,
}

/// One engine-offered alternative, ranked or explicitly unranked (§4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankedCandidate {
    /// Index into the engine's offer order. Candidate identity is this index plus `move_label`.
    pub offer_index: usize,
    /// The engine's own label for the offered action.
    pub move_label: String,
    /// `LegalAction::rules`, verbatim.
    pub offer_rules: Vec<String>,
    /// Mean practice outcome, rounded to two decimals. Never a probability.
    pub score: Option<f32>,
    pub scored_samples: usize,
    /// Rollouts that reached no result: neither a win nor a loss.
    pub without_result: usize,
    /// Search worlds that refused this root action.
    pub refused_times: usize,
    pub line: Option<CandidateLine>,
    pub reasons: Vec<ReasonTag>,
    pub uncertainty: Uncertainty,
    /// The search's own selection. Not a verdict.
    pub chosen: bool,
}

/// The whole review (§4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankedReview {
    /// Must equal [`SPEC_ID`]; a payload from another schema is not auditable.
    pub spec: String,
    pub binding: ReviewBinding,
    pub budget: BudgetRecord,
    /// Empty exactly when `refusal` is `Some`.
    pub candidates: Vec<RankedCandidate>,
    /// `None` exactly when `refusal` is `Some` (AC-9).
    pub preference: Option<Preference>,
    /// The search's typed reason, verbatim.
    pub refusal: Option<String>,
    /// Provenance and uncertainty sentences, kind-labelled.
    pub notes: Vec<ReviewText>,
}

impl RankedReview {
    /// Refused reviews rank nothing and claim no preference (§5.4, AC-9).
    pub fn is_refusal(&self) -> bool {
        self.refusal.is_some()
    }

    /// The candidate at an engine offer index, if the payload has one.
    pub fn candidate(&self, offer_index: usize) -> Option<&RankedCandidate> {
        self.candidates
            .iter()
            .find(|c| c.offer_index == offer_index)
    }
}
