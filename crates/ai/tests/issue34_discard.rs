//! #34 D1 — the policy's hand-choice answer must be one of the engine's offers.
//!
//! Before the fix the engine offered a single hard-coded "last N cards of hand"
//! set while the heuristic answered with its own ranked set. The answer was
//! legal in rules terms but absent from the offer, so the UI dropped it and the
//! table waited at the cleanup step forever.

mod common;

use aperture_ai::{ChoiceAnswer, DecisionBinding, DecisionEnvelope, HeuristicPolicy, Policy};
use aperture_core::view::PendingKind;
use aperture_core::*;
use common::*;

/// Eleven cards, so cleanup discards `count == 2` (CR 514.1).
fn full_hand() -> [CardId; 11] {
    [
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Shock,
        CardId::Shock,
        CardId::Forest,
        CardId::Forest,
    ]
}

fn reach_cleanup_discard(seed: u64) -> Game {
    let mut game = fixture(seed)
        .hand(P0, &full_hand())
        .main(P0, Step::PrecombatMain)
        .build();
    let mut log = Vec::new();
    let reached = run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::Discard { .. })),
        64,
    );
    assert!(reached, "the cleanup discard was never reached");
    game
}

#[test]
fn the_heuristic_discards_from_the_engine_offer() {
    let mut game = reach_cleanup_discard(3401);
    let view = common::view(&game, P0);
    let count = match view.pending_kind {
        Some(PendingKind::Discard { count }) => count,
        other => panic!("expected a cleanup discard, got {other:?}"),
    };
    assert_eq!(
        count as usize,
        view.own_hand.len() - 7,
        "cleanup discards exactly down to the seven-card maximum (CR 514.1)"
    );
    let mut policy = HeuristicPolicy::new();
    let decision = policy.decide(&view);
    assert!(
        view.legal
            .iter()
            .any(|legal| legal.action == decision.action),
        "the heuristic answered {:?}, which is not one of the {} offered action(s)",
        decision.action,
        view.legal.len()
    );
    // The engine accepts exactly the offered answer, and the step completes.
    submit_ok(&mut game, P0, decision.action);
    assert!(
        game.state.pending.is_none(),
        "the accepted discard must finish the cleanup step"
    );
}

#[test]
fn the_decider_envelope_offers_each_legal_discard_set() {
    let game = reach_cleanup_discard(3401);
    let view = common::view(&game, P0);
    let binding = DecisionBinding {
        viewer: P0,
        revision: game.revision(),
        decision: game.decision(),
        generation: 9,
    };
    let envelope =
        DecisionEnvelope::prepare(&view, binding).expect("a real choice is a real question");
    assert_eq!(
        envelope.request().options.len(),
        view.legal.len(),
        "every offered set must be its own option"
    );
    assert!(
        envelope.request().options.len() >= 2,
        "a bounded offer still has to be a choice"
    );
    let mut labels: Vec<String> = envelope
        .request()
        .options
        .iter()
        .map(|option| option.label.clone())
        .collect();
    let offered_labels = labels.len();
    labels.sort();
    labels.dedup();
    assert_eq!(
        labels.len(),
        offered_labels,
        "two different sets must not share one label"
    );
    for option in envelope.request().options.clone() {
        let answer = ChoiceAnswer {
            key: option.key.clone(),
            label: option.label.clone(),
            answer_confidence: None,
            confidence: None,
            model: None,
        };
        let proposal = envelope
            .resolve(&answer)
            .expect("an offered key must resolve");
        assert!(
            view.legal
                .iter()
                .any(|legal| legal.action == proposal.action),
            "a resolved proposal must be an offered action: {:?}",
            proposal.action
        );
    }
    let outside = ChoiceAnswer {
        key: "zzz-not-offered".to_string(),
        label: "not an offer".to_string(),
        answer_confidence: None,
        confidence: None,
        model: None,
    };
    assert!(
        envelope.resolve(&outside).is_err(),
        "a key outside the offer must be refused"
    );
}
