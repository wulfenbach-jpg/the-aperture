//! Private top-card eligibility must not change another seat's projection.
mod common;
use aperture_core::*;
use common::*;
fn waiting(top: CardId) -> Game {
    let mut b = vec![CardId::ViviensGrizzly];
    b.extend([CardId::Forest; 8]);
    let mut g = fixture(2098)
        .battlefield(P0, &b)
        .library(P0, &[top, CardId::Forest])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = g
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == CardId::ViviensGrizzly)
        .unwrap()
        .id;
    let a = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action,Action::ActivateAbility{source:s,..} if s==source))
        .unwrap()
        .action;
    submit_ok(&mut g, P0, a);
    for _ in 0..16 {
        if matches!(g.state.pending, Some(Pending::TopCardChoice { .. })) {
            return g;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    panic!("look pending not reached");
}
#[test]
fn opponents_cannot_distinguish_private_top_card_eligibility() {
    let eligible = waiting(CardId::GrizzlyBears);
    let land = waiting(CardId::Forest);
    assert_ne!(eligible.state, land.state);
    let a = eligible.view_for(P1, &[]);
    let b = land.view_for(P1, &[]);
    assert_eq!(
        a, b,
        "private top-card eligibility must not change another seat's projection"
    );
}
