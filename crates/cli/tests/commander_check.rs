//! Real CLI acceptance for `commander-check`, plus the practice-file
//! laundering regression at the actual CLI file consumers.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const BIN: &str = env!("CARGO_BIN_EXE_aperture-cli");
const OM: (&str, &str) = (
    "Omnath, Locus of Mana",
    "9da896e1-2256-425b-b801-1ae6f0470559",
);
const FOREST: (&str, &str) = ("Forest", "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
const ISLAND: (&str, &str) = ("Island", "b2c6aa39-2d2a-459c-a555-fb48ba993373");
const THRASIOS: (&str, &str) = (
    "Thrasios, Triton Hero",
    "3d867016-2601-4a37-a73d-308898d3bd37",
);
const TYMNA: (&str, &str) = ("Tymna the Weaver", "d15642e4-e61c-4d29-af48-de837991245e");
const FACELESS: (&str, &str) = ("Faceless One", "ef220170-c01a-4ca5-b887-f248c50227ee");

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../import/tests/fixtures/commander/real")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn entry(card: (&str, &str), quantity: u32) -> Value {
    json!({"name": card.0, "oracle_id": card.1, "quantity": quantity})
}

fn temp_file(name: &str, contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "aperture-cli-{name}-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, contents).expect("temp file writes");
    path
}

fn deck(commanders: Vec<Value>, library: Vec<Value>) -> String {
    let zones = json!({
        "commanders": commanders,
        "library": library,
        "companions": [],
        "sideboard": [],
        "excluded": []
    });
    let raw = serde_json::to_string(&zones).unwrap();
    json!({
        "schema": "aperture.commander-deck.v1",
        "provenance": {
            "kind": "constructed",
            "provider": "offline-test-snapshot",
            "source": "constructed by the CLI acceptance test",
            "captured_at": "2026-10-21T05:20:00Z",
            "raw_sha256": sha256_hex(raw.as_bytes())
        },
        "raw_deck": raw
    })
    .to_string()
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("the real aperture-cli binary runs")
}

#[test]
fn commander_check_returns_2_for_duplicates_with_incomplete_oracle_text() {
    let original: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("cards-scryfall.json")).unwrap(),
    )
    .unwrap();
    for name in ["Relentless Rats", "Grizzly Bears"] {
        for text in [None, Some(Value::Null), Some(json!(7))] {
            let mut snapshot = original.clone();
            let mut cards: Vec<Value> = snapshot["raw_cards_jsonl"]
                .as_str()
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            let card = cards.iter_mut().find(|card| card["name"] == name).unwrap();
            let duplicate =
                json!({"name": card["name"], "oracle_id": card["oracle_id"], "quantity": 2});
            match text {
                Some(value) => card["oracle_text"] = value,
                None => {
                    card.as_object_mut().unwrap().remove("oracle_text");
                }
            }
            let raw = cards
                .iter()
                .map(|card| serde_json::to_string(card).unwrap())
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            snapshot["provenance"]["kind"] = json!("synthetic");
            snapshot["provenance"]["provider"] = json!("synthetic");
            snapshot["provenance"]["source"] = json!("R100-1 CLI mutation, not captured metadata");
            snapshot["provenance"]["raw_sha256"] = json!(sha256_hex(raw.as_bytes()));
            snapshot["provenance"]["cards_sha256"] = json!(sha256_hex(raw.as_bytes()));
            snapshot["raw_cards_jsonl"] = json!(raw);
            let commander = if name == "Relentless Rats" {
                (
                    "Vito, Thorn of the Dusk Rose",
                    "7336a541-3ae9-4ab2-acb8-f574c0a5b3a9",
                )
            } else {
                OM
            };
            let land = if name == "Relentless Rats" {
                ("Swamp", "56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
            } else {
                FOREST
            };
            let cards_path = temp_file("incomplete-singleton-cards", &snapshot.to_string());
            let deck_path = temp_file(
                "incomplete-singleton-deck",
                &deck(vec![entry(commander, 1)], vec![duplicate, entry(land, 97)]),
            );
            let out = run(&[
                "commander-check",
                "--deck",
                deck_path.to_str().unwrap(),
                "--cards",
                cards_path.to_str().unwrap(),
                "--json",
            ]);
            std::fs::remove_file(cards_path).unwrap();
            std::fs::remove_file(deck_path).unwrap();
            assert_eq!(
                out.status.code(),
                Some(2),
                "{name}: {}",
                String::from_utf8_lossy(&out.stdout)
            );
            let report: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(report["status"], "not_certified");
            assert_eq!(report["deck"]["total"], 100);
            assert_eq!(report["gameplay"]["available"], false);
            assert!(
                report["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|f| f["code"] == "CMD-SINGLETON" && f["status"] == "not_certified")
            );
            assert!(
                !report["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|f| f["code"] == "CMD-SINGLETON" && f["status"] == "illegal")
            );
        }
    }
}

#[test]
fn commander_check_returns_0_for_a_legal_but_unplayable_deck() {
    let cards = fixtures().join("cards-scryfall.json");
    let deck = fixtures().join("deck-legal-100.json");
    let out = run(&[
        "commander-check",
        "--deck",
        deck.to_str().unwrap(),
        "--cards",
        cards.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).expect("report JSON");
    assert_eq!(report["status"], "legal");
    assert_eq!(report["gameplay"]["available"], false);
    assert_eq!(report["engine_support"]["status"], "unsupported");
    assert_eq!(report["deck"]["total"], 100);
}

#[test]
fn commander_check_returns_1_for_illegal_and_2_for_not_certified() {
    let cards = fixtures().join("cards-scryfall.json");
    let illegal = temp_file(
        "illegal",
        &deck(
            vec![entry(OM, 1)],
            vec![entry(FOREST, 98), entry(ISLAND, 1)],
        ),
    );
    let out = run(&[
        "commander-check",
        "--deck",
        illegal.to_str().unwrap(),
        "--cards",
        cards.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(out.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["status"], "illegal");

    let not_certified = temp_file(
        "partners",
        &deck(
            vec![entry(THRASIOS, 1), entry(TYMNA, 1)],
            vec![entry(FOREST, 98)],
        ),
    );
    let out = run(&[
        "commander-check",
        "--deck",
        not_certified.to_str().unwrap(),
        "--cards",
        cards.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(out.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["status"], "not_certified");
}

#[test]
fn commander_check_returns_2_for_malformed_input_and_out_matches_stdout() {
    let cards = fixtures().join("cards-scryfall.json");
    let mut bad: Value = serde_json::from_str(include_str!(
        "../../import/tests/fixtures/commander/real/deck-legal-100.json"
    ))
    .unwrap();
    bad["schema"] = json!("aperture.commander-deck.v999");
    let bad_path = temp_file("bad-schema", &bad.to_string());
    let out = run(&[
        "commander-check",
        "--deck",
        bad_path.to_str().unwrap(),
        "--cards",
        cards.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("schema"), "{stderr}");

    let legal = fixtures().join("deck-legal-100.json");
    let out_path = temp_file("report", "placeholder");
    let out = run(&[
        "commander-check",
        "--deck",
        legal.to_str().unwrap(),
        "--cards",
        cards.to_str().unwrap(),
        "--json",
        "--out",
        out_path.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        std::fs::read(&out_path).unwrap(),
        out.stdout,
        "--out must carry the same report bytes as stdout"
    );
}

#[test]
fn a_commander_section_in_a_practice_file_is_refused_by_real_consumers() {
    let bad = temp_file("launder", "Commander:\n1 Grizzly Bears\nDeck:\n39 Forest\n");
    let practice = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decks/izzet-control.txt");

    // `coach` loads two explicit files through the real practice file loader.
    let out = run(&[
        "coach",
        "--seed",
        "1",
        "--deck-a",
        bad.to_str().unwrap(),
        "--deck-b",
        practice.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.to_lowercase().contains("commander"), "{stderr}");
    assert!(stderr.contains("practice"), "{stderr}");

    // `selfplay` is the actual launch path named in the defect report.
    let out = run(&[
        "selfplay",
        "--seed",
        "1",
        "--max-steps",
        "5",
        "--deck",
        bad.to_str().unwrap(),
        "--deck",
        practice.to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.to_lowercase().contains("commander"), "{stderr}");
    assert!(
        stderr.contains("practice"),
        "the refusal must explain the 40-card practice scope: {stderr}"
    );
}

#[test]
fn parenthesized_section_headers_are_refused_by_real_consumers() {
    let practice = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decks/izzet-control.txt");
    for header in ["Commander (1):", "Sideboard (15):"] {
        let bad = temp_file(
            "paren-header",
            &format!("{header}\n1 Grizzly Bears\nDeck:\n39 Forest\n"),
        );
        let out = run(&[
            "selfplay",
            "--seed",
            "1",
            "--max-steps",
            "5",
            "--deck",
            bad.to_str().unwrap(),
            "--deck",
            practice.to_str().unwrap(),
        ]);
        assert!(
            !out.status.success(),
            "{header} must not launch a practice game"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.to_lowercase().contains("section") || stderr.contains("header"),
            "{header}: {stderr}"
        );
    }
}

#[test]
fn a_valid_mainboard_prefixed_practice_file_still_loads_and_plays() {
    let practice = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decks/izzet-control.txt");
    let mainboard = temp_file("mainboard", "Mainboard:\n1 Grizzly Bears\n39 Forest\n");

    let out = run(&[
        "coach",
        "--seed",
        "1",
        "--deck-a",
        mainboard.to_str().unwrap(),
        "--deck-b",
        practice.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "Mainboard-prefixed valid practice list must load: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run(&[
        "selfplay",
        "--seed",
        "1",
        "--max-steps",
        "5",
        "--deck",
        mainboard.to_str().unwrap(),
        "--deck",
        practice.to_str().unwrap(),
    ]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unknown section header"),
        "Mainboard-prefixed list must not be refused as an unknown header: {stderr}"
    );
}

#[test]
fn pregame_chosen_color_commander_exits_2_without_false_color_illegal() {
    let cards = fixtures().join("cards-scryfall.json");
    let chosen = temp_file(
        "pregame-color",
        &deck(vec![entry(FACELESS, 1)], vec![entry(FOREST, 99)]),
    );
    let out = run(&[
        "commander-check",
        "--deck",
        chosen.to_str().unwrap(),
        "--cards",
        cards.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["status"], "not_certified");
    let color_illegal = report["findings"].as_array().unwrap().iter().any(|f| {
        matches!(
            f["code"].as_str(),
            Some("CMD-IDENTITY") | Some("CMD-BASIC-LAND-TYPE")
        ) && f["status"] == "illegal"
    });
    assert!(!color_illegal, "{report}");
}

/// R360-4: the `--commander` help is the human's allow-list for reproducing a
/// certified list, so it must name every entry of `CardId::SUPPORTED_COMMANDERS`
/// (Ayula, Queen Among Bears was missing) and must not repeat the retired
/// "no legend rule" claim — CR 704.5j is implemented.
#[test]
fn selfplay_commander_help_lists_every_supported_commander() {
    let out = run(&["selfplay", "--help"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let help = String::from_utf8_lossy(&out.stdout);
    assert!(
        !help.contains("no legend rule"),
        "retired legend-rule claim is still in `selfplay --help`: {help}"
    );
    assert!(
        help.contains("CardId::SUPPORTED_COMMANDERS"),
        "the help no longer names the registry it must track: {help}"
    );
    for id in aperture_core::card::SUPPORTED_COMMANDERS {
        let name = aperture_core::catalog::def(*id).name;
        assert!(
            help.contains(name),
            "`selfplay --help` omits supported commander {name}: {help}"
        );
    }
}
