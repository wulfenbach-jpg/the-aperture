//! R102-1: a pregame chosen-color commander (CR 903.4b) has an unresolved
//! color identity during construction. The slice must not treat the recorded
//! empty identity as colorless, and must not emit dependent 903.5c/d illegals.
//! Synthetic wrappers are explicitly marked; real wrappers are the captured
//! Scryfall subset.

use aperture_import::commander::{LegalityStatus, analyze_snapshots};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const REAL_CARDS: &str = include_str!("fixtures/commander/real/cards-scryfall.json");
const NOW: u64 = 1_792_560_000;
const FACELESS: (&str, &str) = ("Faceless One", "ef220170-c01a-4ca5-b887-f248c50227ee");
const OM: (&str, &str) = (
    "Omnath, Locus of Mana",
    "9da896e1-2256-425b-b801-1ae6f0470559",
);
const FOREST: (&str, &str) = ("Forest", "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
const SWAMP: (&str, &str) = ("Swamp", "56719f6a-1a6c-4c0a-8d21-18f7d7350b68");
const WASTES: (&str, &str) = ("Wastes", "05d24b0c-904a-46b6-b42a-96a4d91a0dd4");
const GRIZZLY: (&str, &str) = ("Grizzly Bears", "14c8f55d-d177-4c25-a931-ebeb9e6062a0");
const GOLOS: (&str, &str) = (
    "Golos, Tireless Pilgrim",
    "efe9c3e0-7315-408e-9903-9d25a681039b",
);
const THRASIOS: (&str, &str) = (
    "Thrasios, Triton Hero",
    "3d867016-2601-4a37-a73d-308898d3bd37",
);

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn entry(card: (&str, &str), quantity: u32) -> Value {
    json!({"name": card.0, "oracle_id": card.1, "quantity": quantity})
}
fn deck(commanders: Vec<Value>, library: Vec<Value>) -> String {
    let zones = json!({"commanders": commanders, "library": library, "companions": [], "sideboard": [], "excluded": []});
    let raw = serde_json::to_string(&zones).unwrap();
    json!({"schema": "aperture.commander-deck.v1",
        "provenance": {"kind": "constructed", "provider": "offline-test-snapshot", "source": "R102-1 pregame color regression", "captured_at": "2026-10-21T05:20:00Z", "raw_sha256": sha256_hex(raw.as_bytes())},
        "raw_deck": raw}).to_string()
}
fn synthetic_chosen_color_cards() -> String {
    let mut cards: Value = serde_json::from_str(REAL_CARDS).unwrap();
    let mut raw: Vec<Value> = cards["raw_cards_jsonl"]
        .as_str()
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for card in &mut raw {
        if card["name"] == OM.0 {
            card["color_identity"] = json!([]);
            card["keywords"] = json!([]);
            card["oracle_text"] = json!(
                "If Omnath, Locus of Mana is your commander, choose a color before the game begins. Omnath, Locus of Mana is the chosen color."
            );
        }
    }
    let jsonl = raw
        .iter()
        .map(|c| serde_json::to_string(c).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let digest = sha256_hex(jsonl.as_bytes());
    cards["raw_cards_jsonl"] = Value::String(jsonl);
    cards["provenance"]["kind"] = json!("synthetic");
    cards["provenance"]["provider"] = json!("synthetic");
    cards["provenance"]["source"] = json!("R102-1 synthetic chosen-color-only class");
    cards["provenance"]["cards_sha256"] = json!(digest.clone());
    cards["provenance"]["raw_sha256"] = json!(digest);
    cards["provenance"]["card_count"] = json!(raw.len());
    cards.to_string()
}
fn has(
    report: &aperture_import::commander::CommanderReport,
    code: &str,
    status: LegalityStatus,
) -> bool {
    report
        .findings
        .iter()
        .any(|f| f.code == code && f.status == status)
}

fn assert_unresolved_choice(report: &aperture_import::commander::CommanderReport) {
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
    assert!(has(
        report,
        "CMD-ELIGIBILITY-VARIANT",
        LegalityStatus::NotCertified
    ));
    assert!(has(report, "CMD-IDENTITY", LegalityStatus::NotCertified));
    assert!(!has(report, "CMD-IDENTITY", LegalityStatus::Illegal));
    assert!(!has(report, "CMD-BASIC-LAND-TYPE", LegalityStatus::Illegal));
}

#[test]
fn faceless_one_with_colored_lands_is_not_certified_without_dependent_color_illegals() {
    let report = analyze_snapshots(
        &deck(vec![entry(FACELESS, 1)], vec![entry(FOREST, 99)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_unresolved_choice(&report);
}

#[test]
fn faceless_one_with_a_colored_spell_is_not_certified() {
    let report = analyze_snapshots(
        &deck(
            vec![entry(FACELESS, 1)],
            vec![entry(WASTES, 98), entry(GRIZZLY, 1)],
        ),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_unresolved_choice(&report);
}

#[test]
fn synthetic_chosen_color_only_without_background_or_partner_is_not_certified() {
    let cards = synthetic_chosen_color_cards();
    let report = analyze_snapshots(
        &deck(vec![entry(OM, 1)], vec![entry(SWAMP, 99)]),
        &cards,
        NOW,
    )
    .unwrap();
    assert_unresolved_choice(&report);
}

#[test]
fn ordinary_known_identity_off_color_is_still_illegal() {
    let report = analyze_snapshots(
        &deck(vec![entry(OM, 1)], vec![entry(SWAMP, 99)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    assert!(has(&report, "CMD-IDENTITY", LegalityStatus::Illegal));
}

#[test]
fn chosen_color_independent_count_banned_and_duplicate_violations_still_apply() {
    let short = analyze_snapshots(
        &deck(vec![entry(FACELESS, 1)], vec![entry(FOREST, 98)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(short.status, LegalityStatus::Illegal);
    assert!(has(&short, "CMD-COUNT-100", LegalityStatus::Illegal));

    let banned = analyze_snapshots(
        &deck(
            vec![entry(FACELESS, 1)],
            vec![entry(WASTES, 98), entry(GOLOS, 1)],
        ),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(banned.status, LegalityStatus::Illegal);
    assert!(has(&banned, "CMD-FORMAT-BANNED", LegalityStatus::Illegal));

    let duplicate = analyze_snapshots(
        &deck(
            vec![entry(FACELESS, 1)],
            vec![entry(WASTES, 97), entry(GRIZZLY, 2)],
        ),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(duplicate.status, LegalityStatus::Illegal);
    assert!(has(&duplicate, "CMD-SINGLETON", LegalityStatus::Illegal));
}

#[test]
fn unsupported_variant_with_known_identity_retains_off_color_illegal() {
    let report = analyze_snapshots(
        &deck(vec![entry(THRASIOS, 1)], vec![entry(SWAMP, 99)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    assert!(has(&report, "CMD-IDENTITY", LegalityStatus::Illegal));
    assert!(has(
        &report,
        "CMD-ELIGIBILITY-VARIANT",
        LegalityStatus::NotCertified
    ));
}

#[test]
fn ordinary_mono_green_control_is_legal() {
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
