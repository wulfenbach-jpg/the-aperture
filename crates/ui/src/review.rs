//! Move review: the presentation layer over the ranking contract
//! (`aperture_ai::review`), following `docs/design/move-review-spec.md` §5.
//!
//! This module renders. It computes no rule, runs no search, submits no action
//! and never decides what a candidate is: every number, label, line step and
//! sentence it prints comes from a [`RankedReview`] the ranking layer produced
//! (D-1). Four properties are structural, not conventions:
//!
//! 1. **Lines are recorded traces.** A candidate line is rendered from
//!    `CandidateLine.steps`, which the search recorded while a rollout was
//!    actually played by the engine. This module cannot add, reorder or extend a
//!    step, and never builds one from a label it has lying around. When the
//!    payload carries no line, the panel says so and says why (§4.4).
//! 2. **Uncertainty is words and counts.** Every ranked row prints its mean
//!    *and* the number of scored samples behind it; rows the payload marks
//!    `EffectivelyTied` render as one unordered group; a one-sample row says it
//!    is not ordered; offers with no scored sample go in a separate unranked
//!    list. There is no bar, gauge, gradient, percentage or certainty figure
//!    anywhere in this module, and a test scans every rendered line for one.
//! 3. **Reason tags are teaching notes.** A candidate's reason tags go through
//!    the ranking layer's single reason formatter and print under the row with a
//!    label naming them a heuristic. Rule-kind sentences are never part of that
//!    note; they belong to the rules block.
//! 4. **Rules and policy are two blocks with two constructors.** Rule text comes
//!    only from [`rules_section`] (engine trace explanations) plus the payload's
//!    own `TextKind::Rule` sentences; preference text comes only from this type.
//!    Different titles, source labels and badges, and no line crossing between
//!    them — asserted on the rendered output, not on intent (AC-11).
//!
//! Text kinds are payload data (§4.8): the renderer maps `TextKind` to a block
//! and a badge and never re-classifies a sentence.

use aperture_ai::SearchConfig;
use aperture_ai::review::{
    BudgetRecord, CandidateLine, LineScope, RankedCandidate, RankedReview, ReasonTag,
    ReviewBinding, TextKind, Uncertainty, reason_text,
};
use aperture_core::PlayerId;
use aperture_core::coach::{self, Explanation, ExplanationKind};
use aperture_core::view::PlayerView;

/// The label shown when the reviewing viewer holds no decision to review. This
/// is the one refusal this module states itself; every other refusal is the
/// search's own typed reason, carried verbatim through the payload.
pub const NOT_YOUR_DECISION: &str = "the reviewing seat holds no engine offer right now (it is not that seat's decision point); there is nothing to rank";

/// Title of the policy-preference block. Deliberately not a verdict.
pub const POLICY_TITLE: &str =
    "MOVE REVIEW — ranked practice outcomes (policy preference, not a rule)";
/// Source label of the policy-preference block.
pub const POLICY_SOURCE: &str = "source: aperture_ai::review over engine-offered actions · practice outcomes, not a chance of winning";
/// Title of the rules block.
pub const RULES_TITLE: &str = "RULES FOR THESE OFFERED ACTIONS — engine trace";
/// Source label of the rules block.
pub const RULES_SOURCE: &str =
    "source: engine rule/trace explanations and the pinned CR text · never a policy preference";

/// The label every reason-tag sentence carries. A reason tag is a heuristic
/// about the search's own counts: not a rule statement, and not a verdict.
pub const TEACHING_NOTE_LABEL: &str =
    "teaching note (heuristic from the search's reason tags — not a rule statement)";

/// Why a row that scored carries no line.
const LINE_NONE_UNRECORDED: &str =
    "the search recorded no line for this offer, and this panel does not synthesise one";
/// Why a row that never scored carries no line.
const LINE_NONE_NO_RESULT: &str =
    "no rollout reached a result for this offer, so there was no line to record";

/// The standing disclosure that this panel passes no verdict.
const NO_VERDICT_NOTE: &str = "this panel reports the engine's own counts and the search's own preference only: it passes no verdict on your play, and it labels no alternative as an error";

/// One titled block of the review panel, with its own source label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSection {
    pub title: &'static str,
    pub source: &'static str,
    pub lines: Vec<String>,
}

/// Both blocks of the panel, each built by its own constructor (§5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSections {
    pub rules: ReviewSection,
    pub policy: ReviewSection,
}

/// One finished move review as the panel shows it: the ranking payload verbatim,
/// plus the policy block rendered from it once.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveReview {
    ranked: RankedReview,
    policy: ReviewSection,
}

/// A review of a decision point the search did not run (§5.4): no ranking, the
/// typed reason verbatim. Forwarded to the ranking layer's own refusal
/// constructor, so the UI never invents payload data.
pub fn refused(
    view: &PlayerView,
    config: &SearchConfig,
    binding: ReviewBinding,
    reason: String,
) -> MoveReview {
    MoveReview::from_ranked(aperture_ai::review::refused(view, config, binding, reason))
}

impl MoveReview {
    /// Wrap a payload. The policy block is rendered once, here.
    pub fn from_ranked(ranked: RankedReview) -> Self {
        let policy = ReviewSection {
            title: POLICY_TITLE,
            source: POLICY_SOURCE,
            lines: policy_lines(&ranked),
        };
        MoveReview { ranked, policy }
    }

    /// The payload this panel presents, so a caller can audit exactly what was
    /// rendered instead of trusting the formatted text.
    pub fn ranked(&self) -> &RankedReview {
        &self.ranked
    }

    /// What this review is bound to (§5.5).
    pub fn binding(&self) -> &ReviewBinding {
        &self.ranked.binding
    }

    pub fn seat(&self) -> PlayerId {
        self.ranked.binding.seat
    }

    pub fn generation(&self) -> u64 {
        self.ranked.binding.generation
    }

    pub fn revision(&self) -> u64 {
        self.ranked.binding.revision
    }

    pub fn decision(&self) -> u64 {
        self.ranked.binding.decision
    }

    pub fn view_digest(&self) -> &str {
        &self.ranked.binding.view_digest
    }

    /// The typed refusal, when the search produced no ranking at all.
    pub fn refusal(&self) -> Option<&str> {
        self.ranked.refusal.as_deref()
    }

    /// Why this review may not be shown against `live`, if it may not (§5.5,
    /// AC-14). The digest is part of the identity: two different games can
    /// reproduce the same seat/revision/decision triple.
    pub fn discard_reason(&self, live: &ReviewBinding) -> Option<&'static str> {
        let bound = self.binding();
        if bound.generation != live.generation {
            Some("stale_generation")
        } else if bound.revision != live.revision {
            Some("stale_revision")
        } else if bound.decision != live.decision {
            Some("stale_decision")
        } else if bound.seat != live.seat {
            Some("wrong_seat")
        } else if bound.view_digest != live.view_digest {
            Some("stale_view_digest")
        } else {
            None
        }
    }

    /// The policy-preference block. Never rule text.
    pub fn policy_section(&self) -> &ReviewSection {
        &self.policy
    }

    /// The payload's own rule-kind sentences: `TextKind::Rule` notes and the
    /// rule-kind reason tags. The ranking layer emits none today; when it does,
    /// they belong to the rules block and never to the policy block (§4.8).
    pub fn rule_notes(&self) -> Vec<String> {
        rule_note_bodies(&self.ranked)
    }

    /// Both blocks at once, each from its own producer. The rules block is the
    /// engine trace's own explanations plus the payload's rule-kind sentences;
    /// the policy block is this type's own rendering. Nothing here writes rule
    /// text into the policy block or a preference into the rules block.
    pub fn sections(&self, explanations: &[Explanation]) -> ReviewSections {
        let mut rules = rules_section(explanations);
        rules.lines.extend(self.rule_notes());
        ReviewSections {
            rules,
            policy: self.policy.clone(),
        }
    }

    /// One line for the busy/status readout when a review is applied.
    pub fn summary_line(&self) -> String {
        if let Some(reason) = self.refusal() {
            return format!(
                "move review refused for P{}: {reason} (nothing is ranked)",
                self.seat()
            );
        }
        format!(
            "move review ready for P{}: {} of {} engine-offered candidate(s) scored over {} engine decision(s)",
            self.seat(),
            scored_rows(&self.ranked).len(),
            self.ranked.candidates.len(),
            self.ranked.budget.decisions_used
        )
    }
}

fn is_rule_tag(tag: &ReasonTag) -> bool {
    matches!(tag, ReasonTag::EngineRuleApplies { .. })
}

/// The payload's own rule-kind sentences, from notes and from reason tags.
fn rule_note_bodies(ranked: &RankedReview) -> Vec<String> {
    let mut bodies: Vec<String> = ranked
        .notes
        .iter()
        .filter(|note| note.kind == TextKind::Rule)
        .map(|note| note.body.clone())
        .collect();
    for candidate in &ranked.candidates {
        for tag in &candidate.reasons {
            if is_rule_tag(tag) {
                let text = reason_text(tag, candidate, &ranked.budget);
                if text.kind == TextKind::Rule {
                    bodies.push(text.body);
                }
            }
        }
    }
    bodies
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Build the rules block from the engine trace's own explanations. Only rule and
/// simplification records are kept: a strategy record is a policy statement and
/// must not be presented as a rule here.
pub fn rules_section(explanations: &[Explanation]) -> ReviewSection {
    let mut lines = Vec::new();
    for explanation in explanations {
        let badge = match explanation.kind {
            ExplanationKind::Rule => "[RULE]",
            ExplanationKind::Simplification => "[SIMPLIFICATION]",
            // A strategy record is a policy statement: it never enters the rules
            // section.
            ExplanationKind::Strategy => continue,
        };
        lines.push(format!(
            "{badge} [{}] {}",
            explanation.topic, explanation.text
        ));
        for rule in &explanation.rules {
            if let Some(note) = coach::rule_note(rule) {
                lines.push(format!("    CR {rule}: {note}"));
            }
        }
    }
    if lines.is_empty() {
        lines.push(
            "no rule or simplification record applies to these offered actions right now"
                .to_string(),
        );
    }
    ReviewSection {
        title: RULES_TITLE,
        source: RULES_SOURCE,
        lines,
    }
}

/// The badge a payload sentence carries, derived from its kind alone (§4.8).
fn badge(kind: TextKind) -> &'static str {
    match kind {
        TextKind::Rule => "[RULE]",
        TextKind::Policy => "[POLICY]",
        TextKind::Uncertainty => "[UNCERTAINTY]",
        TextKind::Provenance => "[PROVENANCE]",
    }
}

fn scored_rows(ranked: &RankedReview) -> Vec<&RankedCandidate> {
    ranked
        .candidates
        .iter()
        .filter(|candidate| candidate.score.is_some())
        .collect()
}

fn unscored_rows(ranked: &RankedReview) -> Vec<&RankedCandidate> {
    ranked
        .candidates
        .iter()
        .filter(|candidate| candidate.score.is_none())
        .collect()
}

fn samples_phrase(count: usize) -> String {
    match count {
        1 => "1 scored sample".to_string(),
        n => format!("{n} scored samples"),
    }
}

fn policy_lines(ranked: &RankedReview) -> Vec<String> {
    let binding = &ranked.binding;
    let mut lines = vec![
        format!(
            "reviewed decision point: P{} · session generation {} · engine revision {} · decision {} · rules {}",
            binding.seat,
            binding.generation,
            binding.revision,
            binding.decision,
            binding.rules_revision
        ),
        format!(
            "masked view digest: {} (this review is bound to it — a different view is a different review)",
            binding.view_digest
        ),
        format!(
            "payload: {} — a read-only presentation of the ranking layer's own data",
            ranked.spec
        ),
    ];

    // Every payload sentence is placed by its kind. `Rule` sentences belong to
    // the rules block; the rest belong here.
    for note in &ranked.notes {
        if note.kind == TextKind::Rule {
            continue;
        }
        lines.push(format!("{} {}", badge(note.kind), note.body));
    }

    if let Some(reason) = &ranked.refusal {
        lines.push(format!("refused: {reason}"));
        lines.push(
            "no candidate line is shown: no rollout ran at this decision point, so there is no recorded line to replay"
                .to_string(),
        );
        lines.push(NO_VERDICT_NOTE.to_string());
        return lines;
    }

    // §5.2: the stopping rule is stated, so a partial sample is not a surprise.
    lines.push(
        "stopping rule: the analysis ends at whichever of the node budget, the per-rollout cap, the round schedule or the wall clock it reaches first, so a partial sample is normal"
            .to_string(),
    );

    let scored = scored_rows(ranked);
    let unscored = unscored_rows(ranked);
    let sole_offer = ranked
        .candidates
        .iter()
        .any(|candidate| candidate.reasons.contains(&ReasonTag::SoleLegalOffer));

    push_ordering_claim(&mut lines, ranked, &scored, sole_offer);

    // Ranked rows, with tied rows kept in one unordered group (§5.2). Rows are
    // numbered only when the payload actually claims an order at the top: a
    // single-sampled or tied leading row is listed, not ranked.
    let numbered = !sole_offer
        && matches!(
            scored.first().map(|row| &row.uncertainty),
            Some(Uncertainty::Ordered)
        );
    let mut index = 0usize;
    let mut position = 0usize;
    while index < scored.len() {
        let row = scored[index];
        match &row.uncertainty {
            Uncertainty::EffectivelyTied { with } => {
                let members: Vec<&RankedCandidate> = scored
                    .iter()
                    .copied()
                    .filter(|other| {
                        other.offer_index == row.offer_index || with.contains(&other.offer_index)
                    })
                    .collect();
                let last = members
                    .iter()
                    .filter_map(|member| {
                        scored
                            .iter()
                            .position(|other| other.offer_index == member.offer_index)
                    })
                    .max()
                    .unwrap_or(index);
                index = last + 1;
                lines.push(tie_heading(&members));
                for member in &members {
                    push_candidate(&mut lines, member, None, &ranked.budget);
                }
            }
            _ => {
                position += 1;
                let number = if numbered { Some(position) } else { None };
                push_candidate(&mut lines, row, number, &ranked.budget);
                index += 1;
            }
        }
    }

    if !unscored.is_empty() {
        lines.push(
            "unscored offers (no rollout reached a result; listed in engine offer order and not ranked):"
                .to_string(),
        );
        for candidate in &unscored {
            lines.push(format!(
                "— {} [offer {}] — no scored sample · {} rollout(s) attempted · {} without a result · {} refused by the search world",
                candidate.move_label,
                candidate.offer_index,
                candidate.rollouts_attempted,
                candidate.without_result,
                candidate.refused_times
            ));
            push_teaching_note(&mut lines, candidate, &ranked.budget);
            push_line(&mut lines, candidate);
        }
    }

    lines.push(NO_VERDICT_NOTE.to_string());
    lines
}

/// The ordering sentence, stated only as firmly as the payload allows (§5.2).
fn push_ordering_claim(
    lines: &mut Vec<String>,
    ranked: &RankedReview,
    scored: &[&RankedCandidate],
    sole_offer: bool,
) {
    if sole_offer {
        lines.push(
            "ordering: none — the engine offers exactly one action at this decision point, so there is nothing to order and no preference is claimed"
                .to_string(),
        );
        return;
    }
    let Some(first) = scored.first() else {
        let attempts: usize = ranked
            .candidates
            .iter()
            .map(|candidate| candidate.rollouts_attempted)
            .sum();
        lines.push(format!(
            "ordering: none — 0 of {} engine-offered candidate(s) reached a result in {} attempted rollout(s), so no alternative is ranked above another",
            ranked.candidates.len(),
            attempts
        ));
        return;
    };
    match &first.uncertainty {
        Uncertainty::EffectivelyTied { .. } => lines.push(
            "ordering: no order is claimed at the top — the leading offers are not separable at this sample count, so they are shown as one group"
                .to_string(),
        ),
        Uncertainty::SingleSample => lines.push(
            "ordering: no order is claimed — the leading offer has a single scored sample"
                .to_string(),
        ),
        _ => {
            lines.push(
                "ordering: highest mean practice outcome first; a group that is not separable at this sample count is shown as one group and is not ordered inside it"
                    .to_string(),
            );
            lines.push(
                "score: mean practice outcome = win +1 / loss −1 / draw 0 over scored determinizations; a rollout that was capped or ended without a result is counted as \"without result\", never as a win or a loss"
                    .to_string(),
            );
        }
    }
}

/// The heading of one unordered tie group (§5.2).
fn tie_heading(members: &[&RankedCandidate]) -> String {
    let mean = members
        .first()
        .and_then(|member| member.score)
        .unwrap_or_default();
    let samples: Vec<usize> = members.iter().map(|member| member.scored_samples).collect();
    if samples.windows(2).all(|pair| pair[0] == pair[1]) {
        format!(
            "effectively tied at {mean:+.2} ({} each) — not ordered inside this group:",
            samples_phrase(samples.first().copied().unwrap_or(0))
        )
    } else {
        format!(
            "effectively tied at {mean:+.2} (scored samples: {}) — not separable at this sample count, so not ordered inside this group:",
            samples
                .iter()
                .map(|count| count.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// One candidate row: the facts the payload holds, then its teaching note, then
/// its recorded line.
fn push_candidate(
    lines: &mut Vec<String>,
    candidate: &RankedCandidate,
    position: Option<usize>,
    budget: &BudgetRecord,
) {
    let number = match position {
        Some(position) => format!("{position}. "),
        // A group member: present, but deliberately not numbered, because the
        // payload does not order it against its group.
        None => "• ".to_string(),
    };
    let tied = matches!(candidate.uncertainty, Uncertainty::EffectivelyTied { .. });
    lines.push(format!(
        "{number}{} [offer {}] — {}",
        candidate.move_label,
        candidate.offer_index,
        row_facts(candidate, tied)
    ));
    push_teaching_note(lines, candidate, budget);
    push_line(lines, candidate);
}

fn row_facts(candidate: &RankedCandidate, tied: bool) -> String {
    let mut facts = match candidate.score {
        Some(mean) if candidate.scored_samples >= 2 => {
            format!(
                "mean {mean:+.2} over {}",
                samples_phrase(candidate.scored_samples)
            )
        }
        // §5.2: a one-sample row says why it is not ordered.
        Some(mean) => {
            format!("mean {mean:+.2} · one scored sample — not ordered against other rows")
        }
        None => "no scored sample".to_string(),
    };
    if candidate.chosen {
        facts.push_str(if tied {
            " · the search's own choice among equally scored offers"
        } else {
            " · the search's own choice"
        });
    }
    if candidate.without_result > 0 {
        facts.push_str(&format!(
            " · {} rollout(s) without a result",
            candidate.without_result
        ));
    }
    if candidate.refused_times > 0 {
        facts.push_str(&format!(
            " · refused {} time(s) by the search's own reconstruction",
            candidate.refused_times
        ));
    }
    facts
}

/// The practical reason for this alternative, from its reason tags, through the
/// ranking layer's single formatter and labelled as a heuristic rather than a
/// rule.
fn push_teaching_note(lines: &mut Vec<String>, candidate: &RankedCandidate, budget: &BudgetRecord) {
    let mut bodies = Vec::new();
    for tag in &candidate.reasons {
        let text = reason_text(tag, candidate, budget);
        // A rule-kind sentence is rule text: it belongs to the rules block and
        // never to the policy block (§5.1).
        if text.kind == TextKind::Rule {
            continue;
        }
        bodies.push(text.body);
    }
    if !bodies.is_empty() {
        lines.push(format!("    {TEACHING_NOTE_LABEL}: {}", bodies.join("; ")));
    }
}

/// The candidate line: a recorded rollout, legible step by step, with its scope
/// stated. Never synthesised (§4.4).
fn push_line(lines: &mut Vec<String>, candidate: &RankedCandidate) {
    match &candidate.line {
        Some(line) if !line.steps.is_empty() => {
            lines.push(format!("    line: {}", line_steps(line)));
            if line.truncated {
                lines.push(format!(
                    "    line: recorded as a prefix of {} step(s) — the rollout continued past the recording cap, so this is not the whole game",
                    line.steps.len()
                ));
            }
            lines.push(format!(
                "    line scope: {} · determinization seed {} · recorded outcome {}",
                scope_sentence(line.scope),
                line.seed_index,
                outcome_sentence(line.outcome)
            ));
        }
        Some(_) | None => {
            let why = if candidate.score.is_none() {
                LINE_NONE_NO_RESULT
            } else {
                LINE_NONE_UNRECORDED
            };
            lines.push(format!("    line: none available — {why}"));
        }
    }
}

/// `decision 0 · P0 · "Play Forest" → decision 1 · P1 · "Pass priority"`.
fn line_steps(line: &CandidateLine) -> String {
    line.steps
        .iter()
        .map(|step| {
            format!(
                "decision {} · P{} · \"{}\"",
                step.decision, step.seat, step.label
            )
        })
        .collect::<Vec<_>>()
        .join(" → ")
}

fn scope_sentence(scope: LineScope) -> &'static str {
    match scope {
        LineScope::SearchGame => {
            "this line was played in the search's own reconstructed game under the disclosed determinization (search world only — not the real game's hidden cards)"
        }
    }
}

fn outcome_sentence(outcome: i8) -> String {
    match outcome {
        1 => "a win (+1) in the search world".to_string(),
        0 => "a draw (0) in the search world".to_string(),
        -1 => "a loss (−1) in the search world".to_string(),
        other => format!("an unexpected recorded value ({other}) — the payload is inconsistent"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aperture_ai::StopReason;
    use aperture_ai::review::{EvidenceRef, LineStep, Preference, ReviewText};
    use aperture_core::coach::ExplanationKind;

    fn budget() -> BudgetRecord {
        BudgetRecord {
            seeds: 2,
            node_budget: 900,
            max_rollout_nodes: 150,
            max_millis: 400,
            max_candidates: 4,
            decisions_used: 480,
            stop: StopReason::DeterminizationsExhausted,
            cancelled: false,
            reproducible: true,
            fidelity:
                "opponents: neutral belief deck (1 opponent(s)); no real hand or library order read"
                    .to_string(),
        }
    }

    fn binding() -> ReviewBinding {
        ReviewBinding {
            seat: 0,
            generation: 4,
            revision: 9,
            decision: 12,
            view_digest: "digest-of-the-masked-view".to_string(),
            rules_revision: "2026-09-25".to_string(),
        }
    }

    fn provenance_notes() -> Vec<ReviewText> {
        vec![
            ReviewText {
                kind: TextKind::Provenance,
                body: budget().rendered(),
                evidence: vec![EvidenceRef::Budget {
                    field: "decisions_used".into(),
                }],
            },
            ReviewText {
                kind: TextKind::Provenance,
                body: format!("determinization fidelity (verbatim): {}", budget().fidelity),
                evidence: vec![EvidenceRef::Budget {
                    field: "fidelity".into(),
                }],
            },
        ]
    }

    fn note(kind: TextKind, body: &str) -> ReviewText {
        ReviewText {
            kind,
            body: body.to_string(),
            evidence: vec![EvidenceRef::EngineOffer { index: 0 }],
        }
    }

    fn line(
        steps: &[(usize, PlayerId, &str)],
        seed: u64,
        outcome: i8,
        truncated: bool,
    ) -> CandidateLine {
        CandidateLine {
            steps: steps
                .iter()
                .map(|(decision, seat, label)| LineStep {
                    decision: *decision,
                    seat: *seat,
                    label: (*label).to_string(),
                })
                .collect(),
            truncated,
            seed_index: seed,
            outcome,
            scope: LineScope::SearchGame,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn candidate(
        offer_index: usize,
        label: &str,
        score: Option<f32>,
        samples: usize,
        uncertainty: Uncertainty,
        chosen: bool,
        reasons: Vec<ReasonTag>,
        line: Option<CandidateLine>,
    ) -> RankedCandidate {
        RankedCandidate {
            offer_index,
            move_label: label.to_string(),
            offer_rules: Vec::new(),
            score,
            scored_samples: samples,
            without_result: 0,
            refused_times: 0,
            rollouts_attempted: samples,
            line,
            reasons,
            uncertainty,
            chosen,
        }
    }

    fn payload(
        candidates: Vec<RankedCandidate>,
        preference: Option<Preference>,
        notes: Vec<ReviewText>,
    ) -> RankedReview {
        RankedReview {
            spec: "move-review/1.2".into(),
            binding: binding(),
            budget: budget(),
            candidates,
            preference,
            refusal: None,
            notes,
        }
    }

    fn refused_payload(reason: &str, offered: usize) -> RankedReview {
        let mut notes = provenance_notes();
        notes.push(note(
            TextKind::Uncertainty,
            &format!(
                "no alternative is ranked: the search did not run for this decision point ({reason}). The engine still offered {offered} action(s) at it."
            ),
        ));
        RankedReview {
            spec: "move-review/1.2".into(),
            binding: binding(),
            budget: BudgetRecord {
                stop: StopReason::NotStarted,
                reproducible: false,
                decisions_used: 0,
                fidelity: "not applicable: the search refused this decision point before any determinization was drawn".to_string(),
                ..budget()
            },
            candidates: Vec::new(),
            preference: None,
            refusal: Some(reason.to_string()),
            notes,
        }
    }

    fn rendered(review: &MoveReview) -> String {
        review.policy_section().lines.join("\n")
    }

    // ---------------------------------------------------------------- (a) lines

    #[test]
    fn a_candidate_line_renders_from_the_recorded_trace_step_by_step() {
        // The three steps are exactly the ones the search recorded for a rollout
        // it played; the renderer may not add, drop or reorder one.
        let review = MoveReview::from_ranked(payload(
            vec![candidate(
                0,
                "Play Forest",
                Some(1.0),
                2,
                Uncertainty::Ordered,
                true,
                vec![ReasonTag::HighestPracticeOutcome],
                Some(line(
                    &[
                        (0, 0, "Play Forest"),
                        (1, 1, "Pass priority"),
                        (2, 0, "Cast Shock"),
                    ],
                    3,
                    1,
                    true,
                )),
            )],
            Some(Preference::Unique { offer_index: 0 }),
            provenance_notes(),
        ));
        let shown = rendered(&review);
        let first = shown
            .find("decision 0 · P0 · \"Play Forest\"")
            .unwrap_or_else(|| panic!("the root step must be rendered: {shown}"));
        let second = shown
            .find("decision 1 · P1 · \"Pass priority\"")
            .unwrap_or_else(|| panic!("the second step must be rendered: {shown}"));
        let third = shown
            .find("decision 2 · P0 · \"Cast Shock\"")
            .unwrap_or_else(|| panic!("the third step must be rendered: {shown}"));
        assert!(
            first < second && second < third,
            "the rendered line must keep the recorded order: {shown}"
        );
        assert!(
            shown.contains("search world only"),
            "the line must state its scope: {shown}"
        );
        assert!(
            shown.contains("determinization seed 3") && shown.contains("a win (+1)"),
            "the line's seed and recorded outcome must be shown: {shown}"
        );
        assert!(
            shown.contains("prefix of 3 step(s)"),
            "a truncated recording must say it is a prefix: {shown}"
        );
    }

    #[test]
    fn a_candidate_without_a_line_says_why_and_never_invents_one() {
        let recorded = MoveReview::from_ranked(payload(
            vec![candidate(
                0,
                "Play Forest",
                Some(1.0),
                2,
                Uncertainty::Ordered,
                true,
                Vec::new(),
                None,
            )],
            Some(Preference::Unique { offer_index: 0 }),
            provenance_notes(),
        ));
        let shown = rendered(&recorded);
        assert!(
            shown.contains("line: none available — the search recorded no line for this offer"),
            "a scored row with no recording must say so: {shown}"
        );
        assert!(
            !shown.contains("line: decision "),
            "no step may be printed for a row with no recorded line: {shown}"
        );

        let unscored = MoveReview::from_ranked(payload(
            vec![candidate(
                0,
                "Cast Opt",
                None,
                0,
                Uncertainty::Unscored,
                false,
                vec![ReasonTag::NoScoredSample],
                None,
            )],
            None,
            provenance_notes(),
        ));
        let shown = rendered(&unscored);
        assert!(
            shown.contains(
                "no rollout reached a result for this offer, so there was no line to record"
            ),
            "an unscored row must state the different reason: {shown}"
        );
    }

    // ---------------------------------------------------------- (b) uncertainty

    #[test]
    fn tied_rows_render_as_one_group_that_is_not_ordered_inside() {
        let review = MoveReview::from_ranked(payload(
            vec![
                candidate(
                    0,
                    "Play Forest",
                    Some(1.0),
                    2,
                    Uncertainty::EffectivelyTied { with: vec![1] },
                    true,
                    vec![ReasonTag::EqualPracticeOutcome],
                    None,
                ),
                candidate(
                    1,
                    "Pass priority",
                    Some(1.0),
                    2,
                    Uncertainty::EffectivelyTied { with: vec![0] },
                    false,
                    vec![ReasonTag::EqualPracticeOutcome],
                    None,
                ),
            ],
            Some(Preference::AmongEquals {
                chosen: 0,
                offer_indices: vec![0, 1],
            }),
            provenance_notes(),
        ));
        let shown = rendered(&review);
        assert!(
            shown.contains(
                "effectively tied at +1.00 (2 scored samples each) — not ordered inside this group:"
            ),
            "a tie group must be announced as one: {shown}"
        );
        assert!(
            shown.contains("ordering: no order is claimed at the top"),
            "a tie at the top must not claim an order: {shown}"
        );
        // Neither member is numbered: the payload does not order them.
        assert!(
            !shown.contains("1. Play Forest") && !shown.contains("2. Pass priority"),
            "tied rows must not be numbered: {shown}"
        );
        for label in ["• Play Forest [offer 0]", "• Pass priority [offer 1]"] {
            assert!(shown.contains(label), "{label} missing: {shown}");
        }
        assert!(
            shown.contains("the search's own choice among equally scored offers"),
            "only a real tie may carry the equal-score qualifier: {shown}"
        );
        assert_eq!(
            shown.matches("mean +1.00 over 2 scored samples").count(),
            2,
            "both tied rows must print mean and samples: {shown}"
        );
    }

    #[test]
    fn a_single_sample_row_says_it_is_not_ordered() {
        let review = MoveReview::from_ranked(payload(
            vec![
                candidate(
                    0,
                    "Play Forest",
                    Some(1.0),
                    1,
                    Uncertainty::SingleSample,
                    true,
                    vec![ReasonTag::LowSample],
                    None,
                ),
                candidate(
                    1,
                    "Pass priority",
                    Some(-1.0),
                    2,
                    Uncertainty::Ordered,
                    false,
                    Vec::new(),
                    None,
                ),
            ],
            Some(Preference::NoOrder { chosen: Some(0) }),
            provenance_notes(),
        ));
        let shown = rendered(&review);
        assert!(
            shown.contains("mean +1.00 · one scored sample — not ordered against other rows"),
            "a one-sample row must say it is not ordered: {shown}"
        );
        assert!(
            shown.contains(
                "ordering: no order is claimed — the leading offer has a single scored sample"
            ),
            "the header must not claim an order over a single sample: {shown}"
        );
        assert!(
            !shown.contains("1. Play Forest"),
            "a row that is not ordered must not carry a ranking number: {shown}"
        );
        assert!(
            !shown.contains("the search's own choice among equally scored"),
            "a single sample is not a tie: {shown}"
        );
    }

    #[test]
    fn unscored_offers_are_listed_separately_and_not_ranked() {
        let review = MoveReview::from_ranked(payload(
            vec![
                candidate(
                    0,
                    "Play Forest",
                    Some(0.5),
                    2,
                    Uncertainty::Ordered,
                    true,
                    Vec::new(),
                    None,
                ),
                candidate(
                    1,
                    "Cast Opt",
                    None,
                    0,
                    Uncertainty::Unscored,
                    false,
                    vec![ReasonTag::NoScoredSample],
                    None,
                ),
            ],
            Some(Preference::Unique { offer_index: 0 }),
            provenance_notes(),
        ));
        let shown = rendered(&review);
        assert!(
            shown.contains(
                "unscored offers (no rollout reached a result; listed in engine offer order and not ranked):"
            ),
            "the unranked list must be titled: {shown}"
        );
        assert!(
            shown.contains("— Cast Opt [offer 1] — no scored sample"),
            "an unscored offer must appear in its own list: {shown}"
        );
        assert!(
            shown.contains("teaching note (heuristic from the search's reason tags"),
            "an unscored row's reason must be labelled a heuristic: {shown}"
        );
        assert!(
            !shown.contains("2. Cast Opt"),
            "an unscored offer must not take a ranking position: {shown}"
        );
    }

    #[test]
    fn zero_scored_samples_rank_nothing_and_count_the_attempts() {
        let mut first = candidate(
            0,
            "Cast Opt",
            None,
            0,
            Uncertainty::Unscored,
            false,
            vec![ReasonTag::NoScoredSample],
            None,
        );
        first.without_result = 2;
        first.rollouts_attempted = 2;
        let mut second = candidate(
            1,
            "Pass",
            None,
            0,
            Uncertainty::Unscored,
            false,
            vec![ReasonTag::NoScoredSample],
            None,
        );
        second.without_result = 3;
        second.rollouts_attempted = 3;
        let review =
            MoveReview::from_ranked(payload(vec![first, second], None, provenance_notes()));
        let shown = rendered(&review);
        assert!(
            shown.contains(
                "ordering: none — 0 of 2 engine-offered candidate(s) reached a result in 5 attempted rollout(s)"
            ),
            "a zero-sample analysis must state the attempt count: {shown}"
        );
        assert!(shown.contains("no scored sample"), "{shown}");
        assert!(
            !shown.contains("mean "),
            "no placeholder mean may be printed: {shown}"
        );
    }

    // ------------------------------------------------------------ (c) sections

    #[test]
    fn rules_and_policy_blocks_are_structurally_separate() {
        let explanations = vec![
            Explanation {
                topic: "priority".to_string(),
                kind: ExplanationKind::Rule,
                text: "you may act during your main phase with an empty stack".to_string(),
                rules: vec!["117.1a"],
            },
            Explanation {
                topic: "strategy".to_string(),
                kind: ExplanationKind::Strategy,
                text: "a strategy record is a policy statement".to_string(),
                rules: Vec::new(),
            },
        ];
        let mut notes = provenance_notes();
        notes.push(note(
            TextKind::Policy,
            "the search's own choice is offer 0 (\"Play Forest\")",
        ));
        let review = MoveReview::from_ranked(payload(
            vec![candidate(
                0,
                "Play Forest",
                Some(1.0),
                2,
                Uncertainty::Ordered,
                true,
                Vec::new(),
                None,
            )],
            Some(Preference::Unique { offer_index: 0 }),
            notes,
        ));
        let sections = review.sections(&explanations);

        assert_eq!(sections.rules.title, RULES_TITLE);
        assert_eq!(sections.rules.source, RULES_SOURCE);
        assert_eq!(sections.policy.title, POLICY_TITLE);
        assert_eq!(sections.policy.source, POLICY_SOURCE);
        assert_ne!(sections.rules.title, sections.policy.title);
        assert_ne!(sections.rules.source, sections.policy.source);

        let rules_text = sections.rules.lines.join("\n");
        assert!(
            rules_text.contains("[RULE] [priority]") && rules_text.contains("CR 117.1a"),
            "the rules block must carry the engine's own rule text: {rules_text}"
        );
        assert!(
            !rules_text.contains("a strategy record is a policy statement"),
            "a strategy record must never reach the rules block: {rules_text}"
        );
        assert!(
            !rules_text.contains("mean ") && !rules_text.contains("[POLICY]"),
            "the rules block must carry no preference: {rules_text}"
        );

        let policy_text = sections.policy.lines.join("\n");
        assert!(
            !policy_text.contains("[RULE]")
                && !policy_text.contains("CR ")
                && !policy_text.contains("empty stack"),
            "the policy block must carry no rule text: {policy_text}"
        );
        assert!(
            policy_text.contains("[POLICY]") && policy_text.contains("[PROVENANCE]"),
            "payload sentences must carry their own kind badges: {policy_text}"
        );
    }

    #[test]
    fn a_payload_rule_sentence_is_placed_in_the_rules_block_and_nowhere_else() {
        let mut notes = provenance_notes();
        notes.push(note(
            TextKind::Rule,
            "[RULE] priority (CR 117.1a): the engine's own rule sentence",
        ));
        let review = MoveReview::from_ranked(payload(
            vec![candidate(
                0,
                "Play Forest",
                Some(1.0),
                2,
                Uncertainty::Ordered,
                true,
                vec![ReasonTag::EngineRuleApplies {
                    rule: "117.1a".into(),
                    topic: "priority".to_string(),
                }],
                None,
            )],
            Some(Preference::Unique { offer_index: 0 }),
            notes,
        ));
        let sections = review.sections(&[]);
        let rules_text = sections.rules.lines.join("\n");
        let policy_text = sections.policy.lines.join("\n");
        assert!(
            rules_text.contains("the engine's own rule sentence"),
            "a rule-kind payload sentence belongs to the rules block: {rules_text}"
        );
        assert!(
            !policy_text.contains("the engine's own rule sentence")
                && !policy_text.contains("CR 117.1a"),
            "a rule-kind payload sentence must never reach the policy block: {policy_text}"
        );
    }

    // --------------------------------------------------- refusal and sole offer

    #[test]
    fn a_refusal_ranks_nothing_and_keeps_the_reason_verbatim() {
        let reason = "non-priority decision (pending kind mulligan); search not enabled there";
        let review = MoveReview::from_ranked(refused_payload(reason, 3));
        let shown = rendered(&review);
        assert!(
            shown.contains(reason),
            "the typed refusal reason must appear verbatim: {shown}"
        );
        assert!(
            shown.contains("no alternative is ranked"),
            "the panel must say plainly that nothing is ranked: {shown}"
        );
        assert!(
            shown.contains("no candidate line is shown"),
            "a refusal must state that no line exists: {shown}"
        );
        assert!(
            !shown.contains("mean ") && !shown.contains("1. "),
            "a refusal must print no ordering: {shown}"
        );
        assert!(
            review.ranked().candidates.is_empty(),
            "a refusal ranks nothing"
        );
    }

    #[test]
    fn a_sole_legal_offer_is_not_a_preference_and_is_not_ordered() {
        let review = MoveReview::from_ranked(payload(
            vec![candidate(
                0,
                "Pass priority",
                Some(1.0),
                2,
                Uncertainty::Ordered,
                true,
                vec![ReasonTag::SoleLegalOffer],
                None,
            )],
            Some(Preference::Unique { offer_index: 0 }),
            provenance_notes(),
        ));
        let shown = rendered(&review);
        assert!(
            shown.contains(
                "ordering: none — the engine offers exactly one action at this decision point"
            ),
            "a sole offer must claim no ordering: {shown}"
        );
        assert!(
            !shown.contains("1. Pass priority"),
            "a single offer must not be numbered as a ranking: {shown}"
        );
        assert!(
            !shown.contains("highest mean practice outcome first"),
            "a sole offer must not carry ordering language: {shown}"
        );
        assert!(
            shown.contains("the engine offers exactly one action at this decision point"),
            "the payload's own sole-offer reason must be shown: {shown}"
        );
    }

    // ---------------------------------------------------------------- (d) honesty

    /// Fragments rather than whole banned phrases: the shared guardrail suite
    /// scans this file's string literals, and a bare banned phrase in a test
    /// would be a violation in its own right.
    const FORBIDDEN_FRAGMENTS: &[&str] = &[
        "win prob",
        "winrat",
        "chance of",
        "confiden",
        "blund",
        "mistak",
        "best mo",
        "inaccura",
    ];

    #[test]
    fn no_rendered_line_carries_a_percent_figure_or_a_certainty_claim() {
        let mut reviews = Vec::new();
        reviews.push(MoveReview::from_ranked(payload(
            vec![
                candidate(
                    0,
                    "Play Forest",
                    Some(1.0),
                    2,
                    Uncertainty::Ordered,
                    true,
                    vec![
                        ReasonTag::HighestPracticeOutcome,
                        ReasonTag::IncompleteRollouts,
                        ReasonTag::RefusedBySearchWorld,
                        ReasonTag::BudgetStopped,
                    ],
                    Some(line(
                        &[(0, 0, "Play Forest"), (1, 1, "Pass priority")],
                        0,
                        -1,
                        false,
                    )),
                ),
                candidate(
                    1,
                    "Pass priority",
                    Some(-0.5),
                    2,
                    Uncertainty::Ordered,
                    false,
                    vec![ReasonTag::IncompleteRollouts],
                    None,
                ),
            ],
            Some(Preference::Unique { offer_index: 0 }),
            provenance_notes(),
        )));
        reviews.push(MoveReview::from_ranked(refused_payload(
            "non-priority decision (pending kind mulligan); search not enabled there",
            3,
        )));
        for review in reviews {
            for line in &review.policy_section().lines {
                assert!(
                    !line.contains('%'),
                    "a rendered line may not carry a percent figure: {line:?}"
                );
                let lower = line.to_lowercase();
                for fragment in FORBIDDEN_FRAGMENTS {
                    assert!(
                        !lower.contains(fragment),
                        "a rendered line may not carry {fragment:?}: {line:?}"
                    );
                }
            }
        }
    }

    // ---------------------------------------------------------------- provenance

    #[test]
    fn every_binding_field_can_condemn_a_review() {
        let review = MoveReview::from_ranked(refused_payload("a decision point", 1));
        let mut live = binding();
        assert_eq!(
            review.discard_reason(&live),
            None,
            "a matching binding is current"
        );

        live.generation += 1;
        assert_eq!(review.discard_reason(&live), Some("stale_generation"));
        live = binding();
        live.revision += 1;
        assert_eq!(review.discard_reason(&live), Some("stale_revision"));
        live = binding();
        live.decision += 1;
        assert_eq!(review.discard_reason(&live), Some("stale_decision"));
        live = binding();
        live.seat = 1;
        assert_eq!(review.discard_reason(&live), Some("wrong_seat"));
        // The digest catches a different game reproducing the same
        // seat/revision/decision triple.
        live = binding();
        live.view_digest = "another-games-masked-view".to_string();
        assert_eq!(review.discard_reason(&live), Some("stale_view_digest"));
    }

    #[test]
    fn rendering_a_review_twice_changes_nothing() {
        let payload = payload(
            vec![candidate(
                0,
                "Play Forest",
                Some(0.5),
                2,
                Uncertainty::Ordered,
                true,
                vec![ReasonTag::HighestPracticeOutcome],
                Some(line(&[(0, 0, "Play Forest")], 1, 0, false)),
            )],
            Some(Preference::Unique { offer_index: 0 }),
            provenance_notes(),
        );
        let before = payload.clone();
        let first = MoveReview::from_ranked(payload);
        let second = first.clone();
        assert_eq!(first.policy_section(), second.policy_section());
        assert_eq!(
            first.ranked(),
            &before,
            "rendering must not modify the payload it presents"
        );
        assert_eq!(first.summary_line(), second.summary_line());
    }

    #[test]
    fn the_panel_prints_the_budget_the_search_recorded() {
        let review = MoveReview::from_ranked(payload(
            vec![candidate(
                0,
                "Play Forest",
                Some(1.0),
                2,
                Uncertainty::Ordered,
                true,
                Vec::new(),
                None,
            )],
            Some(Preference::Unique { offer_index: 0 }),
            provenance_notes(),
        ));
        let shown = rendered(&review);
        assert!(
            shown.contains(&budget().rendered()),
            "the payload's own budget line must be printed verbatim: {shown}"
        );
        assert!(
            shown.contains("no real hand or library order read"),
            "the search's fidelity disclosure must be printed: {shown}"
        );
        assert!(
            shown.contains("generation 4")
                && shown.contains("engine revision 9")
                && shown.contains("decision 12")
                && shown.contains("digest-of-the-masked-view"),
            "the panel must name the binding it is showing: {shown}"
        );
    }

    /// The panel's full rendering, pinned. A snapshot is the only check that
    /// notices a reworded caveat, a dropped sample count or a re-ordered tie
    /// group, and the expected text below is what a reviewer reads on screen.
    #[test]
    fn the_rendered_panel_for_a_tied_and_partly_unscored_analysis_is_pinned() {
        let review = MoveReview::from_ranked(payload(
            vec![
                candidate(
                    0,
                    "Play Forest",
                    Some(1.0),
                    2,
                    Uncertainty::EffectivelyTied { with: vec![1] },
                    true,
                    vec![ReasonTag::EqualPracticeOutcome],
                    Some(line(
                        &[(0, 0, "Play Forest"), (1, 1, "Pass priority")],
                        0,
                        1,
                        false,
                    )),
                ),
                candidate(
                    1,
                    "Pass priority",
                    Some(1.0),
                    2,
                    Uncertainty::EffectivelyTied { with: vec![0] },
                    false,
                    vec![ReasonTag::EqualPracticeOutcome],
                    None,
                ),
                candidate(
                    2,
                    "Cast Shock",
                    None,
                    0,
                    Uncertainty::Unscored,
                    false,
                    vec![ReasonTag::NoScoredSample],
                    None,
                ),
            ],
            Some(Preference::AmongEquals {
                chosen: 0,
                offer_indices: vec![0, 1],
            }),
            provenance_notes(),
        ));
        let expected = r#"reviewed decision point: P0 · session generation 4 · engine revision 9 · decision 12 · rules 2026-09-25
masked view digest: digest-of-the-masked-view (this review is bound to it — a different view is a different review)
payload: move-review/1.2 — a read-only presentation of the ranking layer's own data
[PROVENANCE] budget: 2 determinization seed(s) · 900 engine decisions total · 150 decisions per rollout · 400 ms wall clock · at most 4 engine-offered candidates · used 480 · stopped: determinizations exhausted · reproducible: yes
[PROVENANCE] determinization fidelity (verbatim): opponents: neutral belief deck (1 opponent(s)); no real hand or library order read
stopping rule: the analysis ends at whichever of the node budget, the per-rollout cap, the round schedule or the wall clock it reaches first, so a partial sample is normal
ordering: no order is claimed at the top — the leading offers are not separable at this sample count, so they are shown as one group
effectively tied at +1.00 (2 scored samples each) — not ordered inside this group:
• Play Forest [offer 0] — mean +1.00 over 2 scored samples · the search's own choice among equally scored offers
    teaching note (heuristic from the search's reason tags — not a rule statement): level with another offered alternative at this sample count (+1.00)
    line: decision 0 · P0 · "Play Forest" → decision 1 · P1 · "Pass priority"
    line scope: this line was played in the search's own reconstructed game under the disclosed determinization (search world only — not the real game's hidden cards) · determinization seed 0 · recorded outcome a win (+1) in the search world
• Pass priority [offer 1] — mean +1.00 over 2 scored samples
    teaching note (heuristic from the search's reason tags — not a rule statement): level with another offered alternative at this sample count (+1.00)
    line: none available — the search recorded no line for this offer, and this panel does not synthesise one
unscored offers (no rollout reached a result; listed in engine offer order and not ranked):
— Cast Shock [offer 2] — no scored sample · 0 rollout(s) attempted · 0 without a result · 0 refused by the search world
    teaching note (heuristic from the search's reason tags — not a rule statement): no scored sample: no rollout reached a result, so this alternative is not ranked
    line: none available — no rollout reached a result for this offer, so there was no line to record
this panel reports the engine's own counts and the search's own preference only: it passes no verdict on your play, and it labels no alternative as an error"#;
        assert_eq!(rendered(&review), expected);
    }
}
