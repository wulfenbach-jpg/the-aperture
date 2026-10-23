//! Sourced command-zone practice cards. Official Gatherer text was verified by
//! the independent Rules assignment (Rules-1-notes.md) and recorded in
//! docs/rules-reference.md.
//!
//! These tests use the existing `catalog::by_name` API so the first run is a
//! genuine behavioral RED, not a compile failure.

use aperture_core::catalog;

#[test]
fn sourced_isamaru_is_in_the_supported_pool() {
    let card = catalog::by_name("Isamaru, Hound of Konda")
        .expect("verified Gatherer card must be supported by name");
    let def = card.def();
    assert_eq!(def.cost, Some("{W}"));
    assert_eq!(def.type_line, "Legendary Creature — Dog");
    assert_eq!(def.power, Some(2));
    assert_eq!(def.toughness, Some(2));
    // Vanilla: no rules text and no spell effect beyond the creature spell.
    assert_eq!(def.oracle, "");
}

#[test]
fn sourced_plains_is_in_the_supported_pool() {
    let card = catalog::by_name("Plains").expect("verified Gatherer basic land must be supported");
    let def = card.def();
    assert_eq!(def.type_line, "Basic Land — Plains");
    assert_eq!(def.basic_mana, Some(aperture_core::ManaColor::White));
}

#[test]
fn sourced_unsummon_is_in_the_supported_pool() {
    let card = catalog::by_name("Unsummon").expect("verified Gatherer card");
    let def = card.def();
    assert_eq!(def.cost, Some("{U}"));
    assert_eq!(def.type_line, "Instant");
    assert_eq!(def.oracle, "Return target creature to its owner's hand.");
}

#[test]
fn sourced_time_ebb_is_in_the_supported_pool() {
    let card = catalog::by_name("Time Ebb").expect("verified Gatherer card");
    let def = card.def();
    assert_eq!(def.cost, Some("{2}{U}"));
    assert_eq!(def.type_line, "Sorcery");
    assert_eq!(
        def.oracle,
        "Put target creature on top of its owner's library."
    );
}

#[test]
fn sourced_swords_to_plowshares_is_in_the_supported_pool() {
    let card = catalog::by_name("Swords to Plowshares").expect("verified Gatherer card");
    let def = card.def();
    assert_eq!(def.cost, Some("{W}"));
    assert_eq!(def.type_line, "Instant");
    assert_eq!(
        def.oracle,
        "Exile target creature. Its controller gains life equal to its power."
    );
}
