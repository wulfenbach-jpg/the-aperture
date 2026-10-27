//! Guardrail suite for the move review (spec §6): AC-2, AC-3, AC-4 and the structural half of
//! AC-11.
//!
//! Three layers, because a single layer is easy to fool:
//!
//! 1. **Text.** Forbidden constructs are rejected sentence by sentence, with the exact violation
//!    code asserted, and the two shipped disclaimers are proved to stay allowed.
//! 2. **Payload.** A committed corpus of representative payloads is audited end to end: the honest
//!    ones must produce zero violations, the known-bad ones must produce their specific violation.
//! 3. **Copy.** A source scan over every string literal in `crates/ai/src` (plus
//!    `crates/core/src/coach.rs` and, when it exists, `crates/ui/src/review.rs`) fails when a
//!    forbidden construct appears in review-facing copy, so adding a hard-coded "72% chance" to a
//!    formatter breaks CI instead of shipping.
//!
//! The failure this suite exists for is not a crash. It is a sentence that sounds like analysis and
//! is actually an invention, so the assertions name the sentence and the code.

use std::path::{Path, PathBuf};

use aperture_ai::review::honesty::{
    AuditSources, HonestyViolation, ReasonSource, ReviewSections, SectionLabels, audit_review,
    audit_review_against, audit_sections, audit_text, format_budget_line, format_outcome,
    format_reason, payload_texts, payload_texts_with,
};
use aperture_ai::review::{
    CandidateLine, EvidenceRef, LineScope, LineStep, RankedReview, ReasonTag, ReviewText, TextKind,
    Uncertainty,
};
use aperture_core::coach::{Explanation, ExplanationKind};

// ---------------------------------------------------------------------------------------------
// Fixture loading
// ---------------------------------------------------------------------------------------------

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/review")
}

fn load(name: &str) -> RankedReview {
    let path = fixture_dir().join(name);
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture {} is unreadable: {error}", path.display()));
    serde_json::from_str(&body).unwrap_or_else(|error| {
        panic!(
            "fixture {} does not match the payload schema: {error}",
            path.display()
        )
    })
}

fn codes(violations: &[HonestyViolation]) -> Vec<&'static str> {
    violations.iter().map(HonestyViolation::code).collect()
}

fn assert_clean(review: &RankedReview, sources: &AuditSources<'_>) {
    let violations = audit_review_against(review, sources);
    assert!(
        violations.is_empty(),
        "an honest payload must produce zero violations, got: {violations:?}"
    );
}

fn expect(violations: &[HonestyViolation], code: &str) {
    assert!(
        codes(violations).contains(&code),
        "expected a {code} violation, got {:?}",
        violations
    );
}

/// The engine's own explanation used by the rule-trace tests.
fn rule_explanation() -> Explanation {
    Explanation {
        topic: "instants and priority".to_string(),
        kind: ExplanationKind::Rule,
        text: "An instant may be cast any time its controller has priority.".to_string(),
        rules: vec!["117.1a"],
    }
}

/// A one-candidate payload whose only reason quotes an engine rule.
fn engine_rule_payload() -> RankedReview {
    let mut review = load("honest_single_offer.json");
    review.candidates[0].reasons = vec![
        ReasonTag::SoleLegalOffer,
        ReasonTag::EngineRuleApplies {
            rule: "117.1a".to_string(),
            topic: "instants and priority".to_string(),
        },
    ];
    review
}

// ---------------------------------------------------------------------------------------------
// Layer 1 — text
// ---------------------------------------------------------------------------------------------

#[test]
fn a_fabricated_percentage_is_rejected_with_no_negation_excuse() {
    // The exact sentence the card names as the acceptance fixture.
    let violations = audit_text("72% chance this is best");
    expect(&violations, "percent_figure");
    expect(&violations, "probability_claim");

    // Percent figures have no negation excuse at all.
    let negated = audit_text("there is no 72% chance this is best");
    expect(&negated, "percent_figure");

    // A bare figure in prose about outcomes is enough.
    let compact = audit_text("winning lines: 63 % of the time");
    expect(&compact, "percent_figure");
}

#[test]
fn probability_and_strength_vocabulary_is_rejected() {
    for sentence in [
        "the win probability is high",
        "you have a 4-in-5 chance of winning",
        "the engine winrate favours this line",
        "the win rate here is better",
        "the odds favour passing",
        "this line has the best equity",
        "the expected score is higher",
        "this is worth about 40 centipawns",
        "our Elo model prefers this",
        "the tablebase says this is drawn",
        "this likely wins the game",
    ] {
        let violations = audit_text(sentence);
        expect(&violations, "probability_claim");
    }
}

#[test]
fn verdict_labels_are_rejected_outside_a_negation() {
    for (sentence, word) in [
        ("this was a mistake", "mistake"),
        ("a forced blunder", "blunder"),
        ("that line is an inaccuracy", "inaccuracy"),
        ("a brilliant idea", "brilliant"),
        ("the best move is to pass", "best move"),
        ("the correct move is to play the land", "correct move"),
        ("you should have played the land", "should have"),
        ("the engine reports an error here", "error"),
    ] {
        let violations = audit_text(sentence);
        let found = violations.iter().any(|violation| {
            matches!(
                violation,
                HonestyViolation::VerdictLabel { word: found, .. } if *found == word
            )
        });
        assert!(
            found,
            "expected a verdict_label({word}) violation for {sentence:?}, got {violations:?}"
        );
    }
}

#[test]
fn the_shipped_disclaimers_stay_allowed() {
    // These are real product sentences; legalising them is precisely why rule 5 has its negation
    // clause. If a future tightening breaks one of them, the review loses its own disclaimers.
    for sentence in [
        "it labels no alternative as an error",
        "preference not reported; not a win probability",
        "preference 0.412; not a win probability",
        "the search preferred this alternative and never called it a mistake",
        "nothing here is a probability",
        "neither alternative is a blunder",
    ] {
        let violations = audit_text(sentence);
        assert!(
            violations.is_empty(),
            "{sentence:?} is a permitted disclaimer but produced {violations:?}"
        );
    }
}

#[test]
fn ordinary_review_copy_passes() {
    for sentence in [
        "mean practice outcome +0.50 over 2 scored sample(s)",
        "the search's own choice among equally scored offers",
        "the engine offers exactly one action at this decision point",
        "1 rollout ended without a result and is counted as neither a win nor a loss",
        "the analysis stopped at the wall clock so this sample is incomplete",
        "[RULE] instants and priority: An instant may be cast any time its controller has priority. (CR 117.1a)",
    ] {
        assert!(
            audit_text(sentence).is_empty(),
            "{sentence:?} is honest review copy but was rejected: {:?}",
            audit_text(sentence)
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Layer 2 — payload corpus
// ---------------------------------------------------------------------------------------------

#[test]
fn every_honest_payload_in_the_corpus_is_clean() {
    let mut found = 0usize;
    for entry in std::fs::read_dir(fixture_dir()).expect("fixture corpus is readable") {
        let path = entry.expect("corpus entry").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        if !name.starts_with("honest_") || !name.ends_with(".json") {
            continue;
        }
        found += 1;
        let review = load(&name);
        assert_clean(&review, &AuditSources::empty());
    }
    assert!(
        found >= 3,
        "the corpus must keep at least three honest payloads, found {found}"
    );
}

#[test]
fn payloads_round_trip_through_json_byte_identically() {
    // §4.1: the corpus harness must be able to store and replay a payload verbatim.
    for name in [
        "honest_ranked.json",
        "honest_refusal.json",
        "honest_single_offer.json",
    ] {
        let review = load(name);
        let first = serde_json::to_string(&review).expect("serialise");
        let again: RankedReview = serde_json::from_str(&first).expect("deserialise");
        let second = serde_json::to_string(&again).expect("serialise again");
        assert_eq!(first, second, "{name} does not round-trip byte-identically");
        assert_eq!(
            review, again,
            "{name} does not round-trip to an equal payload"
        );
    }
}

#[test]
fn a_percentage_in_a_review_payload_fails() {
    let review = load("bad_percent.json");
    let violations = audit_review(&review);
    expect(&violations, "percent_figure");
    expect(&violations, "probability_claim");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            HonestyViolation::PercentFigure { sentence } if sentence.contains("72%")
        )),
        "the violation must quote the offending sentence, got {violations:?}"
    );
}

#[test]
fn a_blunder_verdict_in_a_review_payload_fails() {
    let review = load("bad_blunder.json");
    let violations = audit_review(&review);
    expect(&violations, "verdict_label");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            HonestyViolation::VerdictLabel { word, .. } if *word == "blunder"
        )),
        "expected the blunder label to be named, got {violations:?}"
    );
}

#[test]
fn a_fabricated_line_step_fails() {
    let review = load("bad_line_step.json");
    let violations = audit_review(&review);
    expect(&violations, "unresolvable_evidence");
}

#[test]
fn an_unregistered_reason_tag_fails() {
    let review = load("bad_unregistered_reason.json");
    let violations = audit_review(&review);
    expect(&violations, "unregistered_reason");
    // A positional tag is not rescued by being spelled like a rule citation.
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            HonestyViolation::UnregisteredReason { tag } if tag.contains("hanging piece")
        )),
        "expected the positional topic to be named, got {violations:?}"
    );
}

#[test]
fn a_sentence_without_evidence_fails() {
    let review = load("bad_no_evidence.json");
    let violations = audit_review(&review);
    expect(&violations, "untraceable_claim");
}

#[test]
fn a_tie_claimed_as_a_unique_choice_fails() {
    // AC-8 retained regression: the unique-maximum phrase must not survive a real tie.
    let review = load("bad_fake_tie.json");
    let violations = audit_review(&review);
    expect(&violations, "untraceable_claim");
}

#[test]
fn a_truncated_analysis_may_not_claim_reproducibility() {
    let review = load("bad_reproducible.json");
    let violations = audit_review(&review);
    expect(&violations, "untraceable_claim");
}

#[test]
fn a_positional_reason_topic_fails_against_the_engine_explanations() {
    // The registry check passes ("hanging piece" is a non-empty topic and 117.1a is pinned); what
    // fails is that no engine explanation establishes that topic. This is the guard against
    // positional vocabulary the project has no trace for.
    let mut review = load("honest_single_offer.json");
    review.candidates[0].reasons = vec![ReasonTag::EngineRuleApplies {
        rule: "117.1a".to_string(),
        topic: "hanging piece".to_string(),
    }];
    let explanations = vec![rule_explanation()];
    let violations = audit_review_against(
        &review,
        &AuditSources {
            offer: &[],
            explanations: &explanations,
        },
    );
    expect(&violations, "untraceable_claim");
}

#[test]
fn a_strategy_record_may_not_be_quoted_as_a_rule() {
    let mut review = load("honest_single_offer.json");
    review.candidates[0].reasons = vec![ReasonTag::EngineRuleApplies {
        rule: "117.1a".to_string(),
        topic: "instants and priority".to_string(),
    }];
    let mut explanation = rule_explanation();
    explanation.kind = ExplanationKind::Strategy;
    let explanations = vec![explanation];
    let sources = AuditSources {
        offer: &[],
        explanations: &explanations,
    };
    let violations = audit_review_against(&review, &sources);
    expect(&violations, "untraceable_claim");
    // And it never becomes a sentence at all.
    assert!(
        payload_texts_with(&review, &sources)
            .iter()
            .all(|text| text.kind != TextKind::Rule),
        "a strategy record must not be rendered as rule text"
    );
}

#[test]
fn an_engine_rule_sentence_is_clean_and_traceable() {
    let review = engine_rule_payload();
    let explanations = vec![rule_explanation()];
    let sources = AuditSources {
        offer: &[],
        explanations: &explanations,
    };
    assert_clean(&review, &sources);
    let texts = payload_texts_with(&review, &sources);
    let rule = texts
        .iter()
        .find(|text| text.kind == TextKind::Rule)
        .expect("the engine rule reason must produce a rule sentence");
    assert!(
        rule.body.contains("(CR 117.1a)") && rule.body.starts_with("[RULE]"),
        "rule copy must be badged and cite the pinned number: {}",
        rule.body
    );
    assert!(
        rule.evidence
            .iter()
            .any(|reference| matches!(reference, EvidenceRef::RuleCitation { number } if number == "117.1a")),
        "the rule sentence must carry its citation as evidence"
    );
}

#[test]
fn an_unscored_row_prints_no_mean_and_a_scored_row_prints_its_count() {
    let review = load("honest_ranked.json");
    let unscored = review.candidate(2).expect("unscored candidate");
    assert!(
        format_outcome(unscored).is_none(),
        "a row with no scored sample has no mean to print"
    );
    let scored = review.candidate(0).expect("scored candidate");
    let text = format_outcome(scored).expect("scored rows print a mean");
    assert!(text.body.contains("+0.50"), "{}", text.body);
    assert!(text.body.contains("2 scored sample"), "{}", text.body);
}

#[test]
fn a_sentence_labelled_as_a_rule_must_cite_the_rule() {
    let mut review = load("honest_single_offer.json");
    review.notes.push(ReviewText {
        kind: TextKind::Rule,
        body: "instants may be cast when their controller has priority".to_string(),
        evidence: vec![EvidenceRef::EngineOffer { index: 0 }],
    });
    let violations = audit_review(&review);
    expect(&violations, "untraceable_claim");
    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            HonestyViolation::UntraceableClaim { sentence } if sentence.contains("CR citation")
        )),
        "the uncited rule sentence must be named, got {violations:?}"
    );
}

#[test]
fn every_formatter_output_is_clean_and_carries_evidence() {
    for name in [
        "honest_ranked.json",
        "honest_refusal.json",
        "honest_single_offer.json",
    ] {
        let review = load(name);
        for candidate in &review.candidates {
            let mut texts = Vec::new();
            if let Some(text) = format_outcome(candidate) {
                texts.push(text);
            }
            for tag in &candidate.reasons {
                let source = ReasonSource::new(candidate, &review.budget);
                if let Some(text) = format_reason(tag, &source) {
                    texts.push(text);
                }
            }
            for text in texts {
                assert!(
                    audit_text(&text.body).is_empty(),
                    "{name}: formatter emitted {text:?} for candidate {}",
                    candidate.offer_index
                );
                assert!(
                    !text.evidence.is_empty(),
                    "{name}: formatter emitted a sentence with no evidence: {text:?}"
                );
            }
        }
    }
}

#[test]
fn the_budget_line_is_a_pure_function_of_the_record() {
    let review = load("honest_ranked.json");
    let budget = review.budget.clone();
    let line = format_budget_line(&budget);
    assert!(line.contains("stopped: node budget"), "{line}");
    assert!(line.contains("reproducible: yes"), "{line}");
    assert!(line.contains("unbounded"), "{line}");
    assert!(
        !line.contains('%'),
        "the budget line must never print a percentage"
    );
    assert!(audit_text(&line).is_empty(), "{line}");

    // Every field the rendered line names must change it: the line is a function of the record,
    // not a hand-written sentence. (`cancelled` and `fidelity` are not part of this line: §3.4
    // prints them separately.)
    let mut changed = budget.clone();
    changed.decisions_used += 1;
    assert_ne!(line, format_budget_line(&changed));
    let mut not_reproducible = budget.clone();
    not_reproducible.reproducible = false;
    assert!(
        format_budget_line(&not_reproducible).contains("reproducible: no"),
        "{}",
        format_budget_line(&not_reproducible)
    );
    let mut fewer_seeds = budget.clone();
    fewer_seeds.seeds = 4;
    assert_ne!(line, format_budget_line(&fewer_seeds));
    let mut bounded = budget.clone();
    bounded.max_millis = 8000;
    assert!(format_budget_line(&bounded).contains("8000 ms"));
}

// ---------------------------------------------------------------------------------------------
// Layer 2b — section structure (AC-11)
// ---------------------------------------------------------------------------------------------

fn labels() -> SectionLabels<'static> {
    SectionLabels {
        rules_title: "WHAT THE RULES ALLOWED",
        rules_source: "engine rule citations",
        policy_title: "WHAT THE SEARCH PREFERRED",
        policy_source: "bounded search practice outcomes",
    }
}

fn placement<'a>(texts: &'a [ReviewText]) -> ReviewSections<'a> {
    let mut sections = ReviewSections::default();
    for text in texts {
        match text.kind {
            TextKind::Rule => sections.rules.push(text),
            _ => sections.policy.push(text),
        }
    }
    sections
}

#[test]
fn the_two_sections_stay_separate_and_complete() {
    let review = engine_rule_payload();
    let explanations = vec![rule_explanation()];
    let sources = AuditSources {
        offer: &[],
        explanations: &explanations,
    };
    let texts = payload_texts_with(&review, &sources);
    let sections = placement(&texts);
    assert!(
        audit_sections(&review, &sources, &labels(), &sections).is_empty(),
        "a faithful placement must be clean: {:?}",
        audit_sections(&review, &sources, &labels(), &sections)
    );

    // Collapsing the two headings into one surface is a failure even when every sentence is fine.
    let collapsed = SectionLabels {
        rules_title: "REVIEW",
        rules_source: "review source",
        policy_title: "REVIEW",
        policy_source: "review source",
    };
    expect(
        &audit_sections(&review, &sources, &collapsed, &sections),
        "missing_public_distinction",
    );

    // Rule text rendered in the policy section is a failure.
    let mut crossed = placement(&texts);
    let rule_copy: Vec<&ReviewText> = crossed.rules.clone();
    crossed.policy.extend(rule_copy);
    crossed.rules.clear();
    expect(
        &audit_sections(&review, &sources, &labels(), &crossed),
        "rule_text_in_policy_section",
    );

    // Policy text rendered in the rules section is a failure.
    let mut reversed = placement(&texts);
    let policy_copy = reversed.policy[0];
    reversed.rules.push(policy_copy);
    expect(
        &audit_sections(&review, &sources, &labels(), &reversed),
        "policy_text_in_rules_section",
    );

    // A payload sentence the presentation never placed is a failure.
    let mut dropped = placement(&texts);
    dropped.policy.remove(0);
    expect(
        &audit_sections(&review, &sources, &labels(), &dropped),
        "unlabelled_text_kind",
    );
}

#[test]
fn the_reason_registry_matches_the_spec_table() {
    // Step 1 of the documented procedure: a tag exists in the spec table *and* in the enum.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = crate_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let spec = std::fs::read_to_string(repo.join("docs/design/move-review-spec.md"))
        .expect("the move-review spec must be in the tree");
    let table = spec
        .split("### 4.6 Reason tags (closed registry)")
        .nth(1)
        .expect("spec §4.6 must exist")
        .split("### 4.7")
        .next()
        .expect("spec §4.6 must end before §4.7");
    let mut spec_tags: Vec<String> = Vec::new();
    for line in table.lines() {
        let Some(rest) = line.trim_start().strip_prefix("| `") else {
            continue;
        };
        let Some(end) = rest.find('`') else { continue };
        let name = rest[..end].split_whitespace().next().unwrap_or_default();
        if !name.is_empty() {
            spec_tags.push(name.to_string());
        }
    }
    spec_tags.sort();
    spec_tags.dedup();
    assert!(
        spec_tags.len() >= 9,
        "spec §4.6 should list the whole registry, found {spec_tags:?}"
    );

    let mut code_tags: Vec<String> = [
        ReasonTag::HighestPracticeOutcome,
        ReasonTag::EqualPracticeOutcome,
        ReasonTag::NoScoredSample,
        ReasonTag::IncompleteRollouts,
        ReasonTag::RefusedBySearchWorld,
        ReasonTag::LowSample,
        ReasonTag::BudgetStopped,
        ReasonTag::SoleLegalOffer,
        ReasonTag::EngineRuleApplies {
            rule: "117.1a".to_string(),
            topic: "instants and priority".to_string(),
        },
    ]
    .iter()
    .map(|tag| tag.registry_row().expect("registered").to_string())
    .collect();
    code_tags.sort();
    code_tags.dedup();
    assert_eq!(
        spec_tags, code_tags,
        "the enum and the spec §4.6 table have drifted apart"
    );

    let registered = [
        ReasonTag::HighestPracticeOutcome,
        ReasonTag::EqualPracticeOutcome,
        ReasonTag::NoScoredSample,
        ReasonTag::IncompleteRollouts,
        ReasonTag::RefusedBySearchWorld,
        ReasonTag::LowSample,
        ReasonTag::BudgetStopped,
        ReasonTag::SoleLegalOffer,
        ReasonTag::EngineRuleApplies {
            rule: "117.1a".to_string(),
            topic: "instants and priority".to_string(),
        },
    ];
    for tag in registered {
        assert!(
            tag.registry_row().is_some(),
            "{} must be in the §4.6 registry",
            tag.tag_name()
        );
        assert!(
            !tag.required_evidence().is_empty(),
            "{} must name the evidence it needs",
            tag.tag_name()
        );
    }
    // An engine rule tag needs a pinned CR number; an unpinned one is not a rule citation.
    let unpinned = ReasonTag::EngineRuleApplies {
        rule: "999.9".to_string(),
        topic: "hanging piece".to_string(),
    };
    assert!(
        unpinned.registry_row().is_none(),
        "an unpinned rule number must not register"
    );
}

#[test]
fn audit_review_and_payload_texts_agree_on_the_corpus() {
    // payload_texts is the audit surface; if it silently dropped a sentence, the checks below
    // would pass on an empty list.
    for name in ["honest_ranked.json", "honest_single_offer.json"] {
        let review = load(name);
        let texts = payload_texts(&review);
        let expected_notes = review.notes.len();
        assert!(
            texts.len() >= expected_notes,
            "{name}: payload texts must include every note"
        );
        assert!(
            texts.iter().all(|text| !text.body.trim().is_empty()),
            "{name}: an empty sentence is not auditable"
        );
    }
}

#[test]
fn uncertainty_states_must_match_the_sample_counts() {
    let mut review = load("honest_ranked.json");
    // A row with one sample cannot be ordered against another row.
    review.candidates[0].scored_samples = 1;
    review.candidates[0].uncertainty = Uncertainty::Ordered;
    let violations = audit_review(&review);
    expect(&violations, "untraceable_claim");

    // A line attached to a row that never reached a result is a fabricated line.
    let mut invented = load("honest_ranked.json");
    invented.candidates[2].line = Some(CandidateLine {
        steps: vec![LineStep {
            decision: 0,
            seat: 0,
            label: "Pass priority".to_string(),
        }],
        truncated: false,
        seed_index: 0,
        outcome: 0,
        scope: LineScope::SearchGame,
    });
    expect(&audit_review(&invented), "untraceable_claim");
}

// ---------------------------------------------------------------------------------------------
// Layer 3 — copy scan
// ---------------------------------------------------------------------------------------------

/// Extract every string literal from Rust source with its 1-based line number.
///
/// Comments and char literals are skipped; string escapes are unescaped; raw strings are read
/// verbatim. The scan only needs to see copy a human could read, so anything that is not a literal
/// is out of scope by construction.
fn string_literals(src: &str) -> Vec<(usize, String)> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    if bytes[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
            }
            b'r' if starts_raw_string(bytes, i) => {
                let start_line = line;
                let (body, consumed, newlines) = read_raw_string(bytes, i);
                line += newlines;
                i += consumed;
                out.push((start_line, body));
            }
            b'"' => {
                let start_line = line;
                let mut body = String::new();
                i += 1;
                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' => {
                            if let Some(escaped) = bytes.get(i + 1) {
                                match escaped {
                                    b'n' => body.push('\n'),
                                    b't' => body.push('\t'),
                                    other => body.push(*other as char),
                                }
                            }
                            i += 2;
                        }
                        b'"' => {
                            i += 1;
                            break;
                        }
                        b'\n' => {
                            line += 1;
                            body.push('\n');
                            i += 1;
                        }
                        other => {
                            body.push(other as char);
                            i += 1;
                        }
                    }
                }
                out.push((start_line, body));
            }
            b'\'' => {
                // Char literal or lifetime; only the first is interesting and neither carries copy.
                if bytes.get(i + 1) == Some(&b'\\') {
                    i += 2;
                    while i < bytes.len() && bytes[i] != b'\'' {
                        i += 1;
                    }
                    i += 1;
                } else if bytes.get(i + 2) == Some(&b'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

fn starts_raw_string(bytes: &[u8], i: usize) -> bool {
    let previous_is_ident = i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
    if previous_is_ident {
        return false;
    }
    let mut j = i + 1;
    while bytes.get(j) == Some(&b'#') {
        j += 1;
    }
    bytes.get(j) == Some(&b'"')
}

fn read_raw_string(bytes: &[u8], start: usize) -> (String, usize, usize) {
    let mut j = start + 1;
    let mut hashes = 0usize;
    while bytes.get(j) == Some(&b'#') {
        hashes += 1;
        j += 1;
    }
    // Skip the opening quote.
    j += 1;
    let body_start = j;
    while j < bytes.len() {
        if bytes[j] == b'"' {
            let mut close = j + 1;
            let mut seen = 0usize;
            while seen < hashes && bytes.get(close) == Some(&b'#') {
                seen += 1;
                close += 1;
            }
            if seen == hashes {
                let body = String::from_utf8_lossy(&bytes[body_start..j]).to_string();
                let consumed = close - start;
                let newlines = body.matches('\n').count();
                return (body, consumed, newlines);
            }
        }
        j += 1;
    }
    (String::new(), bytes.len() - start, 0)
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn review_facing_copy_strings_are_honest() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = crate_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let mut files = Vec::new();
    collect_rust_files(&crate_dir.join("src"), &mut files);
    files.push(repo.join("crates/core/src/coach.rs"));
    files.push(repo.join("crates/ui/src/review.rs"));

    let mut scanned = 0usize;
    let mut failures = Vec::new();
    for file in files {
        if !file.exists() {
            // The UI review panel arrives with the presentation task; when it lands, this scan
            // covers it with no change here.
            continue;
        }
        let normalised = file.to_string_lossy().replace('\\', "/");
        if normalised.ends_with("crates/ai/src/review/honesty.rs") {
            // This module holds the detector's own pattern table and its negative fixtures. Its
            // *output* is audited instead: see every_formatter_output_is_clean_and_carries_evidence.
            continue;
        }
        let Ok(src) = std::fs::read_to_string(&file) else {
            continue;
        };
        scanned += 1;
        for (line, literal) in string_literals(&src) {
            for violation in audit_text(&literal) {
                failures.push(format!("{normalised}:{line}: {violation}"));
            }
        }
    }
    assert!(
        scanned >= 3,
        "the copy scan must cover the review-facing sources, scanned {scanned}"
    );
    assert!(
        failures.is_empty(),
        "review copy violates the honesty boundary:\n{}",
        failures.join("\n")
    );
}

#[test]
fn the_copy_scanner_actually_sees_string_literals() {
    // A scanner that silently extracts nothing would make the previous test vacuous.
    let sample = r###"
        // "72% chance this is best" in a comment is not copy
        let fine = "mean practice outcome +0.50 over 2 scored sample(s)";
        let bad = "72% chance this is best";
        let raw = r#"forced blunder"#;
    "###;
    let literals: Vec<String> = string_literals(sample)
        .into_iter()
        .map(|(_, text)| text)
        .collect();
    assert_eq!(literals.len(), 3, "got {literals:?}");
    assert!(audit_text(&literals[0]).is_empty());
    expect(&audit_text(&literals[1]), "percent_figure");
    expect(&audit_text(&literals[2]), "verdict_label");
}
