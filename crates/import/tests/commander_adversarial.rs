//! Adversarial snapshots derived from the real fixture at test time.
//! These wrappers are explicitly synthetic mutations, not captured Scryfall data.

use aperture_import::commander::{LegalityStatus, analyze_snapshots};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const REAL_CARDS: &str = include_str!("fixtures/commander/real/cards-scryfall.json");
const LEGAL_DECK: &str = include_str!("fixtures/commander/real/deck-legal-100.json");
const NOW: u64 = 1_792_560_000;

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn wrapper(cards: &[Value]) -> String {
    let mut base: Value = serde_json::from_str(REAL_CARDS).unwrap();
    let raw = cards
        .iter()
        .map(|card| serde_json::to_string(card).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let digest = sha256_hex(raw.as_bytes());
    base["raw_cards_jsonl"] = Value::String(raw);
    base["provenance"]["cards_sha256"] = Value::String(digest.clone());
    base["provenance"]["raw_sha256"] = Value::String(digest);
    base["provenance"]["card_count"] = json!(cards.len());
    base["provenance"]["note"] =
        json!("ADVERSARIAL MUTATION; not a genuine captured Scryfall fixture");
    base.to_string()
}

fn cards() -> Vec<Value> {
    let base: Value = serde_json::from_str(REAL_CARDS).unwrap();
    base["raw_cards_jsonl"]
        .as_str()
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn conflicting_metadata_for_the_same_oracle_id_is_not_certified_in_any_order() {
    let original = cards();
    let mut modified = original[0].clone();
    modified["color_identity"] = json!(["U"]);

    for mutated in [
        original
            .iter()
            .cloned()
            .chain([modified.clone()])
            .collect::<Vec<_>>(),
        std::iter::once(modified.clone())
            .chain(original.iter().cloned())
            .collect::<Vec<_>>(),
    ] {
        let report = analyze_snapshots(LEGAL_DECK, &wrapper(&mutated), NOW).unwrap();
        assert_eq!(
            report.status,
            LegalityStatus::NotCertified,
            "conflicting same-oracle metadata must not depend on array order: {:#?}",
            report.findings
        );
        assert!(report.has_finding("CMD-AMBIGUOUS-IDENTITY", LegalityStatus::NotCertified));
    }
}

#[test]
fn a_null_oracle_text_on_a_commander_is_not_certified() {
    let mut mutated = cards();
    mutated[0]["oracle_text"] = Value::Null;
    let report = analyze_snapshots(LEGAL_DECK, &wrapper(&mutated), NOW).unwrap();
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-METADATA", LegalityStatus::NotCertified));
}

#[test]
fn an_unknown_type_line_token_is_not_certified() {
    let mut mutated = cards();
    mutated[0]["type_line"] = json!("Fictional Legendary Creature");
    let report = analyze_snapshots(LEGAL_DECK, &wrapper(&mutated), NOW).unwrap();
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-METADATA", LegalityStatus::NotCertified));
}

#[test]
fn the_same_english_name_with_two_oracle_ids_is_not_certified() {
    let mut mutated = cards();
    let mut duplicate_name = mutated
        .iter()
        .find(|card| card["name"] == "Forest")
        .unwrap()
        .clone();
    duplicate_name["oracle_id"] = json!("00000000-0000-0000-0000-000000000099");
    duplicate_name["id"] = json!("synthetic-duplicate-name");
    mutated.push(duplicate_name);
    let report = analyze_snapshots(LEGAL_DECK, &wrapper(&mutated), NOW).unwrap();
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-AMBIGUOUS-IDENTITY", LegalityStatus::NotCertified));
}

#[test]
fn partner_rules_text_without_keywords_is_still_not_certified() {
    let deck_zones = json!({
        "commanders": [{"name": "Thrasios, Triton Hero", "oracle_id": "3d867016-2601-4a37-a73d-308898d3bd37", "quantity": 1}],
        "library": [{"name": "Forest", "oracle_id": "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6", "quantity": 99}],
        "companions": [], "sideboard": [], "excluded": []
    });
    let raw_deck = serde_json::to_string(&deck_zones).unwrap();
    let digest = sha256_hex(raw_deck.as_bytes());
    let deck = json!({"schema": "aperture.commander-deck.v1",
        "provenance": {"kind": "constructed", "provider": "offline-test-snapshot", "source": "partner text regression", "captured_at": "2026-10-21T05:20:00Z", "raw_sha256": digest},
        "raw_deck": raw_deck}).to_string();
    let mut mutated = cards();
    let thrasios = mutated
        .iter_mut()
        .find(|card| card["oracle_id"] == "3d867016-2601-4a37-a73d-308898d3bd37")
        .expect("Thrasios is in the real fixture");
    thrasios["keywords"] = json!([]);
    let report = analyze_snapshots(&deck, &wrapper(&mutated), NOW).unwrap();
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-ELIGIBILITY-VARIANT", LegalityStatus::NotCertified));
}
