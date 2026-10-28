//! Retrospective source and snow-mana controls, not test-first claims.
mod common;
use aperture_core::*;
use common::*;

#[test]
fn both_slice44_cards_match_their_sourced_printings() {
    let records: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/starter-corpus/slice44-sourced.json"
    ))
    .unwrap();
    for record in records["cards"].as_array().unwrap() {
        let name = record["name"].as_str().unwrap();
        let def = catalog::by_name(name).unwrap().def();
        assert_eq!(def.oracle, record["oracle_text"].as_str().unwrap());
        assert_eq!(def.type_line, record["type_line"].as_str().unwrap());
        let cost = record["mana_cost"].as_str().filter(|s| !s.is_empty());
        assert_eq!(def.cost, cost);
        assert_eq!(
            def.power.map(|p| p.to_string()).as_deref(),
            record["power"].as_str()
        );
        assert_eq!(
            def.toughness.map(|t| t.to_string()).as_deref(),
            record["toughness"].as_str()
        );
        assert!(def.is_snow());
        assert!(!record["scryfall_id"].as_str().unwrap().is_empty());
        assert!(!record["retrieved_at"].as_str().unwrap().is_empty());
    }
}

#[test]
fn snow_forest_is_offered_as_a_land_and_as_a_green_mana_source() {
    let mut g = fixture(4412)
        .hand(P0, &[CardId::SnowCoveredForest])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let snow = hand_card(&g, P0, CardId::SnowCoveredForest);
    let play = g
        .legal_actions()
        .into_iter()
        .find(|a| a.action == Action::PlayLand { card: snow })
        .expect("snow basic is offered for land play")
        .action;
    submit_ok(&mut g, P0, play);
    assert_eq!(g.state.object(snow).zone, Zone::Battlefield);
    let tap = g
        .legal_actions()
        .into_iter()
        .find(|a| a.action == Action::ActivateManaAbility { land: snow })
        .expect("snow basic is offered for green mana")
        .action;
    let events = submit_ok(&mut g, P0, tap);
    assert!(g.state.object(snow).tapped);
    assert_eq!(
        g.state.object(snow).card.def().basic_mana,
        Some(aperture_core::card::ManaColor::Green)
    );
    assert!(!events.is_empty());
    assert!(g.state.validate().is_ok());
}
