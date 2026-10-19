//! Normal single-face cards and quantity handling.
//!
//! `data/archidekt-normal-fixture.json` is a *synthetic* fixture that
//! reproduces the real Archidekt provider structure for ordinary cards: the
//! checked-in captured response (`data/archidekt-26206384.json`) shows that a
//! normal card (e.g. Island, Grizzly Bears) arrives with `layout: "normal"` and
//! an empty `faces` array, with the name/layout on `oracleCard`. Before the fix
//! `classify` rejected every empty-faces entry as "no card data (faces
//! missing)", so this fixture exercises the corrected path.

use aperture_core::CardId;
use aperture_import::*;
use serde_json::json;

const FIXTURE: &str = include_str!("data/archidekt-normal-fixture.json");

#[test]
fn normal_single_face_cards_are_supported_and_launchable() {
    let deck = normalize(FIXTURE).expect("the synthetic normal-card fixture normalizes");
    assert_eq!(deck.deck_id, Some(4242));
    assert_eq!(deck.name, "Fixture Normal Deck");
    assert_eq!(deck.entries.len(), 3, "three provider entries");
    assert_eq!(
        deck.entries.iter().map(|e| e.quantity as u32).sum::<u32>(),
        40
    );

    let report = analyze(&deck);
    assert_eq!(report.total_cards, 40);
    assert_eq!(report.supported_cards, 40, "all 40 cards are in the pool");
    assert_eq!(report.unsupported_cards, 0);
    assert!(report.launchable, "refusal: {:?}", report.refusal);
    assert_eq!(report.entries.len(), 3, "no entry is dropped");
    assert!(
        report
            .entries
            .iter()
            .all(|e| e.status == EntryStatus::Supported)
    );
    assert_eq!(
        report
            .entries
            .iter()
            .find(|e| e.name == "Grizzly Bears")
            .map(|e| e.quantity),
        Some(4)
    );

    let list = plan_deck(&report).expect("a fully supported deck plans");
    assert_eq!(list.size(), 40);
    assert_eq!(list.count_of(CardId::GrizzlyBears), 4);
    assert_eq!(list.count_of(CardId::Island), 20);
    assert_eq!(list.count_of(CardId::Forest), 16);
}

fn card(name: &str, quantity: serde_json::Value) -> serde_json::Value {
    json!({
        "id": 1,
        "quantity": quantity,
        "categories": ["Creature"],
        "companion": false,
        "deletedAt": null,
        "modifier": "Normal",
        "card": {
            "id": 10,
            "oracleCard": {
                "id": 20,
                "name": name,
                "layout": "normal",
                "faces": []
            }
        }
    })
}

fn deck_with(cards: serde_json::Value) -> String {
    json!({
        "id": 7,
        "name": "Quantity Probe",
        "deckFormat": 0,
        "private": false,
        "unlisted": false,
        "theorycrafted": false,
        "updatedAt": "2026-10-18T00:00:00.000000Z",
        "categories": [],
        "cards": cards
    })
    .to_string()
}

fn quantity_error(quantity: serde_json::Value) -> String {
    let json = deck_with(json!([card("Grizzly Bears", quantity)]));
    match normalize(&json) {
        Err(ImportError::Parse(message)) => message,
        other => panic!("expected a Parse error, got {other:?}"),
    }
}

#[test]
fn a_zero_quantity_is_an_error_not_a_silent_skip() {
    let message = quantity_error(json!(0));
    assert!(message.contains("zero quantity"), "{message}");
}

#[test]
fn a_quantity_above_the_u8_range_is_rejected() {
    let message = quantity_error(json!(300));
    assert!(message.contains("300"), "{message}");
    assert!(message.contains("1..255"), "{message}");
}

#[test]
fn a_negative_quantity_is_rejected() {
    let message = quantity_error(json!(-1));
    assert!(message.contains("-1"), "{message}");
    assert!(message.contains("unsigned"), "{message}");
}

#[test]
fn a_string_quantity_is_rejected() {
    let message = quantity_error(json!("4"));
    assert!(message.contains("unsigned"), "{message}");
}

#[test]
fn duplicate_entries_are_preserved_exactly_and_merged_at_plan_time() {
    let cards = json!([
        card("Island", json!(20)),
        card("Forest", json!(16)),
        card("Grizzly Bears", json!(2)),
        card("Grizzly Bears", json!(2)),
    ]);
    let deck = normalize(&deck_with(cards)).expect("normalizes");
    let bears: Vec<u8> = deck
        .entries
        .iter()
        .filter(|e| e.name == "Grizzly Bears")
        .map(|e| e.quantity)
        .collect();
    assert_eq!(bears, vec![2, 2], "each entry keeps its exact quantity");

    let report = analyze(&deck);
    assert_eq!(report.supported_cards, 40);
    let list = plan_deck(&report).expect("plans");
    assert_eq!(list.count_of(CardId::GrizzlyBears), 4);
}
