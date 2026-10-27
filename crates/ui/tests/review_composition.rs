//! Canonical rank -> honesty audit -> presentation regressions for #28.
#[path = "../../ai/tests/common.rs"]
mod common;

use aperture_ai::review::{RankedReview, ReviewBinding, Uncertainty, rank};
use aperture_ai::{
    CandidateScore, RecordedLine, RecordedStep, SearchAnalysis, SearchConfig, StopReason,
};
use aperture_core::state::Step;
use aperture_core::*;
use aperture_ui::review::MoveReview;

fn review(rows: &[(usize, f32, usize)]) -> RankedReview {
    let game = common::fixture(11)
        .battlefield(P0, &[CardId::Forest, CardId::Mountain, CardId::Island])
        .hand(P0, &[CardId::GrizzlyBears, CardId::Shock, CardId::Forest])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Island; 6])
        .main(P0, Step::PrecombatMain)
        .build();
    let view = game.view_for(P0, &[]);
    let candidates = rows
        .iter()
        .map(|(index, mean, samples)| {
            let offer = &view.legal[*index];
            CandidateScore {
                label: offer.label.clone(),
                action: offer.action.clone(),
                mean: Some(*mean),
                scored_samples: *samples,
                rollouts_without_result: 0,
                refused_times: 0,
                line: Some(RecordedLine {
                    steps: vec![RecordedStep {
                        decision: 0,
                        seat: view.viewer,
                        label: offer.label.clone(),
                    }],
                    truncated: false,
                    seed_index: 0,
                    outcome: mean.round() as i8,
                }),
            }
        })
        .collect::<Vec<_>>();
    let chosen = candidates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.mean
                .partial_cmp(&b.mean)
                .unwrap()
                .then(a.scored_samples.cmp(&b.scored_samples))
        })
        .map(|(index, _)| index);
    let analysis = SearchAnalysis {
        candidates,
        decisions_used: 400,
        cancelled: false,
        fidelity: "test fidelity: neutral belief deck".into(),
        stop: StopReason::DeterminizationsExhausted,
        reproducible: true,
        chosen,
    };
    let config = SearchConfig {
        seeds: 4,
        node_budget: 12_000,
        max_rollout_nodes: 2_000,
        max_millis: u64::MAX,
        max_candidates: 4,
    };
    rank(
        &view,
        &analysis,
        &config,
        ReviewBinding::for_view(&view, 1, 0, 0),
    )
}

fn rendered_once(payload: RankedReview) -> String {
    let candidates = payload.candidates.clone();
    let violations = aperture_ai::review::honesty::audit_review(&payload);
    assert!(
        violations.is_empty(),
        "canonical payload rejected: {violations:#?}"
    );
    let shown = MoveReview::from_ranked(payload)
        .policy_section()
        .lines
        .join("\n");
    for row in &candidates {
        assert_eq!(
            shown
                .matches(&format!("[offer {}]", row.offer_index))
                .count(),
            1,
            "{shown}"
        );
        // Each candidate's own teaching note and recorded line survive rendering.
        assert!(shown.contains(&row.move_label), "{shown}");
        assert!(
            shown.contains(&row.line.as_ref().unwrap().steps[0].label),
            "{shown}"
        );
    }
    assert_eq!(
        shown.matches("teaching note").count(),
        candidates
            .iter()
            .filter(|row| !row.reasons.is_empty())
            .count(),
        "{shown}"
    );
    assert_eq!(
        shown.matches("    line: decision").count(),
        candidates.len(),
        "{shown}"
    );
    shown
}

#[test]
fn canonical_unequal_tie_passes_audit_without_claiming_equal_scores() {
    let shown = rendered_once(review(&[(0, 1.0, 2), (1, 0.75, 4)]));
    assert!(
        shown.contains("the search's own choice among effectively tied offers"),
        "{shown}"
    );
    assert!(!shown.contains("equally scored"), "{shown}");
    assert!(shown.contains("mean +1.00") && shown.contains("mean +0.75"));
}

#[test]
fn interleaved_single_sample_survives_canonical_pipeline() {
    let shown = rendered_once(review(&[(0, 1.0, 2), (1, 1.0, 1), (2, 0.75, 4)]));
    assert!(
        shown.contains("one scored sample — not ordered against other rows"),
        "{shown}"
    );
}

#[test]
fn overlapping_ties_preserve_pairwise_facts_without_transitive_grouping() {
    let payload = review(&[(0, 1.0, 4), (1, 2.0 / 3.0, 3), (2, 0.5, 4)]);
    assert_eq!(
        payload.candidate(0).unwrap().uncertainty,
        Uncertainty::EffectivelyTied { with: vec![1] }
    );
    assert_eq!(
        payload.candidate(1).unwrap().uncertainty,
        Uncertainty::EffectivelyTied { with: vec![0, 2] }
    );
    assert_eq!(
        payload.candidate(2).unwrap().uncertainty,
        Uncertainty::EffectivelyTied { with: vec![1] }
    );
    let shown = rendered_once(payload);
    assert!(
        shown.contains("pairwise effective ties: offer 0 ↔ offer 1; offer 1 ↔ offer 2"),
        "{shown}"
    );
    assert!(!shown.contains("offer 0 ↔ offer 2"), "{shown}");
}

#[test]
fn canonical_exact_tie_separated_and_single_sample_controls_pass() {
    for rows in [
        vec![(0, 1.0, 2), (1, 1.0, 2)],
        vec![(0, 1.0, 2), (1, 0.5, 2)],
        vec![(0, 1.0, 1), (1, 0.5, 2)],
    ] {
        rendered_once(review(&rows));
    }
}
