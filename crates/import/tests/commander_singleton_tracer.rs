//! Selected behavior: 100-card singleton (CR 903.5b), Basic-supertype
//! exemption and card-specific deckbuilding allowances.

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
const GRIZZLY: (&str, &str) = ("Grizzly Bears", "14c8f55d-d177-4c25-a931-ebeb9e6062a0");
const VITO: (&str, &str) = (
    "Vito, Thorn of the Dusk Rose",
    "7336a541-3ae9-4ab2-acb8-f574c0a5b3a9",
);
const SWAMP: (&str, &str) = ("Swamp", "56719f6a-1a6c-4c0a-8d21-18f7d7350b68");
const RAT_COLONY: (&str, &str) = ("Rat Colony", "ec77d23b-0165-450d-9aae-73b755163753");

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
    json!({"schema": "aperture.commander-deck.v1",
        "provenance": {"kind": "constructed", "provider": "offline-test-snapshot", "source": "singleton tracer", "captured_at": "2026-10-21T05:20:00Z", "raw_sha256": digest},
        "raw_deck": raw}).to_string()
}

#[test]
fn repeated_basic_lands_are_exempt() {
    let report = analyze_snapshots(
        &deck(vec![entry(OM, 1)], vec![entry(FOREST, 99)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(
        report.status,
        LegalityStatus::Legal,
        "{:#?}",
        report.findings
    );
}

#[test]
fn duplicate_nonbasic_cards_are_illegal() {
    let report = analyze_snapshots(
        &deck(
            vec![entry(OM, 1)],
            vec![entry(GRIZZLY, 2), entry(FOREST, 97)],
        ),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(
        report.status,
        LegalityStatus::Illegal,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-SINGLETON", LegalityStatus::Illegal));
}

#[test]
fn card_specific_allowances_are_not_certified_not_illegal() {
    let report = analyze_snapshots(
        &deck(
            vec![entry(VITO, 1)],
            vec![entry(RAT_COLONY, 2), entry(SWAMP, 97)],
        ),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-DECKBUILDING-EXCEPTION", LegalityStatus::NotCertified));
    assert!(!report.has_finding("CMD-SINGLETON", LegalityStatus::Illegal));
}
