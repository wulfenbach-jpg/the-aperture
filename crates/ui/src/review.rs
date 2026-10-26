//! Move review: ranked engine-offered alternatives at one live decision point,
//! with the analysis' own counts and its uncertainty stated rather than implied.
//!
//! This module is a *presentation* layer over [`aperture_ai::search`]. The
//! search itself runs on a background worker (`crate::session`); nothing here
//! computes a rule, submits an action, or reads the engine.
//!
//! Four properties are structural, not conventions:
//!
//! 1. **Offered actions only.** A row is built from a
//!    [`aperture_ai::CandidateScore`], and every candidate score the search
//!    produces came from `view.legal`. [`MoveReview`] stores the engine's own
//!    label, never an [`aperture_core::Action`], so this type cannot submit a
//!    decision even by accident.
//! 2. **No false certainty.** The score is the mean *practice outcome* over
//!    determinizations (win +1, loss −1, draw 0; a capped or failed rollout is
//!    counted separately and is neither). It is never a win probability, never
//!    a percentage, and a ranking over a handful of samples is labelled with the
//!    sample count that produced it. No output line says a move was "best", a
//!    "blunder" or a "mistake", and nothing here claims the engine is stronger.
//! 3. **Rules stay apart from preference.** Rules text is produced only by
//!    [`rules_section`], which accepts the engine trace's own
//!    [`aperture_core::coach::Explanation`] records and keeps only the rule and
//!    simplification kinds. Policy ordering lives only in
//!    [`MoveReview::policy_section`]. The two have different constructors,
//!    different titles and different source labels; there is no path that writes
//!    rule text into the policy section or a preference into the rules section.
//! 4. **Refusal is a first-class outcome.** When the search refuses a decision
//!    point it returns a typed [`aperture_ai::SearchUnavailable`]; the panel then
//!    carries that reason verbatim and ranks nothing at all.

use aperture_ai::DecisionSource;
use aperture_ai::{SearchConfig, SearchOutcome};
use aperture_core::PlayerId;
use aperture_core::action::LegalAction;
use aperture_core::coach::{self, Explanation, ExplanationKind};

/// The label shown when the reviewing viewer holds no decision to review. This
/// is the one refusal this module states itself; every other refusal is the
/// search's own typed reason, carried verbatim.
pub const NOT_YOUR_DECISION: &str = "the reviewing seat holds no engine offer right now (it is not that seat's decision point); there is nothing to rank";

/// Title of the policy-preference block. Deliberately not a verdict.
pub const POLICY_TITLE: &str =
    "MOVE REVIEW — ranked practice outcomes (policy preference, not a rule)";
/// Source label of the policy-preference block.
pub const POLICY_SOURCE: &str = "source: aperture_ai::search over engine-offered actions · practice outcomes, not a chance of winning";
/// Title of the rules block.
pub const RULES_TITLE: &str = "RULES FOR THESE OFFERED ACTIONS — engine trace";
/// Source label of the rules block.
pub const RULES_SOURCE: &str =
    "source: engine rule/trace explanations and the pinned CR text · never a policy preference";

/// The exact compute budget a move review uses. Pinned so a report is
/// reproducible: same view, same seeds, same budget, same ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewBudget {
    pub seeds: usize,
    pub node_budget: usize,
    pub max_rollout_nodes: usize,
    pub max_millis: u64,
    pub max_candidates: usize,
}

/// The default review budget: small enough to finish on a background worker
/// while a player waits, large enough to score several offers.
pub const REVIEW_BUDGET: ReviewBudget = ReviewBudget {
    // Two determinizations, so every candidate gets two independent samples
    // before any candidate gets a second one (the search round-robins).
    seeds: 2,
    // Measured on the bundled tables: one rollout of a four-seat practice game
    // needs several thousand engine decisions before it reaches a result, so a
    // smaller per-rollout cap would only ever produce "without result" rows.
    node_budget: 30_000,
    max_rollout_nodes: 6_000,
    max_millis: 8_000,
    max_candidates: 6,
};

impl ReviewBudget {
    /// The search configuration this budget describes.
    pub fn config(self) -> SearchConfig {
        SearchConfig {
            seeds: self.seeds,
            node_budget: self.node_budget,
            max_rollout_nodes: self.max_rollout_nodes,
            max_millis: self.max_millis,
            max_candidates: self.max_candidates,
        }
    }

    /// The disclosed budget line, so the reader can see what the numbers cost.
    pub fn line(&self) -> String {
        format!(
            "budget: {} determinization seed(s) · {} engine decisions total · {} decisions per rollout · {} ms wall clock · at most {} engine-offered candidates",
            self.seeds,
            self.node_budget,
            self.max_rollout_nodes,
            self.max_millis,
            self.max_candidates
        )
    }
}

/// One engine-offered alternative and the analysis' own counts for it.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateRow {
    /// The engine's own label for the offered action.
    pub label: String,
    /// Mean practice outcome over scored rollouts, or `None` when no rollout
    /// reached an engine result.
    pub mean: Option<f32>,
    pub scored_samples: usize,
    /// Rollouts that ended without a result (cap/truncation/failure) — neither a
    /// win nor a loss.
    pub without_result: usize,
    /// How often the search world refused this root action as illegal.
    pub refused_times: usize,
    /// True for the candidate the search itself selected. Used only to break
    /// ties in the display order so the panel never contradicts the search's
    /// own choice; it is not a verdict on the player's play.
    pub chosen: bool,
}

/// What the search would choose, labelled for what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preference {
    /// A candidate the search actually scored highest. A policy preference.
    Search(String),
    /// Nothing scored: the search returned the deterministic legal fallback.
    /// This is a fallback, not a preference.
    Fallback(String),
    /// A refused review: the search produced no choice at all.
    None,
}

/// One finished move review, bound to the decision it was computed for.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveReview {
    /// The seat whose decision was reviewed.
    pub seat: PlayerId,
    /// Session generation, engine revision and decision serial the review was
    /// computed against; a panel whose binding no longer matches is stale.
    pub generation: u64,
    pub revision: u64,
    pub decision_serial: u64,
    pub budget: ReviewBudget,
    /// Engine decision points the search actually consumed.
    pub decisions_used: usize,
    /// The search's own determinization-fidelity disclosure, verbatim.
    pub fidelity: String,
    /// True when the search stopped early (cancellation/wall clock).
    pub cancelled: bool,
    /// Candidates in the engine's offer order. Empty exactly when `refusal` is
    /// `Some`; a refusal never carries a ranking.
    pub candidates: Vec<CandidateRow>,
    /// The search's choice, with its provenance.
    pub preference: Preference,
    /// The typed search refusal, verbatim. `Some` means no ranking at all.
    pub refusal: Option<String>,
}

/// The standing disclosure that no candidate line is available. The search
/// returns root candidate scores only, so a line is never synthesised here.
const LINES_NOTE: &str = "lines: the search exposes no principal variation — it returns root candidate scores only, not a played-out line — so no candidate line is shown here rather than synthesising one";

/// Every panel line denying a verdict. Kept in one place so the refusal, the
/// zero-sample panel and the ranked panel all state the same thing.
const NO_VERDICT_NOTE: &str = "this panel reports the engine's own counts and the search's own preference only: it passes no verdict on your play, and it labels no alternative as an error";

impl MoveReview {
    /// A review that ranks nothing, carrying a typed refusal reason.
    pub fn refused(
        seat: PlayerId,
        generation: u64,
        revision: u64,
        decision_serial: u64,
        budget: ReviewBudget,
        reason: String,
    ) -> Self {
        MoveReview {
            seat,
            generation,
            revision,
            decision_serial,
            budget,
            decisions_used: 0,
            fidelity: "no analysis ran for this decision point".to_string(),
            cancelled: false,
            candidates: Vec::new(),
            preference: Preference::None,
            refusal: Some(reason),
        }
    }

    /// A review built from a completed search and the masked view it ran on.
    pub fn from_outcome(
        seat: PlayerId,
        generation: u64,
        revision: u64,
        decision_serial: u64,
        budget: ReviewBudget,
        outcome: &SearchOutcome,
        offers: &[LegalAction],
    ) -> Self {
        let analysis = &outcome.analysis;
        let candidates = analysis
            .candidates
            .iter()
            .map(|score| CandidateRow {
                label: score.label.clone(),
                mean: score.mean,
                scored_samples: score.scored_samples,
                without_result: score.rollouts_without_result,
                refused_times: score.refused_times,
                chosen: score.action == outcome.decision.action,
            })
            .collect::<Vec<_>>();
        let label_of = |action: &aperture_core::Action| {
            offers
                .iter()
                .find(|legal| legal.action == *action)
                .map(|legal| legal.label.clone())
        };
        let preference = match (outcome.decision.source, label_of(&outcome.decision.action)) {
            (DecisionSource::Search, Some(label)) => Preference::Search(label),
            (_, Some(label)) => Preference::Fallback(label),
            (DecisionSource::Search, None) => {
                // Cannot happen for a search decision (its candidates come from
                // `view.legal`), but a missing label is disclosed rather than
                // invented.
                Preference::Search("an engine offer the search did not label".to_string())
            }
            (_, None) => {
                Preference::Fallback("an engine offer the heuristic did not label".to_string())
            }
        };
        MoveReview {
            seat,
            generation,
            revision,
            decision_serial,
            budget,
            decisions_used: analysis.decisions_used,
            fidelity: analysis.fidelity.clone(),
            cancelled: analysis.cancelled,
            candidates,
            preference,
            refusal: None,
        }
    }

    /// The candidates that actually carry a scored sample, in display order:
    /// highest mean practice outcome first, ties broken by more scored samples.
    /// Empty when nothing was scored — a zero-sample analysis is not an
    /// ordering, and this returns nothing rather than inventing one.
    pub fn ranked(&self) -> Vec<&CandidateRow> {
        let mut ranked: Vec<&CandidateRow> = self
            .candidates
            .iter()
            .filter(|candidate| candidate.mean.is_some())
            .collect();
        ranked.sort_by(|a, b| {
            b.mean
                .partial_cmp(&a.mean)
                .unwrap_or(std::cmp::Ordering::Equal)
                // A tie is broken toward the search's own selection, so the
                // displayed first row is the alternative the preference line
                // names.
                .then_with(|| b.chosen.cmp(&a.chosen))
                .then_with(|| b.scored_samples.cmp(&a.scored_samples))
        });
        ranked
    }

    /// The policy-preference section (ranked alternatives). Never rule text.
    pub fn policy_section(&self) -> ReviewSection {
        let mut lines = Vec::new();
        lines.push(format!(
            "reviewed decision point: P{} · engine revision {} · decision {}",
            self.seat, self.revision, self.decision_serial
        ));
        lines.push(self.budget.line());
        if let Some(reason) = &self.refusal {
            lines.push(format!("refused: {reason}"));
            lines.push(
                "no alternative is ranked: the search refused this decision point before any rollout, so this panel shows no ordering and no scores".to_string(),
            );
            lines.push(LINES_NOTE.to_string());
            lines.push(NO_VERDICT_NOTE.to_string());
            return ReviewSection {
                title: POLICY_TITLE,
                source: POLICY_SOURCE,
                lines,
            };
        }
        lines.push(format!(
            "used: {} of {} engine decision(s){}",
            self.decisions_used,
            self.budget.node_budget,
            if self.cancelled {
                " · cancelled early (the analysis stopped before its full budget; the rows below are the complete sample it had)"
            } else {
                ""
            }
        ));
        lines.push(
            "stopping rule: the analysis ends at whichever of the node budget, the per-rollout cap or the wall clock it reaches first, so a partial sample is normal"
                .to_string(),
        );
        lines.push(format!("fidelity: {}", self.fidelity));
        lines.push(LINES_NOTE.to_string());
        lines.push(match &self.preference {
            Preference::Search(label) => format!(
                "policy preference (a preference from scored practice outcomes — not a rule, and not a claim of strength): \"{label}\""
            ),
            Preference::Fallback(label) => format!(
                "policy preference: none — no candidate reached a result, so the search returned the deterministic legal action \"{label}\" (a fallback, not a preference)"
            ),
            Preference::None => "policy preference: none — nothing was scored for this decision point".to_string(),
        });
        let ranked = self.ranked();
        if ranked.is_empty() {
            lines.push(format!(
                "ordering: none — 0 of {} engine-offered candidate(s) reached a result, so no alternative is ranked above another",
                self.candidates.len()
            ));
        } else {
            lines.push("ordering: highest mean practice outcome first".to_string());
            lines.push(
                "score: mean practice outcome = win +1 / loss -1 / draw 0 over scored determinizations; a capped or failed rollout is counted as \"without result\", never as a win or a loss"
                    .to_string(),
            );
            lines.push(
                "uncertainty: sample counts differ between rows and more samples do not make a row better; these samples are small and small samples are not evidence of strength"
                    .to_string(),
            );
        }
        for (index, candidate) in ranked.iter().enumerate() {
            lines.push(format!("{}. {}", index + 1, candidate_line(candidate)));
        }
        let unscored: Vec<&CandidateRow> = self
            .candidates
            .iter()
            .filter(|candidate| candidate.mean.is_none())
            .collect();
        if !unscored.is_empty() {
            lines.push(
                "unscored offers (no rollout reached a result; listed in engine offer order and not ranked against each other):"
                    .to_string(),
            );
            for candidate in unscored {
                lines.push(format!("— {}", candidate_line(candidate)));
            }
        }
        lines.push(NO_VERDICT_NOTE.to_string());
        ReviewSection {
            title: POLICY_TITLE,
            source: POLICY_SOURCE,
            lines,
        }
    }

    /// One line for the busy/status readout when a review is applied.
    pub fn summary_line(&self) -> String {
        if let Some(reason) = &self.refusal {
            return format!(
                "move review refused for P{}: {reason} (nothing is ranked)",
                self.seat
            );
        }
        let ranked = self.ranked().len();
        let total = self.candidates.len();
        format!(
            "move review ready for P{}: {} of {} engine-offered candidate(s) scored over {} engine decision(s)",
            self.seat, ranked, total, self.decisions_used
        )
    }
}

/// One candidate row as the panel prints it: label, mean (or the absence of
/// one), and every count the search kept.
fn candidate_line(candidate: &CandidateRow) -> String {
    let samples = match candidate.scored_samples {
        1 => "1 scored sample".to_string(),
        n => format!("{n} scored samples"),
    };
    let refusing = if candidate.refused_times == 1 {
        "1 refused by the search world".to_string()
    } else {
        format!("{} refused by the search world", candidate.refused_times)
    };
    match candidate.mean {
        Some(mean) => format!(
            "{} — mean {mean:+.2} over {samples} · {} without result · {refusing}{}",
            candidate.label,
            candidate.without_result,
            if candidate.chosen {
                " · the search's own choice among equally scored offers"
            } else {
                ""
            }
        ),
        None => format!(
            "{} — no scored sample (mean: none) · {} without result · {refusing}",
            candidate.label, candidate.without_result
        ),
    }
}

/// One titled block of the review panel, with its own source label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSection {
    pub title: &'static str,
    pub source: &'static str,
    pub lines: Vec<String>,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use aperture_ai::CandidateScore;
    use aperture_ai::{SearchAnalysis, SearchOutcome};
    use aperture_core::Action;
    use aperture_core::coach::ExplanationKind;

    fn budget() -> ReviewBudget {
        ReviewBudget {
            seeds: 2,
            node_budget: 900,
            max_rollout_nodes: 150,
            max_millis: 400,
            max_candidates: 4,
        }
    }

    fn row(label: &str, mean: Option<f32>, scored: usize, without: usize) -> CandidateRow {
        CandidateRow {
            label: label.to_string(),
            mean,
            scored_samples: scored,
            without_result: without,
            refused_times: 0,
            chosen: false,
        }
    }

    fn chosen_row(label: &str, mean: Option<f32>, scored: usize) -> CandidateRow {
        CandidateRow {
            chosen: true,
            ..row(label, mean, scored, 0)
        }
    }

    #[test]
    fn an_equal_score_is_displayed_in_the_searchs_own_order() {
        // The search's tie-break picks a specific offer; the panel must not
        // print a different one first, or the ordering would contradict the
        // preference line beside it.
        let review = review_with(
            vec![
                row("Pass priority", Some(1.0), 1, 0),
                chosen_row("Play Forest", Some(1.0), 1),
            ],
            Preference::Search("Play Forest".to_string()),
        );
        let order: Vec<&str> = review
            .ranked()
            .iter()
            .map(|row| row.label.as_str())
            .collect();
        assert_eq!(
            order,
            vec!["Play Forest", "Pass priority"],
            "an equal score must be ordered by the search's own selection"
        );
        let shown = review.policy_section().lines.join("\n");
        assert!(
            shown.contains("1. Play Forest"),
            "the preferred offer must lead the list: {shown}"
        );
        assert!(
            shown.contains("the search's own choice"),
            "a tie must say why that row leads: {shown}"
        );
    }

    fn review_with(candidates: Vec<CandidateRow>, preference: Preference) -> MoveReview {
        MoveReview {
            seat: 0,
            generation: 4,
            revision: 9,
            decision_serial: 12,
            budget: budget(),
            decisions_used: 480,
            fidelity:
                "opponents: neutral belief deck (1 opponent(s)); no real hand or library order read"
                    .to_string(),
            cancelled: false,
            candidates,
            preference,
            refusal: None,
        }
    }

    fn all_lines(section: &ReviewSection) -> Vec<String> {
        section.lines.clone()
    }

    #[test]
    fn a_refusal_is_typed_and_ranks_nothing() {
        let reason = "non-priority decision (pending kind mulligan); search not enabled there";
        let review = MoveReview::refused(0, 4, 9, 12, budget(), reason.to_string());
        let section = review.policy_section();
        assert!(
            section.lines.iter().any(|line| line.contains(reason)),
            "the typed refusal reason must appear verbatim: {section:?}"
        );
        assert!(
            review.ranked().is_empty(),
            "a refused review must rank nothing: {review:?}"
        );
        assert!(
            section.lines.iter().all(|line| !line.contains("mean ")),
            "a refusal must not print a scored ordering: {section:?}"
        );
        assert!(
            section
                .lines
                .iter()
                .any(|line| line.contains("no alternative is ranked")),
            "the panel must say plainly that nothing is ranked: {section:?}"
        );
    }

    #[test]
    fn a_zero_sample_analysis_makes_no_ordering_claim() {
        let review = review_with(
            vec![
                row("Cast Opt", None, 0, 2),
                row("Pass", None, 0, 3),
                row("Play Island", None, 0, 1),
            ],
            Preference::Fallback("Pass".to_string()),
        );
        assert!(
            review.ranked().is_empty(),
            "zero scored samples are not an ordering: {:?}",
            review.ranked()
        );
        let lines = all_lines(&review.policy_section());
        let joined = lines.join("\n");
        assert!(
            joined.contains("no candidate reached a result"),
            "the panel must say no candidate reached a result: {joined}"
        );
        assert!(
            joined.contains("0 of 3"),
            "the panel must state how many offers produced no result: {joined}"
        );
        // Every offer is still shown with its own counts, so nothing is hidden.
        for label in ["Cast Opt", "Pass", "Play Island"] {
            assert!(
                joined.contains(label),
                "{label} missing from the panel: {joined}"
            );
        }
        assert!(
            joined.contains("2 without result"),
            "per-offer no-result counts must be shown: {joined}"
        );
        // A fallback is not a preference and must say so.
        assert!(
            joined.contains("fallback"),
            "a fallback choice must be labelled a fallback: {joined}"
        );
        assert!(
            !joined.contains("fell back to") || joined.contains("not a preference"),
            "the fallback line must deny being a preference: {joined}"
        );
    }

    #[test]
    fn a_ranked_analysis_orders_by_mean_with_sample_counts() {
        let review = review_with(
            vec![
                row("Pass", Some(0.0), 2, 0),
                row("Cast Opt", Some(-0.5), 2, 0),
                row("Play Island", Some(0.5), 1, 1),
            ],
            Preference::Search("Play Island".to_string()),
        );
        let order: Vec<&str> = review
            .ranked()
            .iter()
            .map(|row| row.label.as_str())
            .collect();
        assert_eq!(
            order,
            vec!["Play Island", "Pass", "Cast Opt"],
            "display order must be highest mean first"
        );
        let joined = all_lines(&review.policy_section()).join("\n");
        assert!(
            joined.contains("+0.50") && joined.contains("-0.50"),
            "signed means must be shown: {joined}"
        );
        assert!(
            joined.contains("1 scored sample") && joined.contains("2 scored samples"),
            "the sample count behind every mean must be shown: {joined}"
        );
        assert!(
            joined.contains("not evidence of strength"),
            "the ordering must say small samples are not evidence of strength: {joined}"
        );
        assert!(
            joined.contains("win +1 / loss -1 / draw 0")
                || joined.contains("win +1 / loss −1 / draw 0"),
            "the score must be defined as a practice outcome: {joined}"
        );
        assert!(
            joined.contains("not a rule"),
            "a policy preference must deny being a rule: {joined}"
        );
    }

    #[test]
    fn rules_and_policy_are_separate_labelled_sections() {
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
                text: "a strategy record never comes from a policy".to_string(),
                rules: Vec::new(),
            },
        ];
        let rules = rules_section(&explanations);
        assert_eq!(rules.title, RULES_TITLE);
        assert_eq!(rules.source, RULES_SOURCE);
        assert_ne!(rules.title, POLICY_TITLE);
        assert_ne!(rules.source, POLICY_SOURCE);
        let rules_text = rules.lines.join("\n");
        assert!(
            rules_text.contains("[RULE]") && rules_text.contains("117.1a"),
            "rule lines must carry their own badge and citation: {rules_text}"
        );
        assert!(
            !rules_text.contains("mean") && !rules_text.contains("scored sample"),
            "the rules section must never carry a policy score: {rules_text}"
        );

        let review = review_with(
            vec![row("Pass", Some(0.5), 2, 0)],
            Preference::Search("Pass".to_string()),
        );
        let policy = review.policy_section();
        let policy_text = policy.lines.join("\n");
        assert!(
            !policy_text.contains("[RULE]") && !policy_text.contains("CR "),
            "the policy section must never carry rule text: {policy_text}"
        );
        assert_eq!(policy.title, POLICY_TITLE);
        assert_eq!(policy.source, POLICY_SOURCE);
    }

    #[test]
    fn the_panel_discloses_its_boundary_budget_and_missing_lines() {
        let review = review_with(
            vec![row("Pass", Some(0.25), 4, 0)],
            Preference::Search("Pass".to_string()),
        );
        let joined = all_lines(&review.policy_section()).join("\n");
        assert!(
            joined.contains("no real hand or library order read"),
            "the search's own fidelity line must reach the panel: {joined}"
        );
        assert!(
            joined.contains("2 determinization seed(s)") && joined.contains("900 engine decisions"),
            "the compute budget must be disclosed: {joined}"
        );
        assert!(
            joined.contains("480"),
            "the decisions the analysis actually used must be shown: {joined}"
        );
        assert!(
            joined.contains("no principal variation"),
            "the panel must state that no line is available rather than invent one: {joined}"
        );
        assert!(
            joined.contains("P0") && joined.contains("decision 12"),
            "the reviewed decision point must be named: {joined}"
        );

        let mut cancelled = review.clone();
        cancelled.cancelled = true;
        let cancelled_text = cancelled.policy_section().lines.join("\n");
        assert!(
            cancelled_text.contains("cancelled"),
            "a cancelled analysis must say so: {cancelled_text}"
        );
    }

    #[test]
    fn no_line_claims_certainty_or_strength() {
        let mut reviews = vec![
            review_with(
                vec![
                    row("Pass", Some(0.5), 2, 0),
                    row("Cast Opt", Some(-0.25), 3, 1),
                ],
                Preference::Search("Pass".to_string()),
            ),
            review_with(
                vec![row("Pass", None, 0, 2)],
                Preference::Fallback("Pass".to_string()),
            ),
            MoveReview::refused(0, 1, 1, 1, budget(), NOT_YOUR_DECISION.to_string()),
        ];
        reviews[0].cancelled = true;
        for review in reviews {
            for line in review.policy_section().lines {
                let lower = line.to_lowercase();
                for banned in [
                    "best move",
                    "blunder",
                    "mistake",
                    "win probability",
                    "probability",
                    "stronger than",
                    "%",
                ] {
                    assert!(
                        !lower.contains(banned),
                        "the panel must never claim certainty: {banned:?} in {line:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_budget_converts_to_the_search_config_it_describes() {
        let config = REVIEW_BUDGET.config();
        assert_eq!(config.seeds, REVIEW_BUDGET.seeds);
        assert_eq!(config.node_budget, REVIEW_BUDGET.node_budget);
        assert_eq!(config.max_rollout_nodes, REVIEW_BUDGET.max_rollout_nodes);
        assert_eq!(config.max_millis, REVIEW_BUDGET.max_millis);
        assert_eq!(config.max_candidates, REVIEW_BUDGET.max_candidates);
        assert!(
            REVIEW_BUDGET.line().contains("determinization seed"),
            "the budget line must be readable: {}",
            REVIEW_BUDGET.line()
        );
    }

    #[test]
    fn a_completed_search_report_keeps_only_offered_labels_and_counts() {
        // The report a finished search produces must describe exactly the
        // candidates the search scored, in offer order, with its own counts.
        let scores = vec![
            CandidateScore {
                label: "Play Island".to_string(),
                action: Action::Pass,
                mean: Some(0.5),
                scored_samples: 2,
                rollouts_without_result: 0,
                refused_times: 0,
            },
            CandidateScore {
                label: "Pass".to_string(),
                action: Action::Pass,
                mean: None,
                scored_samples: 0,
                rollouts_without_result: 2,
                refused_times: 1,
            },
        ];
        let analysis = SearchAnalysis {
            candidates: scores,
            decisions_used: 320,
            cancelled: false,
            fidelity: "opponents: neutral belief deck (1 opponent(s)); no real hand or library order read; own future draws from the neutral belief deck".to_string(),
        };
        let outcome = SearchOutcome {
            decision: aperture_ai::Decision {
                action: Action::Pass,
                source: aperture_ai::DecisionSource::Search,
                note: None,
                preference: None,
            },
            analysis,
        };
        let offers: Vec<LegalAction> = Vec::new();
        let review = MoveReview::from_outcome(0, 4, 9, 12, budget(), &outcome, &offers);
        assert_eq!(review.candidates.len(), 2);
        assert_eq!(review.candidates[0].label, "Play Island");
        assert_eq!(review.candidates[1].refused_times, 1);
        assert_eq!(review.decisions_used, 320);
        assert_eq!(review.fidelity, outcome.analysis.fidelity);
        assert!(
            review.refusal.is_none(),
            "a scored analysis is not a refusal"
        );
        // The chosen action is a `Pass` that is not an offer label in this
        // fixture, so the preference is reported as the fallback/unlabelled
        // case rather than inventing a label.
        assert!(
            matches!(
                review.preference,
                Preference::Search(_) | Preference::Fallback(_)
            ),
            "the preference must be labelled either way: {:?}",
            review.preference
        );
    }
}
