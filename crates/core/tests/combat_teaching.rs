//! Coaching must state CR 510.1c correctly: a blocked creature's controller
//! divides its combat damage among the blockers as they choose. The engine's
//! automatic lethal-first assignment is a bounded slice simplification and must
//! be labeled as such, never taught as the rule (supervisor acceptance fix 7).

mod common;

use aperture_core::{P0, P1, Step, coach};
use common::*;

#[test]
fn the_5101c_note_does_not_teach_lethal_first_as_the_rule() {
    let note = coach::rule_note("510.1c").expect("510.1c has a note");
    assert!(
        note.contains("as its controller chooses"),
        "the note must state the division is the controller's choice: {note}"
    );
    assert!(
        !note.contains("lethal first"),
        "the pinned 2026 rule has no lethal-first requirement: {note}"
    );
}

#[test]
fn combat_coaching_identifies_the_automatic_assignment_as_a_simplification() {
    let mut game = fixture(7)
        .battlefield(P0, &[aperture_core::CardId::GrizzlyBears])
        .battlefield(P1, &[aperture_core::CardId::HillGiant])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attacker = battlefield_card(&game, P0, aperture_core::CardId::GrizzlyBears);
    let blocker = battlefield_card(&game, P1, aperture_core::CardId::HillGiant);
    game.state.turn.attackers = vec![attacker];
    game.state.turn.blockers = vec![(blocker, attacker)];

    let legal = game.legal_actions();
    let explanations = coach::explain(&game, P0, &legal);
    let assignment = explanations
        .iter()
        .find(|e| e.topic == "combat damage assignment")
        .unwrap_or_else(|| {
            panic!(
                "combat coaching must disclose the automatic assignment: {:?}",
                explanations.iter().map(|e| &e.topic).collect::<Vec<_>>()
            )
        });
    assert!(
        assignment.text.contains("simplification"),
        "the disclosure must label the behavior a simplification: {}",
        assignment.text
    );
    assert!(
        assignment.rules.contains(&"510.1c"),
        "the disclosure must cite CR 510.1c: {:?}",
        assignment.rules
    );
}
