mod common;
use aperture_core::*;
use common::*;
#[test]
fn selected_modes_are_public_and_offers_name_the_printed_instructions() {
    let mut g = fixture(4593)
        .hand(P0, &[CardId::TitaniasCommand])
        .battlefield(P0, &[CardId::Forest; 6])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let offer = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::CastSpellWithModes { modes: [2, 3], .. }))
        .unwrap();
    assert!(
        offer.label.contains("Create two Bear tokens"),
        "offer must name modes, not only unexplained indices: {}",
        offer.label
    );
    submit_ok(&mut g, P0, offer.action);
    for p in [P0, P1] {
        let view = serde_json::to_value(g.view_for(p, &[])).unwrap();
        let modes = view["stack"][0]["modes"]
            .as_array()
            .expect("selected modes are public stack information");
        assert_eq!(modes.len(), 2);
        assert_eq!(modes[0], "Create two Bear tokens");
        assert_eq!(
            modes[1],
            "Put two +1/+1 counters on each creature you control"
        );
    }
}
