//! Executable acceptance coverage migrated from the design matrix.
//! Some cases cover behavior that was already implemented before this test was
//! written; those are regression coverage, not claimed TDD evidence.

use aperture_import::commander::{CommanderError, LegalityStatus, analyze_snapshots};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const REAL_CARDS: &str = include_str!("fixtures/commander/real/cards-scryfall.json");
const LEGAL_DECK: &str = include_str!("fixtures/commander/real/deck-legal-100.json");
const NOW: u64 = 1_792_560_000;
const OM: (&str, &str) = (
    "Omnath, Locus of Mana",
    "9da896e1-2256-425b-b801-1ae6f0470559",
);
const FOREST: (&str, &str) = ("Forest", "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
const GOLOS: (&str, &str) = (
    "Golos, Tireless Pilgrim",
    "efe9c3e0-7315-408e-9903-9d25a681039b",
);
const WILSON: (&str, &str) = (
    "Wilson, Refined Grizzly",
    "d2766fd7-5cf9-4037-9f34-9ae3982c613a",
);
const RAISED: (&str, &str) = ("Raised by Giants", "1682cf24-17a3-49ad-8b6f-9b7f13ebf53c");
const KARLACH: (&str, &str) = (
    "Karlach, Fury of Avernus",
    "037355be-71e7-4866-80a6-80352c304970",
);
const MOUNTAIN: (&str, &str) = ("Mountain", "a3fb7228-e76b-4e96-a40e-20b5fed75685");

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
        "provenance": {"kind": "constructed", "provider": "offline-test-snapshot", "source": "acceptance coverage", "captured_at": "2026-10-21T05:20:00Z", "raw_sha256": sha256_hex(raw.as_bytes())},
        "raw_deck": raw}).to_string()
}
fn synthetic_card(name: &str, oracle: &str, type_line: &str, ci: &[&str]) -> Value {
    json!({"object": "card", "id": format!("id-{name}"), "oracle_id": oracle, "name": name,
        "layout": "normal", "type_line": type_line, "color_identity": ci,
        "legalities": {"commander": "legal"}, "keywords": [], "oracle_text": ""})
}
fn synthetic_cards(cards: &[Value]) -> String {
    let raw = cards
        .iter()
        .map(|c| serde_json::to_string(c).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let digest = sha256_hex(raw.as_bytes());
    json!({"schema": "aperture.card-metadata.v1",
        "provenance": {"kind": "synthetic", "provider": "synthetic", "source": "acceptance coverage", "captured_at": "2026-10-21T05:20:00Z", "cards_sha256": digest, "raw_sha256": digest, "card_count": cards.len()},
        "raw_cards_jsonl": raw}).to_string()
}

#[test]
fn count_boundaries_are_99_illegal_100_legal_101_illegal() {
    let cases = [
        (97, 1, LegalityStatus::Illegal, 99),
        (98, 1, LegalityStatus::Legal, 100),
        (99, 1, LegalityStatus::Illegal, 101),
    ];
    for (forests, bears, expected, total) in cases {
        let report = analyze_snapshots(
            &deck(
                vec![entry(OM, 1)],
                vec![
                    entry(FOREST, forests),
                    entry(
                        ("Grizzly Bears", "14c8f55d-d177-4c25-a931-ebeb9e6062a0"),
                        bears,
                    ),
                ],
            ),
            REAL_CARDS,
            NOW,
        )
        .unwrap();
        assert_eq!(report.deck.total, total);
        assert_eq!(
            report.status, expected,
            "total {total}: {:#?}",
            report.findings
        );
    }
}

#[test]
fn a_commander_quantity_other_than_one_is_illegal() {
    let report = analyze_snapshots(
        &deck(vec![entry(OM, 2)], vec![entry(FOREST, 98)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(report.deck.total, 100);
    assert_eq!(report.status, LegalityStatus::Illegal);
}

#[test]
fn incomplete_or_unexpected_face_metadata_is_not_certified() {
    let mut broken = synthetic_card(
        "Broken DFC",
        "00000000-0000-0000-0000-000000000010",
        "Creature — Human // Creature — Insect",
        &["U"],
    );
    broken["layout"] = json!("transform");
    broken["card_faces"] = json!([{"name": "Front", "type_line": "Creature — Human"}]);
    let cards = synthetic_cards(&[
        broken.clone(),
        synthetic_card(
            "Talrand, Sky Summoner",
            "00000000-0000-0000-0000-000000000011",
            "Legendary Creature — Merfolk Wizard",
            &["U"],
        ),
        synthetic_card(
            "Island",
            "00000000-0000-0000-0000-000000000012",
            "Basic Land — Island",
            &["U"],
        ),
    ]);
    let report = analyze_snapshots(
        &deck(
            vec![entry(
                (
                    "Talrand, Sky Summoner",
                    "00000000-0000-0000-0000-000000000011",
                ),
                1,
            )],
            vec![
                entry(("Island", "00000000-0000-0000-0000-000000000012"), 98),
                entry(("Broken DFC", "00000000-0000-0000-0000-000000000010"), 1),
            ],
        ),
        &cards,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::NotCertified);

    let mut normal_with_faces = synthetic_card(
        "Normal With Faces",
        "00000000-0000-0000-0000-000000000013",
        "Creature — Bear",
        &["G"],
    );
    normal_with_faces["card_faces"] = json!([{"name": "Bear", "type_line": "Creature — Bear"}]);
    let cards = synthetic_cards(&[
        normal_with_faces,
        synthetic_card(
            "Omnath, Locus of Mana",
            OM.1,
            "Legendary Creature — Elemental",
            &["G"],
        ),
        synthetic_card("Forest", FOREST.1, "Basic Land — Forest", &["G"]),
    ]);
    let report = analyze_snapshots(
        &deck(
            vec![entry(OM, 1)],
            vec![
                entry(FOREST, 98),
                entry(
                    ("Normal With Faces", "00000000-0000-0000-0000-000000000013"),
                    1,
                ),
            ],
        ),
        &cards,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::NotCertified);
}

#[test]
fn background_variants_are_not_certified() {
    let cases = [
        (vec![entry(WILSON, 1), entry(RAISED, 1)], FOREST, 98),
        (vec![entry(RAISED, 1)], FOREST, 99),
        (vec![entry(WILSON, 1)], FOREST, 99),
        (vec![entry(KARLACH, 1)], MOUNTAIN, 99),
    ];
    for (commanders, basic, count) in cases {
        let report = analyze_snapshots(
            &deck(commanders, vec![entry(basic, count)]),
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
    }
}

#[test]
fn banned_cards_are_illegal_and_staleness_is_disclosed_without_changing_the_ban_result() {
    let report = analyze_snapshots(
        &deck(vec![entry(GOLOS, 1)], vec![entry(FOREST, 99)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    assert_eq!(report.format_legality.status, LegalityStatus::Illegal);

    let mut cards: Value = serde_json::from_str(REAL_CARDS).unwrap();
    cards["provenance"]["captured_at"] = json!("2026-09-01T00:00:00Z");
    let report = analyze_snapshots(LEGAL_DECK, &cards.to_string(), NOW).unwrap();
    assert!(report.format_legality.stale);
    assert!(report.format_legality.age_days > 30);
    assert_eq!(report.status, LegalityStatus::Legal);
}

#[test]
fn missing_or_tampered_snapshot_metadata_fails_closed() {
    let mut missing_legalities = synthetic_card(
        "Mystery Card",
        "00000000-0000-0000-0000-000000000020",
        "Creature — Bear",
        &["G"],
    );
    missing_legalities
        .as_object_mut()
        .unwrap()
        .remove("legalities");
    let cards = synthetic_cards(&[
        missing_legalities,
        synthetic_card(
            "Omnath, Locus of Mana",
            OM.1,
            "Legendary Creature — Elemental",
            &["G"],
        ),
        synthetic_card("Forest", FOREST.1, "Basic Land — Forest", &["G"]),
    ]);
    let report = analyze_snapshots(
        &deck(
            vec![entry(OM, 1)],
            vec![
                entry(FOREST, 98),
                entry(("Mystery Card", "00000000-0000-0000-0000-000000000020"), 1),
            ],
        ),
        &cards,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::NotCertified);

    let mut cards: Value = serde_json::from_str(REAL_CARDS).unwrap();
    cards["provenance"]["cards_sha256"] = json!("0".repeat(64));
    assert!(matches!(
        analyze_snapshots(LEGAL_DECK, &cards.to_string(), NOW),
        Err(CommanderError::Input(_))
    ));

    let mut cards: Value = serde_json::from_str(REAL_CARDS).unwrap();
    cards["raw_cards_jsonl"] = json!("");
    cards["provenance"]["cards_sha256"] = json!(sha256_hex(b""));
    assert!(matches!(
        analyze_snapshots(LEGAL_DECK, &cards.to_string(), NOW),
        Err(CommanderError::Input(_))
    ));
}

#[test]
fn a_snapshot_without_an_eligible_commander_never_claims_legal() {
    let report = analyze_snapshots(
        &deck(vec![], vec![entry(FOREST, 99), entry(OM, 1)]),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
    let report = analyze_snapshots(
        &deck(
            vec![entry(
                ("Grizzly Bears", "14c8f55d-d177-4c25-a931-ebeb9e6062a0"),
                1,
            )],
            vec![entry(FOREST, 99)],
        ),
        REAL_CARDS,
        NOW,
    )
    .unwrap();
    assert_eq!(report.status, LegalityStatus::Illegal);
}

#[test]
fn a_printed_name_alias_with_the_same_oracle_is_not_certified() {
    // Scryfall represents an interchangeable/alternate printed name via
    // `printed_name` on a printing of the SAME oracle_id. This slice must not
    // use that alias to certify identity; the mismatched canonical name is NC.
    let mut commander = synthetic_card(
        "Greymond, Avacyn's Stalwart",
        "739a5220-50cd-4148-be58-1bbdeef2954e",
        "Legendary Creature — Human Soldier",
        &["W"],
    );
    commander["printed_name"] = json!("Rick, Steadfast Leader");
    let cards = synthetic_cards(&[
        commander,
        synthetic_card(
            "Wastes",
            "05d24b0c-904a-46b6-b42a-96a4d91a0dd4",
            "Basic Land",
            &[],
        ),
    ]);
    let deck = deck(
        vec![
            json!({"name": "Rick, Steadfast Leader", "oracle_id": "739a5220-50cd-4148-be58-1bbdeef2954e", "quantity": 1}),
        ],
        vec![
            json!({"name": "Wastes", "oracle_id": "05d24b0c-904a-46b6-b42a-96a4d91a0dd4", "quantity": 99}),
        ],
    );
    let report = analyze_snapshots(&deck, &cards, NOW).unwrap();
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "{:#?}",
        report.findings
    );
}
