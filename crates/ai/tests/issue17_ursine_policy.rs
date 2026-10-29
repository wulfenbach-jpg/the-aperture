use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::*;

#[test]
fn heuristic_obeys_ursine_attack_requirement_from_public_offer() {
    let deck = DeckList::parse_practice(
        "ursine-package",
        include_str!("../../../decks/package-slice54.txt"),
    )
    .unwrap();
    let decks = vec![deck; 4];
    let commanders: Vec<_> = (0..4).map(|p| (p, CardId::JasmineBoreal)).collect();
    let (mut g, mut events) = Game::new_commander_combat_logged(2323, &decks, &commanders).unwrap();
    let mut policy = HeuristicPolicy::new();
    let mut forced = 0;
    for step in 0..10000 {
        if g.state.is_over() {
            break;
        }
        let p = g.decision_owner().unwrap();
        let view = g.view_for(p, &events);
        let action = policy.decide(&view).action;
        if matches!(g.state.pending, Some(Pending::DeclareAttackers { .. }))
            && g.state
                .effects
                .iter()
                .any(|e| e.kind == ContinuousKind::MustAttackThisCombat)
        {
            forced += 1;
        }
        assert!(
            g.validate(p, &action).is_ok(),
            "heuristic proposed illegal action at step{step}: {action:?}: {:?}",
            g.validate(p, &action)
        );
        events.extend(g.submit_action(p, action).unwrap());
    }
    assert!(
        g.state.is_over(),
        "real default-policy game must terminate, not cap"
    );
    assert!(forced > 0, "actual Ursine forced attack must be reached");
}
