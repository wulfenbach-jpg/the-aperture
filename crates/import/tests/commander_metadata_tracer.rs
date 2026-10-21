//! Selected behaviors: missing metadata is not_certified, and unsupported
//! commander variants / non-normal layouts are refused rather than certified.

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
const FACELESS: (&str, &str) = ("Faceless One", "ef220170-c01a-4ca5-b887-f248c50227ee");
const WASTES: (&str, &str) = ("Wastes", "05d24b0c-904a-46b6-b42a-96a4d91a0dd4");
const ISLAND: (&str, &str) = ("Island", "b2c6aa39-2d2a-459c-a555-fb48ba993373");
const TALRAND: (&str, &str) = (
    "Talrand, Sky Summoner",
    "ea1eb902-a23c-44ff-9169-19baf71de238",
);
const SCHOLAR: (&str, &str) = (
    "Civilized Scholar // Homicidal Brute",
    "8662c9be-2244-4ea3-bf80-31a117f40a59",
);

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
        "provenance": {"kind": "constructed", "provider": "offline-test-snapshot", "source": "metadata tracer", "captured_at": "2026-10-21T05:20:00Z", "raw_sha256": digest},
        "raw_deck": raw}).to_string()
}
fn synthetic_card(
    name: &str,
    oracle: &str,
    type_line: &str,
    ci: &[&str],
    legality: Option<&str>,
    text: &str,
) -> Value {
    let mut card = json!({"object": "card", "id": format!("id-{name}"), "oracle_id": oracle,
        "name": name, "layout": "normal", "type_line": type_line, "color_identity": ci,
        "keywords": [], "oracle_text": text});
    if let Some(legality) = legality {
        card["legalities"] = json!({"commander": legality});
    }
    card
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
        "provenance": {"kind": "synthetic", "provider": "synthetic", "source": "metadata tracer", "captured_at": "2026-10-21T05:20:00Z", "cards_sha256": digest, "raw_sha256": digest, "card_count": cards.len()},
        "raw_cards_jsonl": raw}).to_string()
}

#[test]
fn missing_card_metadata_is_not_certified_not_guessed_legal() {
    let cards = synthetic_cards(&[
        synthetic_card(
            "Omnath, Locus of Mana",
            OM.1,
            "Legendary Creature — Elemental",
            &["G"],
            Some("legal"),
            "",
        ),
        synthetic_card(
            "Forest",
            FOREST.1,
            "Basic Land — Forest",
            &["G"],
            Some("legal"),
            "",
        ),
        synthetic_card(
            "Mystery Card",
            "00000000-0000-0000-0000-000000000001",
            "Creature — Bear",
            &["G"],
            None,
            "",
        ),
    ]);
    let deck = deck(
        vec![entry(OM, 1)],
        vec![
            entry(FOREST, 98),
            entry(("Mystery Card", "00000000-0000-0000-0000-000000000001"), 1),
        ],
    );
    let report = analyze_snapshots(&deck, &cards, NOW).expect("snapshots bind");
    assert_eq!(report.status, LegalityStatus::NotCertified);
    assert!(
        report.has_finding("CMD-METADATA", LegalityStatus::NotCertified),
        "{:#?}",
        report.findings
    );
}

#[test]
fn a_partner_commander_is_not_certified() {
    let deck = deck(vec![entry(THRASIOS, 1)], vec![entry(FOREST, 99)]);
    let report = analyze_snapshots(&deck, REAL_CARDS, NOW).expect("snapshots bind");
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-ELIGIBILITY-VARIANT", LegalityStatus::NotCertified));
}

#[test]
fn a_pregame_chosen_color_commander_is_not_certified() {
    let deck = deck(vec![entry(FACELESS, 1)], vec![entry(WASTES, 99)]);
    let report = analyze_snapshots(&deck, REAL_CARDS, NOW).expect("snapshots bind");
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-ELIGIBILITY-VARIANT", LegalityStatus::NotCertified));
}

#[test]
fn a_non_normal_library_card_is_not_certified_with_face_evidence() {
    let deck = deck(
        vec![entry(TALRAND, 1)],
        vec![entry(ISLAND, 98), entry(SCHOLAR, 1)],
    );
    let report = analyze_snapshots(&deck, REAL_CARDS, NOW).expect("snapshots bind");
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
    assert!(report.has_finding("CMD-LAYOUT-UNSUPPORTED", LegalityStatus::NotCertified));
    let evidence = report
        .card_evidence
        .iter()
        .find(|card| card.name == SCHOLAR.0)
        .expect("face evidence retained");
    assert_eq!(evidence.top_level_color_identity, vec!["R", "U"]);
    assert_eq!(evidence.faces.len(), 2);
}
