//! Boundary regressions for the implemented Commander analysis rules.

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
const THRASIOS: (&str, &str) = (
    "Thrasios, Triton Hero",
    "3d867016-2601-4a37-a73d-308898d3bd37",
);
const TYMNA: (&str, &str) = ("Tymna the Weaver", "d15642e4-e61c-4d29-af48-de837991245e");
const TALRAND: (&str, &str) = (
    "Talrand, Sky Summoner",
    "ea1eb902-a23c-44ff-9169-19baf71de238",
);
const GOLOS: (&str, &str) = (
    "Golos, Tireless Pilgrim",
    "efe9c3e0-7315-408e-9903-9d25a681039b",
);
const VALKI: (&str, &str) = (
    "Valki, God of Lies // Tibalt, Cosmic Impostor",
    "907ae517-22d5-4ac7-bc3a-3f4d5eaeeb57",
);
const MOUNTAIN: (&str, &str) = ("Mountain", "a3fb7228-e76b-4e96-a40e-20b5fed75685");

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
        "provenance": {"kind": "constructed", "provider": "offline-test-snapshot", "source": "edge regression", "captured_at": "2026-10-21T05:20:00Z", "raw_sha256": digest},
        "raw_deck": raw}).to_string()
}
fn synthetic_card(name: &str, oracle: &str, type_line: &str, ci: &[&str], legality: &str) -> Value {
    json!({"object": "card", "id": format!("id-{name}"), "oracle_id": oracle, "name": name,
        "layout": "normal", "type_line": type_line, "color_identity": ci,
        "legalities": {"commander": legality}, "keywords": [], "oracle_text": ""})
}
fn synthetic_cards(cards: &[Value]) -> String {
    let raw = cards
        .iter()
        .map(|c| serde_json::to_string(c).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let digest: String = Sha256::digest(raw.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    json!({"schema": "aperture.card-metadata.v1",
        "provenance": {"kind": "synthetic", "provider": "synthetic", "source": "edge regression", "captured_at": "2026-10-21T05:20:00Z", "cards_sha256": digest, "raw_sha256": digest, "card_count": cards.len()},
        "raw_cards_jsonl": raw}).to_string()
}

#[test]
fn zero_commanders_is_illegal() {
    let report = analyze_snapshots(
        &deck(vec![], vec![entry(FOREST, 99), entry(OM, 1)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    assert!(report.has_finding("CMD-COMMANDER-COUNT", LegalityStatus::Illegal));
}

#[test]
fn every_two_commander_variant_is_not_certified() {
    for commanders in [
        vec![entry(THRASIOS, 1), entry(TYMNA, 1)],
        vec![entry(OM, 1), entry(TALRAND, 1)],
    ] {
        let report =
            analyze_snapshots(&deck(commanders, vec![entry(FOREST, 98)]), REAL_CARDS, NOW).unwrap();
        assert_eq!(report.status, LegalityStatus::NotCertified);
        assert!(report.has_finding("CMD-COMMANDER-COUNT", LegalityStatus::NotCertified));
        assert!(!report.has_finding("CMD-COMMANDER-COUNT", LegalityStatus::Illegal));
    }
}

#[test]
fn a_non_normal_commander_is_not_certified() {
    let report = analyze_snapshots(
        &deck(vec![entry(VALKI, 1)], vec![entry(MOUNTAIN, 99)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::NotCertified);
    assert!(report.has_finding("CMD-ELIGIBILITY-VARIANT", LegalityStatus::NotCertified));
    assert!(report.has_finding("CMD-LAYOUT-UNSUPPORTED", LegalityStatus::NotCertified));
}

#[test]
fn a_banned_commander_is_illegal_even_when_engine_unsupported() {
    let report = analyze_snapshots(
        &deck(vec![entry(GOLOS, 1)], vec![entry(FOREST, 99)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    assert!(report.has_finding("CMD-FORMAT-BANNED", LegalityStatus::Illegal));
    assert_eq!(
        report.engine_support.status,
        aperture_import::commander::SupportStatus::Unsupported
    );
}

#[test]
fn a_restricted_legality_value_is_not_certified() {
    let cards = synthetic_cards(&[
        synthetic_card(
            "Omnath, Locus of Mana",
            OM.1,
            "Legendary Creature — Elemental",
            &["G"],
            "legal",
        ),
        synthetic_card("Forest", FOREST.1, "Basic Land — Forest", &["G"], "legal"),
        synthetic_card(
            "Restricted Relic",
            "00000000-0000-0000-0000-000000000001",
            "Artifact",
            &[],
            "restricted",
        ),
    ]);
    let report = analyze_snapshots(
        &deck(
            vec![entry(OM, 1)],
            vec![
                entry(FOREST, 98),
                entry(
                    ("Restricted Relic", "00000000-0000-0000-0000-000000000001"),
                    1,
                ),
            ],
        ),
        &cards,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::NotCertified);
    assert!(report.has_finding("CMD-FORMAT-RESTRICTED", LegalityStatus::NotCertified));
}

#[test]
fn missing_color_identity_is_not_certified() {
    let mut card = synthetic_card(
        "Omnath, Locus of Mana",
        OM.1,
        "Legendary Creature — Elemental",
        &["G"],
        "legal",
    );
    card.as_object_mut().unwrap().remove("color_identity");
    let cards = synthetic_cards(&[
        card,
        synthetic_card("Forest", FOREST.1, "Basic Land — Forest", &["G"], "legal"),
    ]);
    let report = analyze_snapshots(
        &deck(vec![entry(OM, 1)], vec![entry(FOREST, 99)]),
        &cards,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::NotCertified);
    assert!(report.has_finding("CMD-METADATA", LegalityStatus::NotCertified));
}

#[test]
fn the_basic_supertype_must_precede_the_type_separator() {
    let cards = synthetic_cards(&[
        synthetic_card(
            "Omnath, Locus of Mana",
            OM.1,
            "Legendary Creature — Elemental",
            &["G"],
            "legal",
        ),
        synthetic_card("Forest", FOREST.1, "Basic Land — Forest", &["G"], "legal"),
        synthetic_card(
            "Fake Basic",
            "00000000-0000-0000-0000-000000000002",
            "Legendary Creature — Basic Land",
            &["G"],
            "legal",
        ),
    ]);
    let report = analyze_snapshots(
        &deck(
            vec![entry(OM, 1)],
            vec![
                entry(("Fake Basic", "00000000-0000-0000-0000-000000000002"), 2),
                entry(FOREST, 97),
            ],
        ),
        &cards,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    assert!(report.has_finding("CMD-SINGLETON", LegalityStatus::Illegal));
}

#[test]
fn empty_color_identity_does_not_excuse_a_basic_land_type() {
    let cards = synthetic_cards(&[
        synthetic_card(
            "Kozilek, Butcher of Truth",
            "00000000-0000-0000-0000-000000000003",
            "Legendary Creature — Eldrazi",
            &[],
            "legal",
        ),
        synthetic_card(
            "Wastes",
            "00000000-0000-0000-0000-000000000004",
            "Basic Land",
            &[],
            "legal",
        ),
        synthetic_card(
            "Colorless Forest",
            "00000000-0000-0000-0000-000000000005",
            "Basic Land — Forest",
            &[],
            "legal",
        ),
    ]);
    let report = analyze_snapshots(
        &deck(
            vec![entry(
                (
                    "Kozilek, Butcher of Truth",
                    "00000000-0000-0000-0000-000000000003",
                ),
                1,
            )],
            vec![
                entry(("Wastes", "00000000-0000-0000-0000-000000000004"), 98),
                entry(
                    ("Colorless Forest", "00000000-0000-0000-0000-000000000005"),
                    1,
                ),
            ],
        ),
        &cards,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    assert!(report.has_finding("CMD-BASIC-LAND-TYPE", LegalityStatus::Illegal));
    assert!(!report.has_finding("CMD-IDENTITY", LegalityStatus::Illegal));
}
