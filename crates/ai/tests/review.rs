//! Integration tests for the move-review ranking layer (#28).
//!
//! Proven by execution, not by reading the module. Each test names the
//! acceptance criterion it owns in `docs/design/move-review-spec.md`:
//!
//! - AC-1  identical masked views ⇒ byte-identical payload (hidden-read proof),
//! - AC-4  every sentence carries evidence that resolves inside the payload,
//! - AC-5  two in-process runs and one out-of-process run are byte-equal,
//! - AC-6  a wall-clock or cancelled stop is labelled not reproducible,
//! - AC-7  ties at the lattice resolution are explicit and symmetric,
//! - AC-9  a refusal ranks nothing and keeps the typed reason verbatim,
//! - AC-10 zero scored samples rank nothing,
//! - AC-13 the budget record is recorded and its rendered line is a pure
//!   function of it,
//! - AC-15 every line step is an engine-submitted action, and a row with no
//!   scored sample has no line,
//! - AC-17 no depth/plies/nodes-per-option measurement is reported.
//!
//! The ranking layer encodes no game rule: every legality statement in these
//! tests is checked against the engine's own `PlayerView::legal` offer.

mod common;

use std::cmp::Ordering;
use std::process::Command;

use aperture_ai::review::{
    BudgetRecord, EvidenceRef, Preference, RankedCandidate, RankedReview, ReasonTag, ReviewBinding,
    TextKind, Uncertainty, rank, rank_outcome, reason_text, recorded_config,
};
use aperture_ai::{
    CandidateScore, Knowledge, MAX_LINE_PLIES, RecordedLine, RecordedStep, SearchAnalysis,
    SearchCancellation, SearchConfig, SearchOutcome, StopReason, search,
};
use aperture_core::state::Step;
use aperture_core::view::PlayerView;
use aperture_core::*;

// ---------------------------------------------------------------- fixtures

/// A budget that can only be stopped by its own node and round bounds, so a
/// recorded run is the only kind these tests produce (AC-5, §3.3).
fn test_config() -> SearchConfig {
    SearchConfig {
        // Four seeds rather than the measured default of two: at two samples a
        // lattice step is 0.50, so most real positions come back tied. Four
        // samples put the step at 0.25 and let a real ordering appear, which is
        // what these tests need to exercise.
        seeds: 4,
        node_budget: 12_000,
        max_rollout_nodes: 2_000,
        max_millis: u64::MAX,
        max_candidates: 4,
    }
}

fn pool(colors: &[(ManaColor, u8)]) -> ManaPool {
    let mut pool = ManaPool::default();
    for (color, count) in colors {
        // Keep the helper independent of the pool's count width: master holds
        // `u8` per colour, while the corpus branch widens `ManaPool::add` to
        // `u32`. A plain `*count` argument is what makes the merge ref fail to
        // compile in the `pull_request` CI run; `into()` converts implicitly.
        pool.add(*color, (*count).into());
    }
    pool
}

fn ready_pool() -> ManaPool {
    pool(&[
        (ManaColor::Red, 2),
        (ManaColor::Green, 3),
        (ManaColor::Blue, 1),
        (ManaColor::White, 1),
    ])
}

/// The five-position set the corpus harness will grow from (§8). Each is a real
/// engine state at a plain priority decision with several engine-offered
/// actions, so the ranking layer sees a genuine offer.
fn position_set() -> Vec<(&'static str, Game)> {
    vec![
        (
            "wide-board-with-removal",
            common::fixture(11)
                .battlefield(P0, &[CardId::Forest, CardId::Mountain, CardId::Island])
                .hand(P0, &[CardId::GrizzlyBears, CardId::Shock, CardId::Forest])
                .library(
                    P0,
                    &[
                        CardId::Forest,
                        CardId::Mountain,
                        CardId::Island,
                        CardId::Forest,
                        CardId::Mountain,
                        CardId::Forest,
                    ],
                )
                .library(P1, &[CardId::Island; 6])
                .hand(P1, &[CardId::Island, CardId::Island, CardId::Shock])
                .mana(P0, ready_pool())
                .main(P0, Step::PrecombatMain)
                .build(),
        ),
        (
            "two-creatures-one-land-drop",
            common::fixture(23)
                .battlefield(P0, &[CardId::Forest, CardId::Forest])
                .hand(
                    P0,
                    &[CardId::GrizzlyBears, CardId::GrizzlyBears, CardId::Mountain],
                )
                .library(
                    P0,
                    &[
                        CardId::Forest,
                        CardId::Mountain,
                        CardId::Forest,
                        CardId::Forest,
                        CardId::Mountain,
                    ],
                )
                .library(P1, &[CardId::Island; 5])
                .hand(P1, &[CardId::Island])
                .mana(P0, pool(&[(ManaColor::Green, 2)]))
                .main(P0, Step::PrecombatMain)
                .build(),
        ),
        (
            "creature-ready-to-attack",
            common::fixture(37)
                .battlefield(
                    P0,
                    &[CardId::GrizzlyBears, CardId::Forest, CardId::Mountain],
                )
                .hand(P0, &[CardId::GrizzlyBears, CardId::Shock])
                .library(
                    P0,
                    &[
                        CardId::Forest,
                        CardId::Mountain,
                        CardId::Forest,
                        CardId::Mountain,
                        CardId::Forest,
                    ],
                )
                .library(P1, &[CardId::Island; 4])
                .hand(P1, &[CardId::Island])
                .life(P1, 6)
                .mana(P0, pool(&[(ManaColor::Red, 1), (ManaColor::Green, 1)]))
                .main(P0, Step::PrecombatMain)
                .build(),
        ),
        (
            "low-life-removal-race",
            common::fixture(53)
                .battlefield(P0, &[CardId::Mountain, CardId::Mountain, CardId::Forest])
                .hand(P0, &[CardId::Shock, CardId::Shock, CardId::GrizzlyBears])
                .library(
                    P0,
                    &[
                        CardId::Mountain,
                        CardId::Forest,
                        CardId::Mountain,
                        CardId::Forest,
                        CardId::Mountain,
                    ],
                )
                .library(P1, &[CardId::Island; 4])
                .hand(P1, &[CardId::Island])
                .life(P0, 4)
                .life(P1, 4)
                .mana(P0, pool(&[(ManaColor::Red, 2), (ManaColor::Green, 1)]))
                .main(P0, Step::PrecombatMain)
                .build(),
        ),
        (
            "single-card-hand",
            common::fixture(71)
                .battlefield(P0, &[CardId::Forest])
                .hand(P0, &[CardId::GrizzlyBears, CardId::Forest])
                .library(
                    P0,
                    &[
                        CardId::Forest,
                        CardId::Forest,
                        CardId::Forest,
                        CardId::Forest,
                        CardId::Forest,
                    ],
                )
                .library(P1, &[CardId::Island; 3])
                .hand(P1, &[])
                .mana(P0, pool(&[(ManaColor::Green, 1)]))
                .main(P0, Step::PrecombatMain)
                .build(),
        ),
        (
            // The one position that is not decided by material: an unanswered
            // 2/2 kills a two-life seat next combat, so the removal offer and
            // the do-nothing offer must not come back level.
            "must-answer-lethal-attacker",
            common::fixture(89)
                .battlefield(P0, &[CardId::Mountain, CardId::Mountain, CardId::Forest])
                .battlefield(P1, &[CardId::GrizzlyBears])
                .hand(P0, &[CardId::Shock, CardId::Shock, CardId::GrizzlyBears])
                .library(
                    P0,
                    &[
                        CardId::Mountain,
                        CardId::Forest,
                        CardId::Mountain,
                        CardId::Forest,
                    ],
                )
                .library(P1, &[CardId::Island; 4])
                .hand(P1, &[CardId::Island])
                .life(P0, 2)
                .life(P1, 8)
                .mana(P0, pool(&[(ManaColor::Red, 2), (ManaColor::Green, 1)]))
                .main(P0, Step::PrecombatMain)
                .build(),
        ),
    ]
}

fn run_review(
    game: &Game,
    config: &SearchConfig,
    generation: u64,
) -> (PlayerView, RankedReview, SearchOutcome) {
    let view = game.view_for(P0, &[]);
    let knowledge = Knowledge { own_deck: None };
    let cancel = SearchCancellation::new();
    let outcome = search(game, &view, &knowledge, config, &cancel).expect("search runs");
    let binding =
        ReviewBinding::for_view(&view, generation, game.state.revision, game.state.decision);
    let review = rank(&view, &outcome.analysis, config, binding);
    (view, review, outcome)
}

fn json(review: &RankedReview) -> String {
    serde_json::to_string(review).expect("review serialises")
}

fn cmp_rank(a: &RankedCandidate, b: &RankedCandidate) -> Ordering {
    b.score
        .unwrap()
        .total_cmp(&a.score.unwrap())
        .then_with(|| b.chosen.cmp(&a.chosen))
        .then_with(|| b.scored_samples.cmp(&a.scored_samples))
        .then_with(|| a.offer_index.cmp(&b.offer_index))
}

/// Every field name `EvidenceRef::Budget` may name.
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

/// One reference resolves iff the thing it names exists in this payload — §4.7
/// "resolves", implemented so a wrong index or a stale count fails the test.
fn resolves(review: &RankedReview, view: &PlayerView, reference: &EvidenceRef) -> bool {
    match reference {
        EvidenceRef::EngineOffer { index } | EvidenceRef::EngineLabel { index } => {
            *index < view.legal.len()
        }
        EvidenceRef::SearchCount {
            offer_index,
            scored,
            without_result,
            refused,
        } => review
            .candidates
            .iter()
            .find(|candidate| candidate.offer_index == *offer_index)
            .map(|candidate| {
                candidate.scored_samples == *scored
                    && candidate.without_result == *without_result
                    && candidate.refused_times == *refused
            })
            .unwrap_or(false),
        // This module emits no explanation-backed text (rule text has exactly
        // one producer, the rules section), so such a reference has nothing to
        // resolve against and must never appear.
        EvidenceRef::CoachExplanation { .. } => false,
        EvidenceRef::RuleCitation { number } => view
            .legal
            .iter()
            .any(|legal| legal.rules.iter().any(|rule| *rule == number.as_ref())),
        EvidenceRef::Budget { field } => BUDGET_FIELDS.contains(&field.as_ref()),
        EvidenceRef::LineStep {
            offer_index,
            decision,
        } => review
            .candidates
            .iter()
            .find(|candidate| candidate.offer_index == *offer_index)
            .and_then(|candidate| candidate.line.as_ref())
            .map(|line| line.steps.iter().any(|step| step.decision == *decision))
            .unwrap_or(false),
        EvidenceRef::ViewDigest => review.binding.view_digest == view.digest,
    }
}

fn assert_all_text_traceable(review: &RankedReview, view: &PlayerView) {
    for (index, text) in review.notes.iter().enumerate() {
        assert!(
            !text.evidence.is_empty(),
            "note {index} (\"{}\") carries no evidence reference",
            text.body
        );
        for reference in &text.evidence {
            assert!(
                resolves(review, view, reference),
                "note {index} (\"{}\") cites {reference:?}, which does not resolve",
                text.body
            );
        }
        assert!(
            !text.body.contains('%'),
            "note {index} contains a percentage: {}",
            text.body
        );
    }
    for candidate in &review.candidates {
        for tag in &candidate.reasons {
            let text = reason_text(tag, candidate, &review.budget);
            assert!(
                !text.evidence.is_empty(),
                "reason {tag:?} on offer {} has no evidence",
                candidate.offer_index
            );
            for reference in &text.evidence {
                assert!(
                    resolves(review, view, reference),
                    "reason {tag:?} on offer {} cites {reference:?}, which does not resolve",
                    candidate.offer_index
                );
            }
            assert!(
                !text.body.contains('%'),
                "reason text contains a percentage: {}",
                text.body
            );
        }
    }
}

// ------------------------------------------------------- AC-1: hidden reads

/// §2.3 / AC-1, the operational definition of "no hidden reads": two real
/// engine states whose masked views for the reviewing seat are identical, but
/// which differ in an opponent's hand contents, an opponent's library order and
/// the viewer's own undeclared library order, must produce byte-identical
/// payloads.
#[test]
fn identical_masked_views_produce_byte_identical_payloads() {
    let base = || {
        common::fixture(97)
            .battlefield(P0, &[CardId::Forest, CardId::Mountain])
            .hand(P0, &[CardId::GrizzlyBears, CardId::Shock])
            .mana(P0, ready_pool())
            .main(P0, Step::PrecombatMain)
    };
    // World 1: P1 holds removal, and both libraries are in one order.
    let one = base()
        .hand(P1, &[CardId::SwordsToPlowshares, CardId::Island])
        .library(P1, &[CardId::Island, CardId::Mountain, CardId::Forest])
        .library(P0, &[CardId::Forest, CardId::Mountain, CardId::Island])
        .build();
    // World 2: different hidden hand identities and both library orders
    // reversed. Library *counts* and the viewer's own visible zones match.
    let two = base()
        .hand(P1, &[CardId::GrizzlyBears, CardId::Shock])
        .library(P1, &[CardId::Forest, CardId::Mountain, CardId::Island])
        .library(P0, &[CardId::Island, CardId::Mountain, CardId::Forest])
        .build();

    let view_one = one.view_for(P0, &[]);
    let view_two = two.view_for(P0, &[]);
    assert_eq!(
        view_one.digest, view_two.digest,
        "the fixture is wrong: the masked views differ, so this is not a hidden-read test"
    );

    let config = test_config();
    let (_, first, _) = run_review(&one, &config, 1);
    let (_, second, _) = run_review(&two, &config, 1);
    assert_eq!(
        json(&first),
        json(&second),
        "the payload changed although the only difference was hidden card identity or order"
    );
}

// --------------------------------------------- AC-5: byte reproducibility

#[test]
fn two_runs_and_one_out_of_process_run_agree_byte_for_byte() {
    let config = test_config();
    let (name, game) = &position_set()[2];

    let (view, first, _) = run_review(game, &config, 3);
    let (_, second, _) = run_review(game, &config, 3);
    assert_eq!(
        json(&first),
        json(&second),
        "{name}: two in-process runs of the same input differ"
    );

    // Round-trip: the corpus harness must be able to read the payload back.
    let round_tripped: RankedReview =
        serde_json::from_str(&json(&first)).expect("payload deserialises");
    assert_eq!(round_tripped, first, "{name}: round-trip lost information");
    assert_eq!(json(&round_tripped), json(&first));

    // And once more in a separate process, so the claim is not an artefact of
    // one interpreter state (AC-5's "once more out-of-process").
    let exe = std::env::current_exe().expect("test binary path");
    let output = Command::new(&exe)
        .args([
            "--exact",
            "subprocess_emit_ranking",
            "--include-ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()
        .expect("re-run this test binary");
    assert!(
        output.status.success(),
        "the child process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let child = stdout
        .split("REVIEW_JSON_BEGIN")
        .nth(1)
        .and_then(|rest| rest.split("REVIEW_JSON_END").next())
        .expect("the child printed its payload");
    assert_eq!(
        child,
        json(&first),
        "{name}: the out-of-process run produced different bytes"
    );
    assert_all_text_traceable(&first, &view);
}

/// Emits this crate's deterministic payload for the parent test to diff. It is
/// `#[ignore]`d so a normal `cargo test` run never re-enters it.
#[test]
#[ignore = "emits a payload for two_runs_and_one_out_of_process_run_agree_byte_for_byte"]
fn subprocess_emit_ranking() {
    let config = test_config();
    let (_, game) = &position_set()[2];
    let (_, review, _) = run_review(game, &config, 3);
    println!("REVIEW_JSON_BEGIN{}REVIEW_JSON_END", json(&review));
}

// ------------------------------------------------ AC-7: ties and the lattice

/// A synthetic analysis, so the tie arithmetic is tested directly rather than
/// through whatever sample counts one engine fixture happens to produce.
fn synthetic_analysis(view: &PlayerView, rows: &[(usize, f32, usize)]) -> SearchAnalysis {
    let candidates: Vec<CandidateScore> = rows
        .iter()
        .map(|(legal_index, mean, scored)| {
            let legal = &view.legal[*legal_index];
            CandidateScore {
                label: legal.label.clone(),
                action: legal.action.clone(),
                mean: if *scored == 0 { None } else { Some(*mean) },
                scored_samples: *scored,
                rollouts_without_result: 0,
                refused_times: 0,
                line: if *scored == 0 {
                    None
                } else {
                    Some(RecordedLine {
                        steps: vec![RecordedStep {
                            decision: 0,
                            seat: view.viewer,
                            label: legal.label.clone(),
                        }],
                        truncated: false,
                        seed_index: 0,
                        outcome: mean.round() as i8,
                    })
                },
            }
        })
        .collect();
    let chosen = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.mean.is_some())
        .max_by(|(_, a), (_, b)| {
            a.mean
                .partial_cmp(&b.mean)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.scored_samples.cmp(&b.scored_samples))
        })
        .map(|(index, _)| index);
    SearchAnalysis {
        candidates,
        decisions_used: 400,
        cancelled: false,
        fidelity: "test fidelity: neutral belief deck".to_string(),
        stop: StopReason::DeterminizationsExhausted,
        reproducible: true,
        chosen,
    }
}

fn synthetic_review(rows: &[(usize, f32, usize)]) -> (PlayerView, RankedReview) {
    let game = position_set().remove(0).1;
    let view = game.view_for(P0, &[]);
    let analysis = synthetic_analysis(&view, rows);
    let config = test_config();
    let binding = ReviewBinding::for_view(&view, 1, 0, 0);
    let review = rank(&view, &analysis, &config, binding);
    (view, review)
}

#[test]
fn one_lattice_step_is_the_tie_boundary() {
    // Two samples each: one step is 0.5, so +1.00 against +0.50 separates and
    // an exact match does not.
    let (_, separated) = synthetic_review(&[(0, 1.0, 2), (1, 0.5, 2)]);
    assert_eq!(separated.candidates[0].uncertainty, Uncertainty::Ordered);
    assert_eq!(separated.candidates[1].uncertainty, Uncertainty::Ordered);
    assert_eq!(
        separated.preference,
        Some(Preference::Unique {
            offer_index: separated.candidates[0].offer_index
        })
    );

    let (_, level) = synthetic_review(&[(0, 0.5, 2), (1, 0.5, 2)]);
    for candidate in &level.candidates {
        match &candidate.uncertainty {
            Uncertainty::EffectivelyTied { with } => {
                assert_eq!(with.len(), 1, "a two-row tie names exactly the other row");
            }
            other => panic!("a level pair must be explicitly tied, got {other:?}"),
        }
    }
    // Symmetric: each row names the other.
    let left = level.candidates[0].offer_index;
    let right = level.candidates[1].offer_index;
    assert_eq!(
        level.candidates[0].uncertainty,
        Uncertainty::EffectivelyTied { with: vec![right] }
    );
    assert_eq!(
        level.candidates[1].uncertainty,
        Uncertainty::EffectivelyTied { with: vec![left] }
    );
    assert!(matches!(
        level.preference,
        Some(Preference::AmongEquals { .. })
    ));
    assert!(
        level
            .candidates
            .iter()
            .all(|candidate| candidate.reasons.contains(&ReasonTag::EqualPracticeOutcome)),
        "tied rows must carry the equal-outcome tag, not a highest-outcome claim"
    );
    assert!(
        level.candidates.iter().all(|candidate| !candidate
            .reasons
            .contains(&ReasonTag::HighestPracticeOutcome)),
        "a tie must not be labelled as the highest outcome"
    );
}

#[test]
fn a_sub_lattice_difference_is_reported_as_a_tie_not_an_order() {
    // Two samples against four: the lattice step is 1/min(2,4) = 0.5, so +0.75
    // against +1.00 is inside one step and cannot be ordered.
    let (_, review) = synthetic_review(&[(0, 1.0, 2), (1, 0.75, 4)]);
    for candidate in &review.candidates {
        assert!(
            matches!(candidate.uncertainty, Uncertainty::EffectivelyTied { .. }),
            "a difference inside one lattice step must not be ordered: {:?}",
            candidate.uncertainty
        );
    }
    assert!(matches!(
        review.preference,
        Some(Preference::AmongEquals { .. })
    ));
}

#[test]
fn one_sample_is_its_own_uncertainty_state_and_claims_no_order() {
    let (_, review) = synthetic_review(&[(0, 1.0, 1), (1, -1.0, 1)]);
    assert!(
        review
            .candidates
            .iter()
            .all(|candidate| candidate.uncertainty == Uncertainty::SingleSample),
        "one sample cannot establish an order"
    );
    assert!(
        review
            .candidates
            .iter()
            .all(|candidate| candidate.reasons.contains(&ReasonTag::LowSample))
    );
    assert!(matches!(
        review.preference,
        Some(Preference::NoOrder { .. })
    ));
}

#[test]
fn a_single_sample_at_the_top_blocks_the_highest_outcome_claim() {
    // The leading row reached +1.00 on one sample; the only orderable row is at
    // +0.50. No row may be called the highest practice outcome here — that would
    // contradict the +1.00 row directly above it — and no order is claimed.
    let (_, review) = synthetic_review(&[(0, 1.0, 1), (1, 0.5, 2)]);
    assert!(matches!(
        review.preference,
        Some(Preference::NoOrder { .. })
    ));
    assert!(
        review.candidates.iter().all(|candidate| !candidate
            .reasons
            .contains(&ReasonTag::HighestPracticeOutcome)),
        "a row a single-sampled row beats is not the highest practice outcome"
    );
    assert_eq!(review.candidates[0].score, Some(1.0));
    assert_eq!(review.candidates[0].uncertainty, Uncertainty::SingleSample);
    assert_eq!(review.candidates[1].uncertainty, Uncertainty::Ordered);
}

#[test]
fn unscored_rows_are_listed_separately_and_never_ranked() {
    let (_, review) = synthetic_review(&[(0, 1.0, 2), (1, 0.5, 2), (2, 0.0, 0)]);
    let last = review.candidates.last().expect("candidates");
    assert_eq!(last.score, None);
    assert_eq!(last.uncertainty, Uncertainty::Unscored);
    assert_eq!(last.line, None, "a row with no scored sample has no line");
    assert!(last.reasons.contains(&ReasonTag::NoScoredSample));
    assert!(
        !last.chosen,
        "the search's selection is always a scored row"
    );
    assert!(
        review
            .candidates
            .windows(2)
            .all(|pair| !(pair[0].chosen && pair[1].score.is_none()))
    );
}

// -------------------------------------------- AC-9 / AC-10: nothing ranked

#[test]
fn a_typed_refusal_ranks_nothing() {
    // A mulligan decision is a pending choice: the search refuses it by design.
    let game = common::fixture(5)
        .hand(P0, &[CardId::Forest, CardId::Mountain])
        .hand(P1, &[CardId::Island, CardId::Island])
        .mulligan(P0)
        .build();
    let view = game.view_for(P0, &[]);
    let knowledge = Knowledge { own_deck: None };
    let cancel = SearchCancellation::new();
    let config = test_config();
    let outcome = search(&game, &view, &knowledge, &config, &cancel);
    assert!(
        outcome.is_err(),
        "a pending decision point must be refused by the search"
    );
    let reason = outcome.as_ref().err().map(|err| err.label());
    let binding = ReviewBinding::for_view(&view, 1, 0, 0);
    let review = rank_outcome(&view, &outcome, &config, binding);

    assert!(review.candidates.is_empty(), "a refusal ranks nothing");
    assert_eq!(review.preference, None, "a refusal claims no preference");
    assert_eq!(review.refusal, reason, "the typed reason is verbatim");
    assert!(review.refusal.is_some());
    assert_eq!(review.budget.stop, StopReason::NotStarted);
    assert!(!review.budget.reproducible);
    assert!(
        review
            .notes
            .iter()
            .any(|note| note.body.contains("no alternative is ranked")),
        "the refusal is stated in words, not left as an empty table"
    );
    assert_all_text_traceable(&review, &view);
}

#[test]
fn zero_scored_samples_rank_nothing_and_say_how_many_were_tried() {
    let (_, game) = &position_set()[1];
    let view = game.view_for(P0, &[]);
    let knowledge = Knowledge { own_deck: None };
    let cancel = SearchCancellation::new();
    // A zero per-rollout cap means every rollout ends without a result.
    let config = SearchConfig {
        seeds: 2,
        node_budget: 12_000,
        max_rollout_nodes: 0,
        max_millis: u64::MAX,
        max_candidates: 3,
    };
    let outcome = search(game, &view, &knowledge, &config, &cancel).expect("search runs");
    let binding = ReviewBinding::for_view(&view, 1, 0, 0);
    let review = rank(&view, &outcome.analysis, &config, binding);

    assert!(!review.candidates.is_empty(), "the offers are still listed");
    assert_eq!(review.preference, None, "no samples, no ordering");
    for candidate in &review.candidates {
        assert_eq!(candidate.score, None, "no placeholder mean");
        assert_eq!(candidate.uncertainty, Uncertainty::Unscored);
        assert_eq!(candidate.line, None);
        assert!(candidate.reasons.contains(&ReasonTag::NoScoredSample));
        assert!(
            candidate.rollouts_attempted > 0,
            "the attempts are counted, not hidden"
        );
        assert_eq!(
            candidate.rollouts_attempted,
            candidate.scored_samples + candidate.without_result + candidate.refused_times
        );
    }
    assert!(
        review
            .notes
            .iter()
            .any(|note| note.body.contains("reached no result")),
        "the absence of results is stated explicitly"
    );
    assert_all_text_traceable(&review, &view);
}

// ------------------------------------------------ AC-6: truncated analyses

#[test]
fn a_wall_clock_stop_is_not_reproducible_and_says_so() {
    let (_, game) = &position_set()[0];
    let view = game.view_for(P0, &[]);
    let knowledge = Knowledge { own_deck: None };
    let cancel = SearchCancellation::new();
    let config = SearchConfig {
        max_millis: 0,
        ..test_config()
    };
    let outcome = search(game, &view, &knowledge, &config, &cancel).expect("search runs");
    assert_eq!(
        outcome.analysis.stop,
        StopReason::WallClock,
        "a zero wall clock must report the wall-clock stop"
    );
    assert!(!outcome.analysis.reproducible);
    let binding = ReviewBinding::for_view(&view, 1, 0, 0);
    let review = rank(&view, &outcome.analysis, &config, binding);
    assert!(!review.budget.reproducible);
    assert_eq!(review.budget.stop, StopReason::WallClock);
    assert!(
        review
            .candidates
            .iter()
            .all(|candidate| candidate.reasons.contains(&ReasonTag::BudgetStopped)),
        "a truncated analysis labels every row it could not finish"
    );
    assert!(
        review
            .notes
            .iter()
            .any(|note| note.body.contains("not reproducible")),
        "the panel is told in words that the analysis is not reproducible"
    );
    assert!(review.budget.rendered().contains("reproducible: no"));
    assert_all_text_traceable(&review, &view);
}

#[test]
fn a_cancelled_analysis_is_not_reproducible() {
    let (_, game) = &position_set()[0];
    let view = game.view_for(P0, &[]);
    let knowledge = Knowledge { own_deck: None };
    let cancel = SearchCancellation::new();
    cancel.cancel();
    let config = test_config();
    let outcome = search(game, &view, &knowledge, &config, &cancel).expect("search runs");
    assert_eq!(outcome.analysis.stop, StopReason::Cancelled);
    assert!(outcome.analysis.cancelled);
    assert!(!outcome.analysis.reproducible);
    let binding = ReviewBinding::for_view(&view, 1, 0, 0);
    let review = rank(&view, &outcome.analysis, &config, binding);
    assert!(!review.budget.reproducible);
    assert!(review.budget.cancelled);
    assert!(review.budget.rendered().contains("cancellation observed"));
}

// --------------------------------------------------- AC-13: the budget record

#[test]
fn the_rendered_budget_line_is_a_pure_function_of_the_record() {
    let record = BudgetRecord {
        seeds: 2,
        node_budget: 30_000,
        max_rollout_nodes: 6_000,
        max_millis: 8_000,
        max_candidates: 6,
        decisions_used: 5_387,
        stop: StopReason::NodeBudget,
        cancelled: false,
        reproducible: true,
        fidelity: "fidelity line".to_string(),
    };
    let line = record.rendered();
    assert_eq!(
        line,
        "budget: 2 determinization seed(s) · 30000 engine decisions total · 6000 decisions per \
         rollout · 8000 ms wall clock · at most 6 engine-offered candidates · used 5387 · \
         stopped: node budget · reproducible: yes"
    );
    assert!(!line.contains('%'));

    let mutations: Vec<(&str, BudgetRecord)> = vec![
        (
            "seeds",
            BudgetRecord {
                seeds: 3,
                ..record.clone()
            },
        ),
        (
            "node_budget",
            BudgetRecord {
                node_budget: 30_001,
                ..record.clone()
            },
        ),
        (
            "max_rollout_nodes",
            BudgetRecord {
                max_rollout_nodes: 6_001,
                ..record.clone()
            },
        ),
        (
            "max_millis",
            BudgetRecord {
                max_millis: 8_001,
                ..record.clone()
            },
        ),
        (
            "max_candidates",
            BudgetRecord {
                max_candidates: 7,
                ..record.clone()
            },
        ),
        (
            "decisions_used",
            BudgetRecord {
                decisions_used: 5_388,
                ..record.clone()
            },
        ),
        (
            "stop",
            BudgetRecord {
                stop: StopReason::WallClock,
                ..record.clone()
            },
        ),
        (
            "reproducible",
            BudgetRecord {
                reproducible: false,
                ..record.clone()
            },
        ),
        (
            "cancelled",
            BudgetRecord {
                cancelled: true,
                ..record.clone()
            },
        ),
    ];
    for (field, mutated) in mutations {
        assert_ne!(
            mutated.rendered(),
            line,
            "mutating {field} left the rendered budget line unchanged"
        );
    }

    // An unbounded wall clock is spelled out, not printed as a giant integer.
    let unbounded = BudgetRecord {
        max_millis: u64::MAX,
        ..record.clone()
    };
    assert!(unbounded.rendered().contains("unbounded wall clock"));
    let encoded = serde_json::to_string(&unbounded).expect("serialises");
    assert!(encoded.contains("\"unbounded\""));
    let decoded: BudgetRecord = serde_json::from_str(&encoded).expect("deserialises");
    assert_eq!(decoded, unbounded);
    assert_eq!(decoded.rendered(), unbounded.rendered());
}

#[test]
fn the_recorded_budget_is_the_declared_measured_default_with_no_wall_clock() {
    let recorded = recorded_config();
    assert_eq!(recorded.max_millis, u64::MAX);
    assert_eq!(recorded.seeds, 2);
    assert_eq!(recorded.node_budget, 30_000);
    assert_eq!(recorded.max_rollout_nodes, 6_000);
    assert_eq!(recorded.max_candidates, 6);
}

// ----------------------------------- AC-15 + the position set + AC-4 already

#[test]
fn every_position_in_the_set_ranks_legally_and_traceably() {
    let config = test_config();
    let mut scored_rows = 0usize;
    let mut lines_seen = 0usize;
    let mut ordered_rows = 0usize;
    let mut tied_groups = 0usize;
    for (name, game) in position_set() {
        let (view, review, outcome) = run_review(&game, &config, 7);

        // The payload is well formed and the budget is the budget.
        assert_eq!(review.spec, "move-review/1.2");
        assert_eq!(review.refusal, None, "{name}: search ran, so no refusal");
        assert_eq!(review.budget.stop, outcome.analysis.stop);
        assert_eq!(
            review.budget.decisions_used,
            outcome.analysis.decisions_used
        );
        assert!(
            review.budget.decisions_used <= config.node_budget,
            "{name}: the analysis spent more decisions than the budget allowed"
        );
        assert!(
            review.budget.reproducible,
            "{name}: a node-bounded run with no wall clock must be reproducible"
        );
        assert!(
            matches!(
                review.budget.stop,
                StopReason::DeterminizationsExhausted | StopReason::NodeBudget
            ),
            "{name}: unexpected stop {:?}",
            review.budget.stop
        );

        // Offers are the engine's, verbatim: identity is index plus label.
        for candidate in &review.candidates {
            let legal = view.legal.get(candidate.offer_index).unwrap_or_else(|| {
                panic!(
                    "{name}: offer_index {} is out of range",
                    candidate.offer_index
                )
            });
            assert_eq!(candidate.move_label, legal.label);
            assert_eq!(
                candidate.offer_rules,
                legal
                    .rules
                    .iter()
                    .map(|rule| std::borrow::Cow::Borrowed(*rule))
                    .collect::<Vec<_>>(),
                "{name}: rule citations are not the engine's own"
            );
            assert_eq!(
                candidate.rollouts_attempted,
                candidate.scored_samples + candidate.without_result + candidate.refused_times
            );
            // AC-15: a line exists exactly when a rollout reached a result, and
            // it is a prefix of recorded engine-submitted actions.
            match (&candidate.line, candidate.scored_samples) {
                (None, 0) => {}
                (None, _) => panic!(
                    "{name}: offer {} has {} scored sample(s) but no recorded line",
                    candidate.offer_index, candidate.scored_samples
                ),
                (Some(_), 0) => panic!(
                    "{name}: offer {} has a line but no scored sample",
                    candidate.offer_index
                ),
                (Some(line), _) => {
                    lines_seen += 1;
                    assert!(line.steps.len() <= MAX_LINE_PLIES);
                    assert_eq!(
                        line.truncated,
                        line.steps.len() == MAX_LINE_PLIES,
                        "{name}: the truncation flag disagrees with the recorded length"
                    );
                    let root = &line.steps[0];
                    assert_eq!(root.decision, 0, "{name}: the line must start at the root");
                    assert_eq!(
                        root.seat, view.viewer,
                        "{name}: the root action belongs to the reviewing seat"
                    );
                    assert!(
                        (-1..=1).contains(&line.outcome),
                        "{name}: an outcome outside {{-1, 0, +1}} was recorded"
                    );
                    // Every step's decision ordinal is its position, and every
                    // label is one the engine itself produced at that decision.
                    for (ordinal, step) in line.steps.iter().enumerate() {
                        assert_eq!(step.decision, ordinal);
                        assert!(!step.label.is_empty(), "{name}: an empty label");
                    }
                    for (index, step) in line.steps.iter().enumerate() {
                        assert!(
                            resolves(
                                &review,
                                &view,
                                &EvidenceRef::LineStep {
                                    offer_index: candidate.offer_index,
                                    decision: step.decision
                                }
                            ),
                            "{name}: line step {index} does not resolve"
                        );
                    }
                }
            }
            // Uncertainty must agree with the counts (§4.5).
            match candidate.scored_samples {
                0 => assert_eq!(candidate.uncertainty, Uncertainty::Unscored),
                1 => assert_eq!(candidate.uncertainty, Uncertainty::SingleSample),
                samples => {
                    assert!(samples >= 2);
                    match &candidate.uncertainty {
                        Uncertainty::Ordered => {}
                        Uncertainty::EffectivelyTied { with } => {
                            assert!(!with.is_empty());
                            for other in with {
                                let other_row = review
                                    .candidates
                                    .iter()
                                    .find(|row| row.offer_index == *other)
                                    .expect("a tie names a real row");
                                assert!(
                                    matches!(
                                        other_row.uncertainty,
                                        Uncertainty::EffectivelyTied { .. }
                                    ),
                                    "{name}: the tie is not symmetric"
                                );
                                let min = candidate.scored_samples.min(other_row.scored_samples);
                                assert!(
                                    (candidate.score.unwrap() - other_row.score.unwrap()).abs()
                                        < 1.0 / min as f32,
                                    "{name}: a separable pair was reported as tied"
                                );
                            }
                        }
                        other => panic!("{name}: unexpected uncertainty {other:?}"),
                    }
                }
            }
            scored_rows += usize::from(candidate.scored_samples > 0);
        }

        // The §4.2 order: scored rows first, in the total order; unscored rows
        // after them in offer order, never interleaved.
        let first_unscored = review
            .candidates
            .iter()
            .position(|candidate| candidate.score.is_none());
        if let Some(start) = first_unscored {
            assert!(
                review.candidates[start..]
                    .iter()
                    .all(|candidate| candidate.score.is_none()),
                "{name}: an unscored row is interleaved into the ordering"
            );
            let unscored: Vec<usize> = review.candidates[start..]
                .iter()
                .map(|candidate| candidate.offer_index)
                .collect();
            let mut sorted = unscored.clone();
            sorted.sort_unstable();
            assert_eq!(
                unscored, sorted,
                "{name}: unscored rows are not in offer order"
            );
        }
        for pair in review.candidates.windows(2) {
            if pair[0].score.is_some() && pair[1].score.is_some() {
                assert_ne!(
                    cmp_rank(&pair[0], &pair[1]),
                    Ordering::Greater,
                    "{name}: offers {} and {} are out of order",
                    pair[0].offer_index,
                    pair[1].offer_index
                );
            }
        }

        // The search's own selection is one of the ranked rows and is scored.
        if let Some(chosen) = review.candidates.iter().find(|candidate| candidate.chosen) {
            assert!(
                chosen.scored_samples > 0,
                "{name}: the search selected a row with no scored sample"
            );
        } else {
            assert_eq!(
                review.preference, None,
                "{name}: no selection must mean no preference"
            );
        }

        // The preference must agree with the order it heads (§4.2 point 2).
        match &review.preference {
            None => assert!(
                review
                    .candidates
                    .iter()
                    .all(|candidate| candidate.score.is_none()),
                "{name}: a claimed ranking without a preference"
            ),
            Some(Preference::Unique { offer_index }) => {
                let head = &review.candidates[0];
                assert_eq!(head.offer_index, *offer_index);
                assert_eq!(head.uncertainty, Uncertainty::Ordered);
                assert!(
                    head.chosen,
                    "{name}: the unique maximum is the search's own choice"
                );
            }
            Some(Preference::AmongEquals {
                chosen,
                offer_indices,
            }) => {
                assert!(offer_indices.len() >= 2);
                assert!(
                    offer_indices.contains(&review.candidates[0].offer_index),
                    "{name}: the tie group does not contain the leading row"
                );
                assert!(
                    offer_indices.contains(chosen),
                    "{name}: the search's own choice is outside the group it heads"
                );
                assert_eq!(
                    review.candidates[0].uncertainty,
                    Uncertainty::EffectivelyTied {
                        with: offer_indices
                            .iter()
                            .copied()
                            .filter(|index| *index != review.candidates[0].offer_index)
                            .collect::<Vec<_>>()
                    }
                );
                tied_groups += 1;
            }
            Some(Preference::NoOrder { chosen }) => {
                assert_eq!(
                    review.candidates[0].uncertainty,
                    Uncertainty::SingleSample,
                    "{name}: no order is claimed only when the head is a single sample"
                );
                assert_eq!(
                    *chosen,
                    review
                        .candidates
                        .iter()
                        .find(|c| c.chosen)
                        .map(|c| c.offer_index)
                );
            }
        }

        for candidate in &review.candidates {
            ordered_rows += usize::from(candidate.uncertainty == Uncertainty::Ordered);
        }

        // No invented measurement anywhere (AC-17) and no percentage (AC-2).
        let payload = json(&review);
        for forbidden in [
            "centipawn",
            "winrate",
            "win rate",
            "probability",
            "\"depth\"",
        ] {
            assert!(
                !payload.contains(forbidden),
                "{name}: the payload contains {forbidden}"
            );
        }
        assert!(
            !payload.contains('%'),
            "{name}: the payload contains a percentage"
        );
        assert_all_text_traceable(&review, &view);

        // Round-trip parity, which the corpus harness depends on.
        let round_tripped: RankedReview = serde_json::from_str(&payload).expect("round-trips");
        assert_eq!(round_tripped, review);

        println!(
            "{name}: {} offer(s), stop {:?}, used {} decision(s), preference {:?}",
            review.candidates.len(),
            review.budget.stop,
            review.budget.decisions_used,
            review.preference
        );
        for candidate in &review.candidates {
            println!(
                "  offer {} ({}) score {:?} over {} sample(s) — {:?}{}",
                candidate.offer_index,
                candidate.move_label,
                candidate.score,
                candidate.scored_samples,
                candidate.uncertainty,
                if candidate.chosen { " [chosen]" } else { "" }
            );
        }
    }
    assert!(
        scored_rows > 0,
        "no position in the set produced a scored sample, so nothing was ranked"
    );
    assert!(
        lines_seen > 0,
        "no position in the set produced a recorded line, so AC-15 was not exercised"
    );
    assert!(
        ordered_rows > 0,
        "no position in the set produced an ordered (non-tied) row, so the ordering claim was \
         never exercised on real engine data"
    );
    assert!(
        tied_groups > 0,
        "no position in the set produced an explicit tie, so §4.3 was never exercised"
    );
}

// ------------------------------------------------- AC-8-adjacent: one offer

#[test]
fn a_single_offered_action_is_labelled_and_not_ordered() {
    // The ranking layer ranks the offer it is given. Reducing the offer to the
    // engine's first entry exercises the single-offer branch with a genuine
    // engine-produced `LegalAction`.
    let (_, game) = &position_set()[0];
    let mut view = game.view_for(P0, &[]);
    assert!(view.legal.len() > 1, "the fixture needs a real offer");
    view.legal.truncate(1);
    let analysis = synthetic_analysis(&view, &[(0, 1.0, 2)]);
    let config = test_config();
    let binding = ReviewBinding::for_view(&view, 1, 0, 0);
    let review = rank(&view, &analysis, &config, binding);
    assert_eq!(review.candidates.len(), 1);
    assert!(
        review.candidates[0]
            .reasons
            .contains(&ReasonTag::SoleLegalOffer)
    );
    let text = reason_text(
        &review.candidates[0].reasons[0],
        &review.candidates[0],
        &review.budget,
    );
    assert_eq!(
        text.kind,
        TextKind::Policy,
        "an offer-shape fact is not rule text: it cites no comprehensive-rule number, so it must \
         not enter the rules section (D-9)"
    );
    assert!(text.body.contains("exactly one action"));
}

// ----------------------------------------- AC-2/AC-17: the honesty envelope

#[test]
fn no_review_text_claims_a_probability_a_verdict_or_an_invented_measure() {
    let config = test_config();
    for (name, game) in position_set() {
        let (view, review, _) = run_review(&game, &config, 11);
        let mut bodies: Vec<String> = review.notes.iter().map(|note| note.body.clone()).collect();
        for candidate in &review.candidates {
            for tag in &candidate.reasons {
                bodies.push(reason_text(tag, candidate, &review.budget).body);
            }
        }
        for body in bodies {
            let lower = body.to_lowercase();
            for forbidden in [
                "%",
                "probability",
                "chance",
                "odds",
                "equity",
                "centipawn",
                "blunder",
                "mistake",
                "inaccuracy",
                "best move",
                "should have",
            ] {
                assert!(
                    !lower.contains(forbidden),
                    "{name}: review text contains {forbidden:?}: {body}"
                );
            }
            // Evidence is required on every sentence the layer emits.
            assert!(!body.is_empty());
        }
        assert_all_text_traceable(&review, &view);
    }
}
