//! Issue 17 starter package: the exact list binds through the existing offline
//! analyzer schema. This is not a parallel legality engine.

use std::collections::BTreeMap;

use aperture_core::DeckList;
use aperture_import::commander::{LegalityStatus, SupportStatus, analyze_snapshots};
use serde_json::Value;

const DECK_SNAPSHOT: &str =
    include_str!("../../../fixtures/starter-corpus/jasmine-vanilla-stompy.deck.json");
const CARDS_SNAPSHOT: &str =
    include_str!("../../../fixtures/starter-corpus/jasmine-vanilla-stompy.cards.json");
const FIXTURE_MANIFEST: &str = include_str!("../../../fixtures/starter-corpus/manifest.json");
const DECK_TEXT: &str = include_str!("../../../decks/starter-corpus/jasmine-vanilla-stompy.txt");
const NOW: u64 = 1_793_000_000; // 2026-10-25T11:33:20Z

#[test]
fn the_exact_starter_list_is_format_legal_and_engine_supported() {
    let report = analyze_snapshots(DECK_SNAPSHOT, CARDS_SNAPSHOT, NOW)
        .expect("the snapshots bind and analyze");
    assert_eq!(report.schema, "aperture.commander-report.v1");
    assert_eq!(report.deck.commanders, 1);
    assert_eq!(report.deck.library, 99);
    assert_eq!(report.deck.total, 100);
    assert_eq!(report.deck.unique_canonical_names, 61);
    assert_eq!(
        report.status,
        LegalityStatus::Legal,
        "findings: {:#?}",
        report.findings
    );
    assert_eq!(
        report.engine_support.status,
        SupportStatus::Supported,
        "unsupported: {:#?}",
        report.engine_support.unsupported_cards
    );
    assert!(report.engine_support.unsupported_cards.is_empty());
    assert_eq!(report.commanders.len(), 1);
    assert_eq!(report.commanders[0].name, "Jasmine Boreal");
    assert_eq!(report.commanders[0].eligibility, LegalityStatus::Legal);
    assert_eq!(report.commanders[0].color_identity, vec!["G", "W"]);
    assert_eq!(report.format_legality.status, LegalityStatus::Legal);
}

/// The analyzer's blanket gameplay refusal predates the bounded command-zone
/// profiles. It is deliberately untouched here: a legal, fully supported deck
/// still does not claim full Commander gameplay.
#[test]
fn analyzer_gameplay_refusal_is_preserved() {
    let report = analyze_snapshots(DECK_SNAPSHOT, CARDS_SNAPSHOT, NOW).unwrap();
    assert!(!report.gameplay.available);
    assert!(!report.gameplay.reasons.is_empty());
}

#[test]
fn the_deck_snapshot_binds_the_exact_text_library() {
    let snapshot: Value = serde_json::from_str(DECK_SNAPSHOT).expect("snapshot json");
    let raw: Value =
        serde_json::from_str(snapshot["raw_deck"].as_str().expect("raw_deck")).expect("deck json");
    let library = raw["library"].as_array().expect("library");
    let mut snapshot_cards: BTreeMap<String, u32> = BTreeMap::new();
    for entry in library {
        let name = entry["name"].as_str().expect("name").to_string();
        let quantity = entry["quantity"].as_u64().expect("quantity") as u32;
        assert_eq!(
            snapshot_cards.insert(name.clone(), quantity),
            None,
            "{name} must appear once in the snapshot"
        );
    }

    let deck = DeckList::parse_practice("jasmine-vanilla-stompy", DECK_TEXT).expect("deck text");
    let mut text_cards: BTreeMap<String, u32> = BTreeMap::new();
    for (card, quantity) in &deck.cards {
        text_cards.insert(card.name().to_string(), *quantity as u32);
    }
    assert_eq!(snapshot_cards, text_cards);
    assert_eq!(
        snapshot_cards.values().sum::<u32>(),
        99,
        "the library snapshot is exactly the 99-card library"
    );

    let commanders = raw["commanders"].as_array().expect("commanders");
    assert_eq!(commanders.len(), 1);
    assert_eq!(commanders[0]["name"], "Jasmine Boreal");
    assert_eq!(commanders[0]["quantity"], 1);

    let manifest: Value = serde_json::from_str(FIXTURE_MANIFEST).expect("fixture manifest");
    assert_eq!(manifest["kind"], "real");
    assert!(manifest["source"].as_str().unwrap().contains("Scryfall"));
    assert_eq!(
        manifest["deck_sha256"], snapshot["provenance"]["raw_sha256"],
        "fixture manifest pins the deck snapshot bytes"
    );
    assert_eq!(
        manifest["cards_sha256"],
        serde_json::from_str::<Value>(CARDS_SNAPSHOT).unwrap()["provenance"]["cards_sha256"]
    );
}
