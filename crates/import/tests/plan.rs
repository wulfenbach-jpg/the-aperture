//! Support planning: category inclusion/exclusion, merging duplicate entries,
//! sideboard/commander handling, and strict refusal.

use aperture_import::*;
use serde_json::json;

fn entry(name: &str, qty: u8, categories: &[&str]) -> serde_json::Value {
    json!({
        "id": 1,
        "quantity": qty,
        "categories": categories,
        "companion": false,
        "deletedAt": null,
        "modifier": "Normal",
        "card": {
            "id": 10,
            "oracleCard": {
                "id": 20,
                "layout": "normal",
                "faces": [{"name": name, "manaCost": "{1}", "types": ["Creature"]}]
            }
        }
    })
}

fn deck_json(cards: Vec<serde_json::Value>) -> String {
    json!({
        "id": 999,
        "name": "Practice Import",
        "deckFormat": 0,
        "private": false,
        "unlisted": false,
        "theorycrafted": false,
        "updatedAt": "2026-10-18T00:00:00.000000Z",
        "categories": [
            {"name": "Sideboard", "includedInDeck": true},
            {"name": "Maybeboard", "includedInDeck": false},
            {"name": "Creature", "includedInDeck": true},
            {"name": "Land", "includedInDeck": true},
            {"name": "Spells", "includedInDeck": true}
        ],
        "cards": cards
    })
    .to_string()
}

fn supported_40() -> Vec<serde_json::Value> {
    vec![
        entry("Forest", 9, &["Land"]),
        entry("Mountain", 8, &["Land"]),
        entry("Grizzly Bears", 2, &["Creature"]),
        // Same card, second printing: must merge into one 4-of entry.
        entry("Grizzly Bears", 2, &["Creature"]),
        entry("Runeclaw Bear", 4, &["Creature", "Spells"]),
        entry("Centaur Courser", 4, &["Creature"]),
        entry("Craw Wurm", 4, &["Creature"]),
        entry("Shock", 4, &["Spells"]),
        entry("Giant Growth", 3, &["Spells"]),
        // Reported separately, not part of the 40.
        entry("Counterspell", 2, &["Sideboard"]),
        // Excluded category: must not block launch even though the card is
        // not in the supported pool.
        entry("Llanowar Elves", 4, &["Maybeboard"]),
    ]
}

#[test]
fn a_fully_supported_import_launches_with_merged_entries() {
    let deck = normalize(&deck_json(supported_40())).expect("synthetic deck normalizes");
    let report = analyze(&deck);
    assert_eq!(report.total_cards, 40, "sideboard and maybeboard excluded");
    assert_eq!(report.supported_cards, 40);
    assert_eq!(report.unsupported_cards, 0);
    assert!(report.launchable, "refusal: {:?}", report.refusal);
    assert_eq!(report.sideboard.len(), 1);
    assert_eq!(report.sideboard[0].name, "Counterspell");
    assert_eq!(report.sideboard[0].quantity, 2);
    assert_eq!(report.excluded.len(), 1);
    assert_eq!(report.excluded[0].name, "Llanowar Elves");
    assert_eq!(report.excluded_categories, vec!["Maybeboard".to_string()]);

    let list = plan_deck(&report).expect("a supported deck plans");
    assert_eq!(list.size(), 40);
    assert_eq!(list.count_of(aperture_core::CardId::GrizzlyBears), 4);
    assert_eq!(list.count_of(aperture_core::CardId::Forest), 9);
    assert_eq!(list.count_of(aperture_core::CardId::Counterspell), 0);
}

#[test]
fn an_unsupported_main_deck_card_refuses_to_launch() {
    let mut cards = supported_40();
    cards.push(entry("Llanowar Elves", 4, &["Creature"]));
    let report = analyze(&normalize(&deck_json(cards)).unwrap());
    assert!(!report.launchable);
    assert_eq!(report.unsupported_cards, 4);
    assert!(report.unsupported[0].name == "Llanowar Elves");
    let refusal = report.refusal.clone().unwrap_or_default();
    assert!(refusal.contains("Llanowar Elves"), "{refusal}");
    assert!(refusal.contains("unsupported"), "{refusal}");
    assert!(plan_deck(&report).is_err());
}

#[test]
fn commander_sections_are_reported_and_refused() {
    let mut cards = supported_40();
    cards.push(entry("Grizzly Bears", 1, &["Commander"]));
    let deck = normalize(&deck_json(cards)).unwrap();
    let report = analyze(&deck);
    assert_eq!(report.commander, vec!["Grizzly Bears".to_string()]);
    assert!(!report.launchable);
    let refusal = report.refusal.clone().unwrap_or_default();
    assert!(refusal.contains("Commander"), "{refusal}");
}

#[test]
fn deleted_and_faceless_entries_are_reported_not_ignored() {
    let mut cards = supported_40();
    let mut deleted = entry("Hill Giant", 1, &["Creature"]);
    deleted["deletedAt"] = json!("2026-01-01T00:00:00Z");
    cards.push(deleted);
    let mut faceless = entry("Mystery Card", 1, &["Creature"]);
    faceless["card"]["oracleCard"]["faces"] = json!([]);
    faceless["card"]["oracleCard"]["name"] = json!("Mystery Card");
    cards.push(faceless);
    let report = analyze(&normalize(&deck_json(cards)).unwrap());
    assert_eq!(
        report.total_cards, 41,
        "the undeletable entry counts; the deleted one does not"
    );
    assert_eq!(
        report.supported_cards + report.unsupported_cards,
        report.total_cards
    );
    assert!(report.excluded.iter().any(|c| c.name == "Hill Giant"));
    assert!(
        report
            .unsupported
            .iter()
            .any(|u| u.name == "Mystery Card" && u.reason.contains("no card data"))
    );
    // The faceless entry is a main-deck card we cannot identify: it must block.
    assert!(!report.launchable);
}

#[test]
fn a_deck_that_is_too_small_is_refused_by_the_practice_profile() {
    let cards = vec![
        entry("Forest", 10, &["Land"]),
        entry("Grizzly Bears", 10, &["Creature"]),
    ];
    let report = analyze(&normalize(&deck_json(cards)).unwrap());
    assert_eq!(report.supported_cards, 20);
    assert!(!report.launchable);
    let refusal = report.refusal.clone().unwrap_or_default();
    assert!(refusal.contains("40"), "{refusal}");
    assert!(report.legality.iter().any(|l| l.contains("40")));
}

#[test]
fn private_decks_are_refused_with_an_explanation() {
    let mut value: serde_json::Value = serde_json::from_str(&deck_json(supported_40())).unwrap();
    value["private"] = json!(true);
    let deck = normalize(&value.to_string()).unwrap();
    let report = analyze(&deck);
    assert!(!report.launchable);
    assert!(
        report
            .refusal
            .clone()
            .unwrap_or_default()
            .contains("private")
    );
}
