//! Selected behavior: off-color library cards are illegal (CR 903.5c).

use aperture_import::commander::{LegalityStatus, analyze_snapshots};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const REAL_CARDS: &str = include_str!("fixtures/commander/real/cards-scryfall.json");
const NOW: u64 = 1_792_560_000;
const OM: (&str, &str) = (
    "Omnath, Locus of Mana",
    "9da896e1-2256-425b-b801-1ae6f0470559",
);
const FOREST: (&str, &str) = ("Forest", "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
const ISLAND: (&str, &str) = ("Island", "b2c6aa39-2d2a-459c-a555-fb48ba993373");

fn entry(card: (&str, &str), quantity: u32) -> Value {
    json!({"name": card.0, "oracle_id": card.1, "quantity": quantity})
}

fn deck(commanders: Vec<Value>, library: Vec<Value>) -> String {
    let zones = json!({"commanders": commanders, "library": library, "companions": [], "sideboard": [], "excluded": []});
    let raw = serde_json::to_string(&zones).unwrap();
    let digest: String = Sha256::digest(raw.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    json!({
        "schema": "aperture.commander-deck.v1",
        "provenance": {"kind": "constructed", "provider": "offline-test-snapshot",
            "source": "identity tracer", "captured_at": "2026-10-21T05:20:00Z", "raw_sha256": digest},
        "raw_deck": raw
    }).to_string()
}

#[test]
fn an_off_color_library_card_is_illegal() {
    let deck = deck(
        vec![entry(OM, 1)],
        vec![entry(FOREST, 98), entry(ISLAND, 1)],
    );
    let report = analyze_snapshots(&deck, REAL_CARDS, NOW).expect("snapshots bind");
    assert_eq!(
        report.status,
        LegalityStatus::Illegal,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-IDENTITY", LegalityStatus::Illegal));
}
