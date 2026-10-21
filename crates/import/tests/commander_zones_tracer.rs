//! Selected behavior: companions and sideboards are not_certified and never counted.

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
const LURRUS: (&str, &str) = (
    "Lurrus of the Dream-Den",
    "3bc757c1-3adb-4321-8832-8e1cc9e687f7",
);
const ISLAND: (&str, &str) = ("Island", "b2c6aa39-2d2a-459c-a555-fb48ba993373");
fn entry(card: (&str, &str), quantity: u32) -> Value {
    json!({"name": card.0, "oracle_id": card.1, "quantity": quantity})
}
fn deck(companions: Vec<Value>, sideboard: Vec<Value>) -> String {
    let zones = json!({"commanders": [entry(OM, 1)], "library": [entry(FOREST, 99)], "companions": companions, "sideboard": sideboard, "excluded": []});
    let raw = serde_json::to_string(&zones).unwrap();
    let digest: String = Sha256::digest(raw.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    json!({"schema": "aperture.commander-deck.v1",
        "provenance": {"kind": "constructed", "provider": "offline-test-snapshot", "source": "zones tracer", "captured_at": "2026-10-21T05:20:00Z", "raw_sha256": digest},
        "raw_deck": raw}).to_string()
}
#[test]
fn a_companion_is_not_certified_and_not_counted() {
    let report = analyze_snapshots(&deck(vec![entry(LURRUS, 1)], vec![]), REAL_CARDS, NOW).unwrap();
    assert_eq!(report.status, LegalityStatus::NotCertified);
    assert_eq!(report.deck.total, 100);
    assert!(report.has_finding("CMD-COMPANION", LegalityStatus::NotCertified));
}
#[test]
fn a_sideboard_is_not_certified_and_not_counted() {
    let report = analyze_snapshots(&deck(vec![], vec![entry(ISLAND, 5)]), REAL_CARDS, NOW).unwrap();
    assert_eq!(report.status, LegalityStatus::NotCertified);
    assert_eq!(report.deck.total, 100);
    assert!(report.has_finding("CMD-SIDEBOARD", LegalityStatus::NotCertified));
}
