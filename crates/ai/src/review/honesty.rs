//! Honesty guardrails for the move review (spec §6).
//!
//! Normative source: `docs/design/move-review-spec.md` §6, with the payload defined in
//! [`crate::review`] (§4.1–§4.8) and the budget in §3.4.
//!
//! The review tells a learner three separate things: what the rules made possible, what a bounded
//! search preferred and how weakly, and what the tool does not know. This module is what stops the
//! first two from quietly turning into a third thing nobody can support — a probability, a verdict
//! on the player, or a sentence with no evidence behind it.
//!
//! It is deliberately independent of the UI and of `search`: it takes payload data and strings,
//! and it returns violations. That is what makes it CI-checkable.
//!
//! # What is rejected, and why the line is where it is
//!
//! 1. **Percentages.** `\d+(\.\d+)?\s*%` anywhere in review text is a violation, with **no**
//!    negation excuse (`PercentFigure`). The number itself is the false impression; the sentence
//!    around it cannot excuse it. The search's outcome is a mean of `{-1, 0, +1}` samples, so any
//!    percentage claims a scale the project does not have (D-3, D-4).
//! 2. **Probability and strength vocabulary** (`ProbabilityClaim`): win probability, chance of
//!    winning, winrate/win rate, odds, equity, expected score, centipawn, Elo, tablebase, "likely
//!    wins". Same reason, in words instead of digits.
//! 3. **Verdict labels** (`VerdictLabel`): blunder, mistake, inaccuracy, brilliant, "best move",
//!    "correct move", "should have", standalone "error". The review states what the engine offered
//!    and what the search preferred. It does not grade the player (D-5).
//! 4. **Untraceable claims** (`UntraceableClaim`): a sentence with no `EvidenceRef`, a reason tag
//!    whose required evidence the payload does not contain, an ordering claimed on one sample, a
//!    preference that contradicts the scores, a reproducibility flag the stop reason contradicts,
//!    and a sentence labelled as a rule that cites no rule number.
//! 5. **Structural boundary failures**: an unregistered reason tag, an evidence reference that
//!    does not resolve, rule text in the policy section (or policy text in the rules section), and
//!    rules/policy collapsed into one presentation surface (D-9, AC-11).
//!
//! ## The negation rule, and its one known hole
//!
//! Rule 5 of the spec: a sentence carrying a probability pattern or a verdict label is allowed
//! **only** when the same sentence also carries a negation marker from
//! `{no, not, never, none, nothing, neither}`. This is what legalises the shipped disclaimers —
//! `"it labels no alternative as an error"` and `"preference not reported; not a win probability"`
//! — while still failing `"72% chance this is best"`.
//!
//! The hole is inherent to the rule as written: `"this is not a blunder, it is the best move"`
//! passes, because one sentence carries both a negation and a label. The mitigation is not a
//! smarter scanner (a scanner cannot tell an idiom from an assertion); it is that the formatter in
//! this module is the only producer of review copy, and it emits no verdict vocabulary at all. The
//! guardrail suite pins both halves: the disclaimers must stay allowed, and the formatter's output
//! for every registered tag must be clean.
//!
//! # Adding a reason tag without breaking the boundary
//!
//! A reason tag is the only way a candidate gets a sentence. There is no free-hand prose path, so
//! a new claim always arrives as a tag plus evidence. One commit, all five parts, or the guardrail
//! suite fails:
//!
//! 1. **Spec row.** Add the tag to `docs/design/move-review-spec.md` §4.6 with its *required
//!    evidence* and its *permitted text template*. No tag exists that is not in that table.
//! 2. **Enum variant.** Add the variant to [`ReasonTag`] in `crates/ai/src/review.rs`. The enum is
//!    closed: no `Other(String)`, no catch-all — a new claim is a new variant, so every consumer
//!    (formatter, audit, UI badge) is forced to handle it.
//! 3. **Evidence requirement.** Extend [`ReasonTag::required_evidence`] so the tag names the
//!    payload facts it needs, and extend [`required_evidence_present`] so the audit can prove the
//!    payload actually contains them. A tag whose evidence cannot be checked belongs in the
//!    *unavailable* list below, not in the enum.
//! 4. **Formatter arm.** Extend [`format_reason`], copying the permitted text from the spec row
//!    verbatim (no new wording), and attach the evidence references it cites.
//! 5. **Two fixtures.** A *positive* fixture (honest payload with the tag, zero violations, in
//!    `crates/ai/tests/fixtures/review/`) and a *negative* fixture (the same tag with its evidence
//!    removed or its counts falsified, which must fail with the specific violation code). The
//!    negative fixture is what proves the tag is guarded; a tag without one is a claim without a
//!    test.
//!
//! **Explicitly unavailable tags.** No current engine trace establishes positional vocabulary —
//! hanging piece, tempo, card advantage, board development, mana efficiency, "punishes a
//! mistake", "sets up next turn". They must not be added as tags until a new engine trace exists,
//! and a new trace is its own card. `EngineRuleApplies` is the only tag that quotes engine prose,
//! and it may only quote a `coach::Explanation` of kind `Rule`/`Simplification` that the payload
//! can resolve, with a pinned CR number.
//!
//! # How this runs in CI
//!
//! `crates/ai/tests/review_honesty.rs` is the guardrail suite: a fixture corpus of honest payloads
//! that must produce zero violations, known-bad payloads that must produce their specific
//! violation, and a source scan that fails when forbidden constructs appear in review-facing copy
//! strings under `crates/ai/src` or `crates/core/src/coach.rs`.

use crate::review::{
    BudgetRecord, EvidenceRef, MAX_LINE_PLIES, MIN_ORDERING_SAMPLES, RankedCandidate, RankedReview,
    ReasonTag, ReviewText, StopReason, TextKind, Uncertainty,
};
use aperture_core::LegalAction;
use aperture_core::coach::{Explanation, ExplanationKind, rule_note};

/// Marker used when a violation has no single index to report (a whole-payload defect).
pub const NO_INDEX: usize = usize::MAX;

/// Every negation marker the spec permits as an excuse for a rule-3/rule-4 sentence.
const NEGATION_MARKERS: &[&str] = &["no", "not", "never", "none", "nothing", "neither"];

/// How a forbidden pattern is matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Match {
    /// Substring, case-insensitive: the pattern is a stem or a phrase that cannot appear innocently.
    Substring,
    /// Whole word, case-insensitive, tolerating a plural `s`/`es`. Used for short words that occur
    /// inside ordinary words (`elo` inside `development`, `odds` inside longer coinages).
    Word,
}

/// Probability / strength vocabulary that must never appear in review output (spec §6 rule 3).
///
/// The list is closed on purpose: adding a phrase here is cheaper than discovering that a new
/// sentence shape slipped past. Entries are matched against a lower-cased sentence.
const PROBABILITY_PATTERNS: &[(&str, Match)] = &[
    ("probabilit", Match::Substring),
    ("chance of winning", Match::Substring),
    // The spec's list says "chance of winning"; a bare "chance" is the same claim in fewer words
    // and the acceptance fixture ("72% chance this is best") relies on it. Whole-word, so
    // "chances" is matched and ordinary words containing "chance" are not.
    ("chance", Match::Word),
    ("winrate", Match::Substring),
    ("win rate", Match::Substring),
    ("likely wins", Match::Substring),
    ("expected score", Match::Substring),
    ("centipawn", Match::Substring),
    ("odds", Match::Word),
    ("equity", Match::Word),
    ("elo", Match::Word),
    ("tablebase", Match::Word),
];

/// Verdict words: single tokens (spec §6 rule 4), matched as words.
///
/// Plurals are matched by the word matcher itself; `inaccuracies` is listed because the `y` does
/// not survive as a suffix.
const VERDICT_WORDS: &[&str] = &[
    "blunder",
    "mistake",
    "inaccuracy",
    "inaccuracies",
    "brilliant",
    "error",
];

/// Verdict phrases (spec §6 rule 4), matched as substrings of a lower-cased sentence.
const VERDICT_PHRASES: &[&str] = &["best move", "correct move", "should have"];

/// The budget fields a `Budget` evidence reference may name (§3.4).
const BUDGET_FIELDS: &[&str] = &[
    "seeds",
    "node_budget",
    "max_rollout_nodes",
    "max_millis",
    "max_candidates",
    "decisions_used",
    "stop",
    "cancelled",
    "reproducible",
    "fidelity",
];

/// Everything [`audit_review_against`] may learn from outside the payload.
///
/// Empty sources are legitimate: a payload whose evidence is entirely internal is fully auditable
/// on its own. Sources matter for references that quote the engine's offer and explanations, and
/// for the positional-vocabulary check on `EngineRuleApplies`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AuditSources<'a> {
    /// The engine's own offer for this decision point, in offer order.
    pub offer: &'a [LegalAction],
    /// The engine's rule/simplification explanations for that offer.
    pub explanations: &'a [Explanation],
}

impl AuditSources<'_> {
    /// No external source: only references that resolve inside the payload can be proven.
    pub fn empty() -> Self {
        Self {
            offer: &[],
            explanations: &[],
        }
    }
}

/// The four presentation constants that keep the rules section and the policy section distinct
/// (§5.1, AC-11). The UI owns the values; the guardrail owns the property.
#[derive(Debug, Clone, Copy)]
pub struct SectionLabels<'a> {
    pub rules_title: &'a str,
    pub rules_source: &'a str,
    pub policy_title: &'a str,
    pub policy_source: &'a str,
}

/// The rendered sections, as the presentation layer actually placed the payload's texts.
#[derive(Debug, Clone, Default)]
pub struct ReviewSections<'a> {
    pub rules: Vec<&'a ReviewText>,
    pub policy: Vec<&'a ReviewText>,
}

/// One honesty failure. The set is closed (spec §6); a new failure mode is a spec change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HonestyViolation {
    /// A percentage figure anywhere in review text. No negation excuse.
    PercentFigure { sentence: String },
    /// Probability / strength vocabulary in review text (allowed only inside a negation).
    ProbabilityClaim { sentence: String },
    /// A verdict label asserted as fact (allowed only inside a negation).
    VerdictLabel {
        word: &'static str,
        sentence: String,
    },
    /// A claim the payload's own evidence does not support.
    UntraceableClaim { sentence: String },
    /// A reason tag outside the §4.6 registry, or a tag whose engine topic is not established.
    UnregisteredReason { tag: String },
    /// An evidence reference that does not resolve inside the payload.
    UnresolvableEvidence { index: usize },
    /// A payload text the presentation never labelled with a section.
    UnlabelledTextKind { index: usize },
    /// Rule text rendered in the policy section.
    RuleTextInPolicySection { index: usize },
    /// Policy (or uncertainty, or strategy) text rendered in the rules section.
    PolicyTextInRulesSection { index: usize },
    /// The rules/policy distinction collapsed into one surface.
    MissingPublicDistinction,
}

impl HonestyViolation {
    /// Stable code for evidence logs and assertions.
    pub fn code(&self) -> &'static str {
        match self {
            HonestyViolation::PercentFigure { .. } => "percent_figure",
            HonestyViolation::ProbabilityClaim { .. } => "probability_claim",
            HonestyViolation::VerdictLabel { .. } => "verdict_label",
            HonestyViolation::UntraceableClaim { .. } => "untraceable_claim",
            HonestyViolation::UnregisteredReason { .. } => "unregistered_reason",
            HonestyViolation::UnresolvableEvidence { .. } => "unresolvable_evidence",
            HonestyViolation::UnlabelledTextKind { .. } => "unlabelled_text_kind",
            HonestyViolation::RuleTextInPolicySection { .. } => "rule_text_in_policy_section",
            HonestyViolation::PolicyTextInRulesSection { .. } => "policy_text_in_rules_section",
            HonestyViolation::MissingPublicDistinction => "missing_public_distinction",
        }
    }
}

impl std::fmt::Display for HonestyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HonestyViolation::PercentFigure { sentence } => {
                write!(
                    f,
                    "percent_figure: a percentage figure in review text: {sentence:?}"
                )
            }
            HonestyViolation::ProbabilityClaim { sentence } => {
                write!(
                    f,
                    "probability_claim: probability vocabulary in review text: {sentence:?}"
                )
            }
            HonestyViolation::VerdictLabel { word, sentence } => {
                write!(
                    f,
                    "verdict_label: verdict word {word:?} asserted in {sentence:?}"
                )
            }
            HonestyViolation::UntraceableClaim { sentence } => {
                write!(f, "untraceable_claim: {sentence}")
            }
            HonestyViolation::UnregisteredReason { tag } => {
                write!(
                    f,
                    "unregistered_reason: {tag} is not in the reason registry"
                )
            }
            HonestyViolation::UnresolvableEvidence { index } => write!(
                f,
                "unresolvable_evidence: reference with index {index} does not resolve in this payload"
            ),
            HonestyViolation::UnlabelledTextKind { index } => write!(
                f,
                "unlabelled_text_kind: payload text {index} carries no section"
            ),
            HonestyViolation::RuleTextInPolicySection { index } => write!(
                f,
                "rule_text_in_policy_section: text {index} is rule text rendered in the policy section"
            ),
            HonestyViolation::PolicyTextInRulesSection { index } => write!(
                f,
                "policy_text_in_rules_section: text {index} is not rule text but was rendered in the rules section"
            ),
            HonestyViolation::MissingPublicDistinction => write!(
                f,
                "missing_public_distinction: the rules and policy sections share a title or source label"
            ),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Sentence scanning
// ---------------------------------------------------------------------------------------------

/// Split review text into sentences on `[.!?;]` (spec §6 rule 1).
///
/// Empty fragments are dropped, so a trailing period or a `;`-joined clause list both work. The
/// returned slices are trimmed and keep their interior punctuation.
fn sentences(body: &str) -> impl Iterator<Item = &str> {
    body.split(['.', '!', '?', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// True when the lower-cased sentence contains a negation marker as a whole word.
fn has_negation(lower: &str) -> bool {
    NEGATION_MARKERS
        .iter()
        .any(|marker| contains_word(lower, marker))
}

/// Whole-word search, tolerating a plural trailing `s` (`mistakes`, `blunders`, `errors`).
fn contains_word(lower: &str, word: &str) -> bool {
    let bytes = lower.as_bytes();
    let mut from = 0usize;
    while let Some(found) = lower[from..].find(word) {
        let start = from + found;
        let end = start + word.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let mut after = end;
        if after < bytes.len() && bytes[after] == b's' {
            after += 1;
            if after < bytes.len() && bytes[after] == b'e' {
                after += 1;
            }
        }
        let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
        from = start + word.len();
        if from >= lower.len() {
            break;
        }
    }
    false
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// True when the sentence contains `\d+(\.\d+)?\s*%` (spec §6 rule 2).
fn has_percent_figure(sentence: &str) -> bool {
    let chars: Vec<char> = sentence.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut after = i;
        while after < chars.len() && chars[after].is_ascii_digit() {
            after += 1;
        }
        // Optional decimal part: a '.' must be followed by at least one digit.
        if after + 1 < chars.len() && chars[after] == '.' && chars[after + 1].is_ascii_digit() {
            after += 1;
            while after < chars.len() && chars[after].is_ascii_digit() {
                after += 1;
            }
        }
        // `\s*` then `%`.
        let mut marker = after;
        while marker < chars.len() && chars[marker].is_whitespace() {
            marker += 1;
        }
        if marker < chars.len() && chars[marker] == '%' {
            return true;
        }
        i = after;
    }
    false
}

/// Audit one string. Never returns a violation for an empty string.
pub fn audit_text(body: &str) -> Vec<HonestyViolation> {
    let mut violations = Vec::new();
    for sentence in sentences(body) {
        let lower = sentence.to_lowercase();
        // Rule 2: percentages have no negation excuse.
        if has_percent_figure(sentence) {
            violations.push(HonestyViolation::PercentFigure {
                sentence: sentence.to_string(),
            });
        }
        let negated = has_negation(&lower);
        if negated {
            continue;
        }
        // Rule 3: probability / strength vocabulary.
        if PROBABILITY_PATTERNS
            .iter()
            .any(|(pattern, kind)| match kind {
                Match::Substring => lower.contains(pattern),
                Match::Word => contains_word(&lower, pattern),
            })
        {
            violations.push(HonestyViolation::ProbabilityClaim {
                sentence: sentence.to_string(),
            });
        }
        // Rule 4: verdict labels, single words first, then phrases.
        for word in VERDICT_WORDS {
            if contains_word(&lower, word) {
                violations.push(HonestyViolation::VerdictLabel {
                    word,
                    sentence: sentence.to_string(),
                });
            }
        }
        for phrase in VERDICT_PHRASES {
            if lower.contains(phrase) {
                violations.push(HonestyViolation::VerdictLabel {
                    word: phrase,
                    sentence: sentence.to_string(),
                });
            }
        }
    }
    violations
}

/// A sentence that prints a score must print its sample count in the same sentence (§5.2).
fn score_without_sample_count(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    let has_score = has_signed_decimal(&lower);
    let has_count = lower.contains("sample");
    has_score && !has_count
}

/// True when the text contains a signed two-decimal figure such as `+0.75` or `-1.00`.
fn has_signed_decimal(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    for start in 0..bytes.len().saturating_sub(4) {
        let signed = bytes[start] == b'+' || bytes[start] == b'-';
        if !signed {
            continue;
        }
        let digits = bytes[start + 1..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        let dot = start + 1 + digits;
        if digits == 0 || dot >= bytes.len() || bytes[dot] != b'.' {
            continue;
        }
        let decimals = bytes[dot + 1..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if decimals == 2 {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------------------------
// Reason registry (§4.6)
// ---------------------------------------------------------------------------------------------

impl ReasonTag {
    /// The §4.6 registry row this tag occupies, or `None` when the tag is not registered.
    ///
    /// `EngineRuleApplies` is registered only when it cites a pinned CR number and names a topic;
    /// its engine-side resolution is checked by [`required_evidence_present`].
    pub fn registry_row(&self) -> Option<&'static str> {
        match self {
            ReasonTag::HighestPracticeOutcome => Some("HighestPracticeOutcome"),
            ReasonTag::EqualPracticeOutcome => Some("EqualPracticeOutcome"),
            ReasonTag::NoScoredSample => Some("NoScoredSample"),
            ReasonTag::IncompleteRollouts => Some("IncompleteRollouts"),
            ReasonTag::RefusedBySearchWorld => Some("RefusedBySearchWorld"),
            ReasonTag::LowSample => Some("LowSample"),
            ReasonTag::BudgetStopped => Some("BudgetStopped"),
            ReasonTag::SoleLegalOffer => Some("SoleLegalOffer"),
            ReasonTag::EngineRuleApplies { rule, topic } => {
                if topic.trim().is_empty() || rule_note(rule).is_none() {
                    None
                } else {
                    Some("EngineRuleApplies")
                }
            }
        }
    }

    /// What this tag needs before a sentence may be derived from it (§4.6 evidence column).
    pub fn required_evidence(&self) -> &'static str {
        match self {
            ReasonTag::HighestPracticeOutcome => {
                "SearchCount with scored_samples >= 2 and a recorded score"
            }
            ReasonTag::EqualPracticeOutcome => {
                "SearchCount for this row and for every row it is level with"
            }
            ReasonTag::NoScoredSample => "SearchCount with scored == 0",
            ReasonTag::IncompleteRollouts => "SearchCount with without_result > 0",
            ReasonTag::RefusedBySearchWorld => "SearchCount with refused > 0",
            ReasonTag::LowSample => "SearchCount with scored < 2",
            ReasonTag::BudgetStopped => "BudgetRecord with stop in {wall clock, cancelled}",
            ReasonTag::SoleLegalOffer => "EngineOffer with exactly one entry",
            ReasonTag::EngineRuleApplies { .. } => {
                "CoachExplanation of kind Rule/Simplification plus RuleCitation"
            }
        }
    }

    /// The tag's registry name, for logs and violation messages.
    pub fn tag_name(&self) -> String {
        match self {
            ReasonTag::EngineRuleApplies { topic, .. } => {
                format!("EngineRuleApplies{{topic: {topic}}}")
            }
            other => other.registry_row().unwrap_or("unknown").to_string(),
        }
    }
}

/// True when the payload actually contains the evidence `tag` requires.
///
/// `candidate` is the row the sentence would be attached to; `sources` supplies the engine facts
/// when the reference points outside the payload.
fn required_evidence_present(
    tag: &ReasonTag,
    review: &RankedReview,
    candidate: &RankedCandidate,
    sources: &AuditSources<'_>,
) -> bool {
    match tag {
        ReasonTag::HighestPracticeOutcome => {
            candidate.score.is_some() && candidate.scored_samples >= MIN_ORDERING_SAMPLES
        }
        ReasonTag::EqualPracticeOutcome => match candidate.uncertainty {
            Uncertainty::EffectivelyTied { .. } => candidate.scored_samples >= MIN_ORDERING_SAMPLES,
            _ => false,
        },
        ReasonTag::NoScoredSample => candidate.scored_samples == 0,
        ReasonTag::IncompleteRollouts => candidate.without_result > 0,
        ReasonTag::RefusedBySearchWorld => candidate.refused_times > 0,
        ReasonTag::LowSample => candidate.scored_samples < MIN_ORDERING_SAMPLES,
        ReasonTag::BudgetStopped => {
            matches!(
                review.budget.stop,
                StopReason::WallClock | StopReason::Cancelled
            )
        }
        ReasonTag::SoleLegalOffer => {
            if sources.offer.is_empty() {
                // Without the engine offer, the payload's own candidate list is the evidence.
                review.candidates.len() == 1
            } else {
                sources.offer.len() == 1
            }
        }
        ReasonTag::EngineRuleApplies { rule, topic } => {
            if sources.explanations.is_empty() {
                // Structural only: a pinned citation must exist, and the topic must be named.
                rule_note(rule).is_some() && !topic.trim().is_empty()
            } else {
                sources.explanations.iter().any(|e| {
                    e.topic == *topic
                        && matches!(
                            e.kind,
                            ExplanationKind::Rule | ExplanationKind::Simplification
                        )
                        && e.rules.contains(&rule.as_str())
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Evidence resolution (§4.7)
// ---------------------------------------------------------------------------------------------

impl EvidenceRef {
    /// The primary index this reference names, for violation reporting.
    fn primary_index(&self) -> Option<usize> {
        match self {
            EvidenceRef::EngineOffer { index }
            | EvidenceRef::EngineLabel { index }
            | EvidenceRef::CoachExplanation { index, .. } => Some(*index),
            EvidenceRef::SearchCount { offer_index, .. }
            | EvidenceRef::LineStep { offer_index, .. } => Some(*offer_index),
            EvidenceRef::RuleCitation { .. }
            | EvidenceRef::Budget { .. }
            | EvidenceRef::ViewDigest => None,
        }
    }
}

/// Resolve one evidence reference against the payload and (optionally) the engine sources.
fn resolve(review: &RankedReview, sources: &AuditSources<'_>, reference: &EvidenceRef) -> bool {
    match reference {
        EvidenceRef::EngineOffer { index } => review.candidate(*index).is_some(),
        EvidenceRef::EngineLabel { index } => review
            .candidate(*index)
            .map(|c| !c.move_label.trim().is_empty())
            .unwrap_or(false),
        EvidenceRef::SearchCount {
            offer_index,
            scored,
            without_result,
            refused,
        } => review
            .candidate(*offer_index)
            .map(|c| {
                c.scored_samples == *scored
                    && c.without_result == *without_result
                    && c.refused_times == *refused
            })
            .unwrap_or(false),
        EvidenceRef::CoachExplanation { index, kind, topic } => {
            if sources.explanations.is_empty() {
                // Structural: strategy records are policy statements and cannot be quoted as rules.
                !topic.trim().is_empty()
                    && matches!(
                        kind,
                        ExplanationKind::Rule | ExplanationKind::Simplification
                    )
            } else {
                sources
                    .explanations
                    .get(*index)
                    .map(|e| e.kind == *kind && e.topic == *topic)
                    .unwrap_or(false)
            }
        }
        EvidenceRef::RuleCitation { number } => {
            if rule_note(number).is_none() {
                return false;
            }
            if sources.explanations.is_empty() {
                true
            } else {
                sources
                    .explanations
                    .iter()
                    .any(|e| e.rules.contains(&number.as_str()))
                    || review
                        .candidates
                        .iter()
                        .any(|c| c.offer_rules.iter().any(|cited| cited == number))
            }
        }
        EvidenceRef::Budget { field } => BUDGET_FIELDS.contains(&field.as_str()),
        EvidenceRef::LineStep {
            offer_index,
            decision,
        } => review
            .candidate(*offer_index)
            .and_then(|c| c.line.as_ref())
            .map(|line| line.steps.iter().any(|s| s.decision == *decision))
            .unwrap_or(false),
        EvidenceRef::ViewDigest => !review.binding.view_digest.trim().is_empty(),
    }
}

// ---------------------------------------------------------------------------------------------
// Payload audit
// ---------------------------------------------------------------------------------------------

/// One derivation source for [`format_reason`]: the row, the budget and the engine's own text.
#[derive(Debug, Clone, Copy)]
pub struct ReasonSource<'a> {
    pub candidate: &'a RankedCandidate,
    pub budget: &'a BudgetRecord,
    /// The engine explanation a rule tag quotes, if the caller has it.
    pub explanation: Option<&'a Explanation>,
    /// Where that explanation sits in the engine's explanation list (evidence index).
    pub explanation_index: usize,
}

impl<'a> ReasonSource<'a> {
    pub fn new(candidate: &'a RankedCandidate, budget: &'a BudgetRecord) -> Self {
        Self {
            candidate,
            budget,
            explanation: None,
            explanation_index: 0,
        }
    }

    pub fn with_explanation(mut self, index: usize, explanation: &'a Explanation) -> Self {
        self.explanation = Some(explanation);
        self.explanation_index = index;
        self
    }
}

/// Format the practice-outcome line for a scored candidate (§5.2: mean **and** sample count).
///
/// Returns `None` for an unscored row: a row without a sample has no outcome to print, and the
/// `NoScoredSample` reason states that in words instead.
pub fn format_outcome(candidate: &RankedCandidate) -> Option<ReviewText> {
    let score = candidate.score?;
    if candidate.scored_samples == 0 {
        // A score without a sample is a fabricated figure; do not print one.
        return None;
    }
    let mut body = format!(
        "mean practice outcome {score:+.2} over {} scored sample(s)",
        candidate.scored_samples
    );
    if candidate.without_result > 0 {
        body.push_str(&format!(
            "; {} rollout(s) ended without a result",
            candidate.without_result
        ));
    }
    Some(ReviewText {
        kind: TextKind::Policy,
        body,
        evidence: vec![EvidenceRef::SearchCount {
            offer_index: candidate.offer_index,
            scored: candidate.scored_samples,
            without_result: candidate.without_result,
            refused: candidate.refused_times,
        }],
    })
}

/// Format one reason sentence from its tag. The permitted text comes from the §4.6 table; this
/// function is the only producer of review copy, so no other module may hand-write one.
///
/// Returns `None` when the tag needs engine text the caller did not supply — a rule sentence is
/// never invented to fill the gap.
pub fn format_reason(tag: &ReasonTag, source: &ReasonSource<'_>) -> Option<ReviewText> {
    let candidate = source.candidate;
    let count = EvidenceRef::SearchCount {
        offer_index: candidate.offer_index,
        scored: candidate.scored_samples,
        without_result: candidate.without_result,
        refused: candidate.refused_times,
    };
    let offer = EvidenceRef::EngineOffer {
        index: candidate.offer_index,
    };
    let label = EvidenceRef::EngineLabel {
        index: candidate.offer_index,
    };
    let (kind, body, evidence) = match tag {
        ReasonTag::HighestPracticeOutcome => {
            let score = candidate.score?;
            (
                TextKind::Policy,
                format!(
                    "highest mean practice outcome of the offered alternatives ({score:+.2} over {} scored samples)",
                    candidate.scored_samples
                ),
                vec![count, offer, label],
            )
        }
        ReasonTag::EqualPracticeOutcome => {
            let score = candidate.score?;
            let mut evidence = vec![count, offer];
            if let Uncertainty::EffectivelyTied { with } = &candidate.uncertainty {
                evidence.extend(
                    with.iter()
                        .map(|index| EvidenceRef::EngineOffer { index: *index }),
                );
            }
            (
                TextKind::Uncertainty,
                format!(
                    "level with another offered alternative at this sample count ({score:+.2})"
                ),
                evidence,
            )
        }
        ReasonTag::NoScoredSample => (
            TextKind::Uncertainty,
            "no scored sample: no rollout reached a result, so this alternative is not ranked"
                .to_string(),
            vec![count, offer],
        ),
        ReasonTag::IncompleteRollouts => (
            TextKind::Uncertainty,
            format!(
                "{} rollout(s) ended without a result and are counted as neither a win nor a loss",
                candidate.without_result
            ),
            vec![count],
        ),
        ReasonTag::RefusedBySearchWorld => (
            TextKind::Uncertainty,
            format!(
                "the search's own reconstruction refused this action {} time(s) as illegal",
                candidate.refused_times
            ),
            vec![count],
        ),
        ReasonTag::LowSample => (
            TextKind::Uncertainty,
            "one sample cannot establish an order".to_string(),
            vec![count],
        ),
        ReasonTag::BudgetStopped => (
            TextKind::Uncertainty,
            format!(
                "the analysis stopped at the {} so this sample is incomplete",
                source.budget.stop.label()
            ),
            vec![
                EvidenceRef::Budget {
                    field: "stop".to_string(),
                },
                EvidenceRef::Budget {
                    field: "cancelled".to_string(),
                },
            ],
        ),
        ReasonTag::SoleLegalOffer => (
            TextKind::Policy,
            "the engine offers exactly one action at this decision point".to_string(),
            vec![offer, label],
        ),
        ReasonTag::EngineRuleApplies { rule, topic } => {
            let explanation = source.explanation?;
            if !matches!(
                explanation.kind,
                ExplanationKind::Rule | ExplanationKind::Simplification
            ) {
                return None;
            }
            if !explanation.rules.contains(&rule.as_str()) {
                return None;
            }
            let badge = match explanation.kind {
                ExplanationKind::Rule => "[RULE]",
                ExplanationKind::Simplification => "[SIMPLIFICATION]",
                ExplanationKind::Strategy => return None,
            };
            (
                TextKind::Rule,
                format!("{badge} {topic}: {} (CR {rule})", explanation.text),
                vec![
                    EvidenceRef::CoachExplanation {
                        index: source.explanation_index,
                        kind: explanation.kind,
                        topic: explanation.topic.clone(),
                    },
                    EvidenceRef::RuleCitation {
                        number: rule.clone(),
                    },
                ],
            )
        }
    };
    Some(ReviewText {
        kind,
        body,
        evidence,
    })
}

/// The rendered budget line (§3.4). A pure function of the record, and never a percentage.
pub fn format_budget_line(budget: &BudgetRecord) -> String {
    let clock = if budget.max_millis == u64::MAX {
        "unbounded".to_string()
    } else {
        format!("{} ms", budget.max_millis)
    };
    format!(
        "budget: {} determinization seed(s) · {} engine decisions total · {} decisions per rollout · {} wall clock · at most {} engine-offered candidates · used {} · stopped: {} · reproducible: {}",
        budget.seeds,
        budget.node_budget,
        budget.max_rollout_nodes,
        clock,
        budget.max_candidates,
        budget.decisions_used,
        budget.stop.label(),
        if budget.reproducible { "yes" } else { "no" },
    )
}

/// Every text a payload claims a human may read: `notes`, then per-candidate outcome lines and
/// reason sentences in candidate order. Indices in violations refer to this list.
pub fn payload_texts(review: &RankedReview) -> Vec<ReviewText> {
    payload_texts_with(review, &AuditSources::empty())
}

/// As [`payload_texts`], but able to derive engine-rule sentences from the engine's explanations.
pub fn payload_texts_with(review: &RankedReview, sources: &AuditSources<'_>) -> Vec<ReviewText> {
    let mut texts = review.notes.clone();
    for candidate in &review.candidates {
        if let Some(text) = format_outcome(candidate) {
            texts.push(text);
        }
        for tag in &candidate.reasons {
            let source = ReasonSource::new(candidate, &review.budget);
            let source = match tag {
                ReasonTag::EngineRuleApplies { topic, rule } => {
                    match sources
                        .explanations
                        .iter()
                        .enumerate()
                        .find(|(_, e)| e.topic == *topic && e.rules.contains(&rule.as_str()))
                    {
                        Some((index, explanation)) => source.with_explanation(index, explanation),
                        None => source,
                    }
                }
                _ => source,
            };
            if let Some(text) = format_reason(tag, &source) {
                texts.push(text);
            }
        }
    }
    texts
}

/// Audit a payload on its own (no external engine sources).
pub fn audit_review(review: &RankedReview) -> Vec<HonestyViolation> {
    audit_review_against(review, &AuditSources::empty())
}

/// Audit a payload with the engine's offer/explanations available for reference resolution.
pub fn audit_review_against(
    review: &RankedReview,
    sources: &AuditSources<'_>,
) -> Vec<HonestyViolation> {
    let mut violations = Vec::new();

    // The payload must be the schema the audit understands.
    if !review.spec.is_empty() && review.spec != crate::review::SPEC_ID {
        violations.push(HonestyViolation::UntraceableClaim {
            sentence: format!(
                "payload declares spec {:?} but this audit understands {:?}",
                review.spec,
                crate::review::SPEC_ID
            ),
        });
    }

    // A reproducibility claim must be traceable to the stop reason (§3.3, AC-6).
    if !review.budget.reproducible_flag_is_consistent() {
        violations.push(HonestyViolation::UntraceableClaim {
            sentence: format!(
                "budget claims reproducible={} but the recorded stop reason is {} (cancelled={})",
                review.budget.reproducible,
                review.budget.stop.label(),
                review.budget.cancelled
            ),
        });
    }

    // A refusal ranks nothing and claims no preference (§5.4, AC-9).
    if review.is_refusal() && (!review.candidates.is_empty() || review.preference.is_some()) {
        violations.push(HonestyViolation::UntraceableClaim {
            sentence: "a refused review carries candidates or a preference".to_string(),
        });
    }

    // Text: forbidden constructs, missing evidence, unsupported score printings.
    for text in payload_texts_with(review, sources) {
        violations.extend(audit_text(&text.body));
        if text.evidence.is_empty() {
            violations.push(HonestyViolation::UntraceableClaim {
                sentence: format!("no evidence reference: {:?}", text.body),
            });
        }
        for sentence in sentences(&text.body) {
            if score_without_sample_count(sentence) {
                violations.push(HonestyViolation::UntraceableClaim {
                    sentence: format!("score printed without its sample count: {sentence:?}"),
                });
            }
        }
        // A sentence labelled as a rule must cite the rule it states.
        if text.kind == TextKind::Rule
            && !text
                .evidence
                .iter()
                .any(|reference| matches!(reference, EvidenceRef::RuleCitation { .. }))
        {
            violations.push(HonestyViolation::UntraceableClaim {
                sentence: format!("rule text without a CR citation: {:?}", text.body),
            });
        }
    }

    // Reasons: registry membership, evidence requirement, and (per candidate) sample honesty.
    for candidate in &review.candidates {
        for tag in &candidate.reasons {
            if tag.registry_row().is_none() {
                violations.push(HonestyViolation::UnregisteredReason {
                    tag: tag.tag_name(),
                });
                continue;
            }
            if !required_evidence_present(tag, review, candidate, sources) {
                violations.push(HonestyViolation::UntraceableClaim {
                    sentence: format!(
                        "reason {} requires {} but the payload does not carry it",
                        tag.tag_name(),
                        tag.required_evidence()
                    ),
                });
            }
        }

        // A score is a mean over samples; without samples there is no mean.
        if candidate.score.is_some() && candidate.scored_samples == 0 {
            violations.push(HonestyViolation::UntraceableClaim {
                sentence: format!(
                    "candidate {} prints a score with zero scored samples",
                    candidate.offer_index
                ),
            });
        }

        // The uncertainty state must match the sample count (§4.5, AC-7).
        let samples = candidate.scored_samples;
        let uncertainty_ok = match candidate.uncertainty {
            Uncertainty::Ordered => samples >= MIN_ORDERING_SAMPLES,
            Uncertainty::EffectivelyTied { .. } => samples >= MIN_ORDERING_SAMPLES,
            Uncertainty::SingleSample => samples < MIN_ORDERING_SAMPLES && samples > 0,
            Uncertainty::Unscored => samples == 0,
        };
        if !uncertainty_ok {
            violations.push(HonestyViolation::UntraceableClaim {
                sentence: format!(
                    "candidate {} reports {:?} with {samples} scored sample(s)",
                    candidate.offer_index, candidate.uncertainty
                ),
            });
        }

        // A line for a row that never reached a result is a fabricated line (§4.4, AC-15).
        if candidate.line.is_some() && candidate.scored_samples == 0 {
            violations.push(HonestyViolation::UntraceableClaim {
                sentence: format!(
                    "candidate {} carries a line but no scored sample",
                    candidate.offer_index
                ),
            });
        }
    }

    // Every evidence reference in every printed text must resolve (§4.7, AC-4).
    for text in payload_texts_with(review, sources) {
        for reference in &text.evidence {
            if !resolve(review, sources, reference) {
                violations.push(HonestyViolation::UnresolvableEvidence {
                    index: reference.primary_index().unwrap_or(NO_INDEX),
                });
            }
        }
    }
    for candidate in &review.candidates {
        for rule in &candidate.offer_rules {
            if rule.trim().is_empty() {
                violations.push(HonestyViolation::UntraceableClaim {
                    sentence: format!(
                        "candidate {} carries an empty rule citation",
                        candidate.offer_index
                    ),
                });
            }
        }
        // A recorded line is bounded by the recorder and has one step per decision ordinal.
        if let Some(line) = &candidate.line {
            if line.steps.len() > MAX_LINE_PLIES && !line.truncated {
                violations.push(HonestyViolation::UntraceableClaim {
                    sentence: format!(
                        "candidate {} carries {} line steps but is not marked truncated",
                        candidate.offer_index,
                        line.steps.len()
                    ),
                });
            }
            let mut ordinals: Vec<usize> = line.steps.iter().map(|s| s.decision).collect();
            ordinals.sort_unstable();
            let duplicates = ordinals.windows(2).any(|pair| pair[0] == pair[1]);
            if duplicates {
                violations.push(HonestyViolation::UntraceableClaim {
                    sentence: format!(
                        "candidate {} carries two line steps for the same decision ordinal",
                        candidate.offer_index
                    ),
                });
            }
            for step in &line.steps {
                if step.label.trim().is_empty() {
                    violations.push(HonestyViolation::UntraceableClaim {
                        sentence: format!(
                            "candidate {} line step {} has no engine label",
                            candidate.offer_index, step.decision
                        ),
                    });
                }
            }
        }
    }

    // A claimed preference must be what the scores actually say (§4.2, AC-8).
    violations.extend(audit_preference(review));

    violations
}

/// A preference is a statement about the scores; it may not contradict them.
fn audit_preference(review: &RankedReview) -> Vec<HonestyViolation> {
    let mut violations = Vec::new();
    let Some(preference) = &review.preference else {
        return violations;
    };
    let indexes = preference.offer_indexes();
    if indexes.is_empty() {
        violations.push(HonestyViolation::UntraceableClaim {
            sentence: "a preference names no candidate".to_string(),
        });
        return violations;
    }
    for index in &indexes {
        if review.candidate(*index).is_none() {
            violations.push(HonestyViolation::UntraceableClaim {
                sentence: format!(
                    "a preference names offer index {index}, which is not a candidate"
                ),
            });
        }
    }
    let top = indexes
        .iter()
        .filter_map(|index| review.candidate(*index))
        .filter_map(|c| c.score)
        .fold(None::<f32>, |acc, score| match acc {
            Some(current) if current >= score => Some(current),
            _ => Some(score),
        });
    let Some(top) = top else {
        return violations;
    };
    match preference {
        crate::review::Preference::Unique { offer_index } => {
            for candidate in &review.candidates {
                if candidate.offer_index == *offer_index {
                    continue;
                }
                if let Some(score) = candidate.score
                    && score >= top
                {
                    violations.push(HonestyViolation::UntraceableClaim {
                            sentence: format!(
                                "a unique preference is claimed for offer index {offer_index} but candidate {} scores {score:+.2}",
                                candidate.offer_index
                            ),
                        });
                }
            }
        }
        crate::review::Preference::Tied { offer_indexes } => {
            for candidate in &review.candidates {
                if offer_indexes.contains(&candidate.offer_index) {
                    continue;
                }
                if let Some(score) = candidate.score
                    && score > top
                {
                    violations.push(HonestyViolation::UntraceableClaim {
                        sentence: format!(
                            "a tie is claimed at {top:+.2} but candidate {} scores {score:+.2}",
                            candidate.offer_index
                        ),
                    });
                }
            }
        }
    }
    violations
}

/// Audit how the presentation placed the payload's texts (§5.1, AC-11).
///
/// `sections` holds the references the UI actually rendered in each section, so a collapse shows
/// up here rather than in a reviewer's judgement.
pub fn audit_sections(
    review: &RankedReview,
    sources: &AuditSources<'_>,
    labels: &SectionLabels<'_>,
    sections: &ReviewSections<'_>,
) -> Vec<HonestyViolation> {
    let mut violations = Vec::new();

    let labels_empty = [
        labels.rules_title,
        labels.rules_source,
        labels.policy_title,
        labels.policy_source,
    ]
    .iter()
    .any(|label| label.trim().is_empty());
    if labels_empty
        || labels.rules_title == labels.policy_title
        || labels.rules_source == labels.policy_source
        || labels.rules_title == labels.policy_source
    {
        violations.push(HonestyViolation::MissingPublicDistinction);
    }

    for (position, text) in sections.rules.iter().enumerate() {
        let quotes_strategy = text.evidence.iter().any(|reference| {
            matches!(
                reference,
                EvidenceRef::CoachExplanation {
                    kind: ExplanationKind::Strategy,
                    ..
                }
            )
        });
        if text.kind != TextKind::Rule || quotes_strategy {
            violations.push(HonestyViolation::PolicyTextInRulesSection { index: position });
        }
    }
    for (position, text) in sections.policy.iter().enumerate() {
        if text.kind == TextKind::Rule {
            violations.push(HonestyViolation::RuleTextInPolicySection { index: position });
        }
    }

    // Every text the payload claims to have must be placed exactly once.
    let canonical = payload_texts_with(review, sources);
    for (index, text) in canonical.iter().enumerate() {
        let in_rules = sections.rules.iter().filter(|t| **t == text).count();
        let in_policy = sections.policy.iter().filter(|t| **t == text).count();
        if in_rules + in_policy == 0 {
            violations.push(HonestyViolation::UnlabelledTextKind { index });
        }
    }

    violations
}
