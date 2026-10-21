//! No-laundering and byte-binding regressions.

use aperture_core::DeckList;
use aperture_import::commander::{CommanderError, analyze_snapshots};
use serde_json::Value;

const REAL_CARDS: &str = include_str!("fixtures/commander/real/cards-scryfall.json");
const LEGAL_DECK: &str = include_str!("fixtures/commander/real/deck-legal-100.json");
const NOW: u64 = 1_792_560_000;

#[test]
fn commander_output_cannot_deserialize_as_practice_deck_or_report() {
    let report = analyze_snapshots(LEGAL_DECK, REAL_CARDS, NOW).unwrap();
    let report_json = serde_json::to_string(&report).unwrap();
    assert!(serde_json::from_str::<DeckList>(&report_json).is_err());
    assert!(serde_json::from_str::<aperture_import::DeckReport>(&report_json).is_err());
    assert!(serde_json::from_str::<DeckList>(LEGAL_DECK).is_err());
    assert!(serde_json::from_str::<aperture_import::DeckReport>(LEGAL_DECK).is_err());
}

#[test]
fn provenance_digests_bind_the_embedded_bytes() {
    let mut cards: Value = serde_json::from_str(REAL_CARDS).unwrap();
    let raw = cards["raw_cards_jsonl"].as_str().unwrap().to_string();
    cards["raw_cards_jsonl"] = Value::String(format!("{raw}\n"));
    assert!(matches!(
        analyze_snapshots(LEGAL_DECK, &cards.to_string(), NOW),
        Err(CommanderError::Input(_))
    ));

    let mut deck: Value = serde_json::from_str(LEGAL_DECK).unwrap();
    deck["raw_deck"] = Value::String(deck["raw_deck"].as_str().unwrap().replace("98", "97"));
    assert!(matches!(
        analyze_snapshots(&deck.to_string(), REAL_CARDS, NOW),
        Err(CommanderError::Input(_))
    ));
}

#[test]
fn raw_sha256_must_bind_the_embedded_card_bytes() {
    let mut cards: Value = serde_json::from_str(REAL_CARDS).unwrap();
    cards["provenance"]["raw_sha256"] = Value::String("0".repeat(64));
    assert!(
        matches!(
            analyze_snapshots(LEGAL_DECK, &cards.to_string(), NOW),
            Err(CommanderError::Input(_))
        ),
        "a detached/zero raw_sha256 must not be presented as provenance"
    );
}
