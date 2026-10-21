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
