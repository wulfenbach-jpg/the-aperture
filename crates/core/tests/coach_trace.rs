//! Issue19 guided walkthrough: the coach teaches the pass round, the response
//! window, APNAP choices and what resolves next, all derived from the typed
//! engine state and pinned rule text — never invented prose.
//!
//! The three explanation categories (Rule / Simplification / Strategy) must be
//! separable, and the APNAP privacy boundary (another seat's pending choice is
//! never described) must survive the new labeling.

#![allow(dead_code)]

mod common;

use aperture_core::*;
use aperture_core::{P0, P1, Step, coach};
use common::*;

fn cast_to_pending(game: &mut aperture_core::Game, caster: aperture_core::PlayerId) {
    let crack = hand_card(game, caster, CardId::CrackTheEarth);
    submit_ok(
        game,
        caster,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    );
    let mut guard = 0;
    while !matches!(game.state.pending, Some(Pending::ApnapSacrifice { .. })) {
        let owner = game.decision_owner().expect("priority while resolving");
        submit_ok(game, owner, Action::Pass);
        guard += 1;
        assert!(guard < 16, "the spell never reached its APNAP queue");
    }
}

#[test]
fn the_1014_note_states_apnap_order() {
    let note = coach::rule_note("101.4").expect("101.4 has a note");
    assert!(
        note.contains("turn order"),
        "the APNAP note must state turn-order sequencing: {note}"
    );
}

fn cast_crack(game: &mut aperture_core::Game) {
    let crack = hand_card(game, P0, CardId::CrackTheEarth);
    submit_ok(
        game,
        P0,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    );
}

fn crack_stage() -> aperture_core::Game {
    fixture_n(402, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build()
}

#[test]
fn walkthrough_new_game_pass_round_states_the_next_resolution() {
    // Empty stack: everyone passing ends the step (CR 117.4).
    let game = fixture_n(401, 3).main(P0, Step::PrecombatMain).build();
    let legal = game.legal_actions();
    let explanations = coach::explain(&game, P0, &legal);
    let round = explanations
        .iter()
        .find(|e| e.topic == "pass round")
        .expect("pass-round explanation");
    assert!(
        round.text.contains("No one has passed yet"),
        "fresh state has a clean round: {}",
        round.text
    );
    assert!(
        round.text.contains("P0 has the next response"),
        "the current response holder is named: {}",
        round.text
    );
    assert!(
        round.text.contains("the current step/phase ends next"),
        "an empty stack means the step ends: {}",
        round.text
    );
    assert_eq!(round.kind, coach::ExplanationKind::Rule);
    assert!(round.rules.contains(&"117.3d") && round.rules.contains(&"117.4"));
}

#[test]
fn walkthrough_cast_opens_a_response_window_and_names_what_resolves() {
    let mut game = crack_stage();
    cast_crack(&mut game);
    assert!(game.decision_owner().is_some(), "caster regains priority");
    let legal = game.legal_actions();
    let explanations = coach::explain(&game, P0, &legal);
    let window = explanations
        .iter()
        .find(|e| e.topic == "response window")
        .expect("a casting opens a response window");
    assert!(
        window.text.contains("Open response window") && window.text.contains("Crack the Earth"),
        "the window names the top object: {}",
        window.text
    );
    assert_eq!(window.kind, coach::ExplanationKind::Rule);
    let round = explanations
        .iter()
        .find(|e| e.topic == "pass round")
        .expect("pass-round explanation");
    assert!(
        round.text.contains("Crack the Earth resolves next"),
        "the top stack object is what resolves next: {}",
        round.text
    );
}

#[test]
fn walkthrough_consecutive_passes_are_listed_seat_by_seat() {
    let mut game = crack_stage();
    cast_crack(&mut game);
    // P0 passes; the response moves to P1 (CR 117.3d).
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.decision_owner(), Some(P1));
    let legal = game.legal_actions();
    let explanations = coach::explain(&game, P1, &legal);
    let round = explanations
        .iter()
        .find(|e| e.topic == "pass round")
        .expect("pass-round explanation");
    assert!(
        round.text.contains("Consecutive passes this round: P0."),
        "the pass list is named: {}",
        round.text
    );
    assert!(
        round.text.contains("Still to respond: P1, P2."),
        "remaining responders are named: {}",
        round.text
    );
    assert!(
        round.text.contains("Crack the Earth resolves next"),
        "what resolves next is named: {}",
        round.text
    );
}

#[test]
fn walkthrough_apnap_pending_teaches_apnap_order_and_keeps_private_choices_private() {
    let mut game = crack_stage();
    cast_to_pending(&mut game, P0);
    let chooser = match &game.state.pending {
        Some(Pending::ApnapSacrifice { player, .. }) => *player,
        other => panic!("expected an APNAP queue, got {other:?}"),
    };
    let legal = game.legal_actions();
    let explanations = coach::explain(&game, chooser, &legal);
    let pending = explanations
        .iter()
        .find(|e| e.topic == "pending choice")
        .expect("pending-choice explanation");
    assert!(
        pending.text.contains("APNAP order"),
        "the pending explanation teaches APNAP order: {}",
        pending.text
    );

    // A non-entitled viewer is not told what the choice is, but may be told
    // that APNAP ordering governs the queue.
    let other = 1 - chooser;
    let legal = game.legal_actions();
    let explanations = coach::explain(&game, other, &legal);
    let pending = explanations
        .iter()
        .find(|e| e.topic == "pending choice")
        .expect("pending-choice explanation for the other seat");
    assert!(
        !pending.text.contains("choose a permanent to sacrifice"),
        "another seat's private choice label must not leak: {}",
        pending.text
    );
    assert!(
        pending.text.contains("APNAP order"),
        "the APNAP ordering note stays visible without private detail: {}",
        pending.text
    );
}

#[test]
fn walkthrough_categories_are_separate_and_strategy_is_absent_by_design() {
    let mut game = crack_stage();
    cast_crack(&mut game);
    let legal = game.legal_actions();
    let explanations = coach::explain(&game, P0, &legal);
    assert!(
        explanations
            .iter()
            .any(|e| e.kind == coach::ExplanationKind::Rule),
        "rule explanations exist"
    );
    // The Simplification records visibly declare themselves; nothing
    // strategy-labeled claims to be a rule.
    for explanation in &explanations {
        match explanation.kind {
            coach::ExplanationKind::Simplification => assert!(
                explanation.text.contains("simplification"),
                "a simplification record must disclose itself: {}",
                explanation.text
            ),
            coach::ExplanationKind::Strategy => assert!(
                explanation.text.contains("No strategic suggestions"),
                "strategy records state their non-rule nature: {}",
                explanation.text
            ),
            coach::ExplanationKind::Rule => {}
        }
    }
    let strategy = explanations
        .iter()
        .find(|e| e.kind == coach::ExplanationKind::Strategy)
        .expect("an explicit strategy-absence record");
    assert!(
        strategy.rules.is_empty(),
        "no rule citation backs strategy text: {strategy:?}"
    );
}

#[test]
fn coaching_of_a_non_decision_seat_never_describes_anothers_hand() {
    let game = fixture_n(406, 4)
        .hand(P1, &[CardId::GrizzlyBears, CardId::HillGiant])
        .main(P1, Step::PrecombatMain)
        .build();
    let _ = P1;
    let p0_view = game.view_for(P0, &[]);
    let p0_explain = coach::explain(&game, P0, &p0_view.legal);
    assert!(
        p0_explain
            .iter()
            .all(|e| !e.text.contains("Grizzly Bears") && !e.text.contains("Hill Giant")),
        "P0's coaching must not describe P1's hand: {p0_explain:?}"
    );
}
