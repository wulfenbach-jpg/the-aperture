//! Native discriminating save probe, independent of writer.
mod common;
use aperture_core::*;
use common::*;
#[test]
fn two_stacked_activations_from_one_grizzly_are_valid_save_state() {
    let mut board = vec![CardId::ViviensGrizzly];
    board.extend([CardId::Forest; 12]);
    let mut g = fixture(2099)
        .battlefield(P0, &board)
        .library(P0, &[CardId::GrizzlyBears, CardId::Forest])
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
    for _ in 0..2 {
        let a = g
            .legal_actions()
            .into_iter()
            .find(|a| matches!(a.action,Action::ActivateAbility{source:s,..} if s==source))
            .expect("ability offered with sufficient mana")
            .action;
        submit_ok(&mut g, P0, a);
    }
    assert_eq!(g.state.stack.len(), 2);
    Game::from_state(g.state.clone()).expect("two legal ability entries must remain a valid state");
}
