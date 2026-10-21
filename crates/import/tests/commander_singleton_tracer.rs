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

// These mutations are synthetic, not additional captured Scryfall fixtures.
fn mutated_cards(name: &str, field: &str, value: Option<Value>, banned: bool) -> String {
    let mut snapshot: Value = serde_json::from_str(REAL_CARDS).unwrap();
    let mut cards: Vec<Value> = snapshot["raw_cards_jsonl"]
        .as_str()
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let card = cards.iter_mut().find(|card| card["name"] == name).unwrap();
    match value {
        Some(value) => card[field] = value,
        None => {
            card.as_object_mut().unwrap().remove(field);
        }
    }
    if banned {
        card["legalities"]["commander"] = json!("banned");
    }
    let raw = cards
        .iter()
        .map(|card| serde_json::to_string(card).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let digest: String = Sha256::digest(raw.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    snapshot["raw_cards_jsonl"] = json!(raw);
    snapshot["provenance"]["kind"] = json!("synthetic");
    snapshot["provenance"]["provider"] = json!("synthetic");
    snapshot["provenance"]["source"] = json!("R100-1 incomplete singleton metadata regression");
    snapshot["provenance"]["raw_sha256"] = json!(digest);
    snapshot["provenance"]["cards_sha256"] = json!(digest);
    snapshot.to_string()
}

fn duplicate_cases() -> Vec<(&'static str, String)> {
    let snapshot: Value = serde_json::from_str(REAL_CARDS).unwrap();
    let rats: Value = snapshot["raw_cards_jsonl"]
        .as_str()
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|card| card["name"] == "Relentless Rats")
        .unwrap();
    vec![
        (
            GRIZZLY.0,
            deck(
                vec![entry(OM, 1)],
                vec![entry(GRIZZLY, 2), entry(FOREST, 97)],
            ),
        ),
        (
            RAT_COLONY.0,
            deck(
                vec![entry(VITO, 1)],
                vec![entry(RAT_COLONY, 2), entry(SWAMP, 97)],
            ),
        ),
        (
            "Relentless Rats",
            deck(
                vec![entry(VITO, 1)],
                vec![
                    json!({"name": rats["name"], "oracle_id": rats["oracle_id"], "quantity": 2}),
                    entry(SWAMP, 97),
                ],
            ),
        ),
    ]
}

fn assert_incomplete_text_is_not_a_proven_duplicate(value: Option<Value>) {
    for (card, deck) in duplicate_cases() {
        let report = analyze_snapshots(
            &deck,
            &mutated_cards(card, "oracle_text", value.clone(), false),
            NOW,
        )
        .unwrap();
        assert_eq!(report.deck.total, 100);
        assert_eq!(
            report.status,
            LegalityStatus::NotCertified,
            "{}: {:#?}",
            card,
            report.findings
        );
        assert!(report.has_finding("CMD-METADATA", LegalityStatus::NotCertified));
        assert!(report.has_finding("CMD-SINGLETON", LegalityStatus::NotCertified));
        assert!(!report.has_finding("CMD-SINGLETON", LegalityStatus::Illegal));
    }
}

#[test]
fn duplicate_cards_with_missing_oracle_text_are_not_certified() {
    assert_incomplete_text_is_not_a_proven_duplicate(None);
}

#[test]
fn duplicate_cards_with_null_oracle_text_are_not_certified() {
    assert_incomplete_text_is_not_a_proven_duplicate(Some(Value::Null));
}

#[test]
fn duplicate_cards_with_nonstring_oracle_text_are_not_certified() {
    for value in [json!(7), json!(false), json!([]), json!({})] {
        assert_incomplete_text_is_not_a_proven_duplicate(Some(value));
    }
}

#[test]
fn duplicate_cards_with_unusable_type_metadata_are_not_certified() {
    for value in [
        None,
        Some(Value::Null),
        Some(json!(7)),
        Some(json!("")),
        Some(json!("Unknown Creature")),
    ] {
        let report = analyze_snapshots(
            &deck(
                vec![entry(OM, 1)],
                vec![entry(GRIZZLY, 2), entry(FOREST, 97)],
            ),
            &mutated_cards(GRIZZLY.0, "type_line", value, false),
            NOW,
        )
        .unwrap();
        assert_eq!(
            report.status,
            LegalityStatus::NotCertified,
            "{:#?}",
            report.findings
        );
        assert!(report.has_finding("CMD-SINGLETON", LegalityStatus::NotCertified));
        assert!(!report.has_finding("CMD-SINGLETON", LegalityStatus::Illegal));
    }
}

#[test]
fn incomplete_singleton_semantics_do_not_hide_proven_count_or_ban_violations() {
    let missing_text = mutated_cards(GRIZZLY.0, "oracle_text", None, false);
    for basic_count in [96, 98] {
        let report = analyze_snapshots(
            &deck(
                vec![entry(OM, 1)],
                vec![entry(GRIZZLY, 2), entry(FOREST, basic_count)],
            ),
            &missing_text,
            NOW,
        )
        .unwrap();
        assert_eq!(report.status, LegalityStatus::Illegal);
        assert!(
            report.has_finding("CMD-COUNT-100", LegalityStatus::Illegal),
            "{:#?}",
            report.findings
        );
        assert!(!report.has_finding("CMD-SINGLETON", LegalityStatus::Illegal));
    }
    let report = analyze_snapshots(
        &deck(
            vec![entry(OM, 1)],
            vec![entry(GRIZZLY, 2), entry(FOREST, 97)],
        ),
        &mutated_cards(GRIZZLY.0, "oracle_text", None, true),
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    assert!(report.has_finding("CMD-FORMAT-BANNED", LegalityStatus::Illegal));
    assert!(!report.has_finding("CMD-SINGLETON", LegalityStatus::Illegal));
}

#[test]
fn empty_but_present_oracle_text_still_proves_an_ordinary_duplicate() {
    let report = analyze_snapshots(
        &deck(
            vec![entry(OM, 1)],
            vec![entry(GRIZZLY, 2), entry(FOREST, 97)],
        ),
        &mutated_cards(GRIZZLY.0, "oracle_text", Some(json!("")), false),
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    assert!(report.has_finding("CMD-SINGLETON", LegalityStatus::Illegal));
}

#[test]
fn basic_land_exemption_does_not_need_oracle_text() {
    let report = analyze_snapshots(
        &deck(vec![entry(OM, 1)], vec![entry(FOREST, 99)]),
        &mutated_cards(FOREST.0, "oracle_text", None, false),
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::NotCertified);
    assert!(report.has_finding("CMD-SINGLETON", LegalityStatus::Legal));
    assert!(!report.has_finding("CMD-SINGLETON", LegalityStatus::Illegal));
}
