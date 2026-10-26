//! Issue-17 planned corpus structure: eight target ids, exact desired 100-card
//! lists, honest missing-card accounting. Planned lists are metadata only and
//! are never loaded by the engine; the engine's practice parser still refuses
//! every unsupported name.

use std::collections::BTreeSet;
use std::path::PathBuf;

use aperture_core::catalog;
use aperture_import::commander::{LegalityStatus, SupportStatus, analyze_snapshots};
use serde_json::Value;

const MANIFEST: &str = include_str!("../../../decks/starter-corpus/manifest.json");
const NOW: u64 = 1_793_000_000;
const BASICS: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Parse the planned-list text format only. This is deliberately not
/// `DeckList::parse_practice`: planned cards are not supported by the engine and
/// must never be mistaken for a playable deck.
fn parse_planned(text: &str) -> Vec<(String, u8)> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        let (quantity, name) = line
            .split_once(' ')
            .unwrap_or_else(|| panic!("bad planned line: {line}"));
        out.push((
            name.trim().to_string(),
            quantity
                .parse()
                .unwrap_or_else(|_| panic!("bad quantity: {line}")),
        ));
    }
    out
}

#[test]
fn eight_targets_have_exact_desired_lists_and_honest_missing_cards() {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest parses");
    assert_eq!(manifest["version"], 1);
    let starters = manifest["starters"].as_array().expect("starters");
    assert_eq!(starters.len(), 8);
    let expected_ids = [
        "isamaru-white-aggro",
        "jasmine-vanilla-stompy",
        "talrand-draw-go",
        "krenko-goblin-tokens",
        "ayula-bear-counters",
        "elas-sacrifice",
        "tatyova-landfall",
        "zada-spells",
    ];
    let ids: Vec<&str> = starters
        .iter()
        .map(|starter| starter["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, expected_ids);

    let mut planned = 0;
    for starter in starters {
        let id = starter["id"].as_str().unwrap();
        let status = starter["status"].as_str().unwrap();
        assert!(matches!(status, "planned" | "implemented" | "certified"));
        if id != "jasmine-vanilla-stompy" {
            assert_ne!(status, "certified", "{id} is not certified");
        }
        let deck_path = starter["deck_path"]
            .as_str()
            .unwrap_or_else(|| panic!("{id} has no exact desired list"));
        let commander = starter["commander"].as_str().unwrap();
        let entries = parse_planned(&read(deck_path));
        let total: u32 = entries.iter().map(|(_, q)| u32::from(*q)).sum();
        assert_eq!(total, 99, "{id} library");
        let mut basics = 0u32;
        let mut nonbasic = BTreeSet::new();
        for (name, quantity) in &entries {
            if BASICS.contains(&name.as_str()) {
                basics += u32::from(*quantity);
                continue;
            }
            assert_eq!(*quantity, 1, "{id}: {name} must be singleton");
            assert!(nonbasic.insert(name.clone()), "{id}: duplicate {name}");
        }
        assert!((38..=43).contains(&basics), "{id} basic lands {basics}");
        assert!(
            (56..=61).contains(&nonbasic.len()),
            "{id}: {} unique nonbasic names",
            nonbasic.len()
        );
        assert_eq!(
            total - basics,
            nonbasic.len() as u32,
            "{id}: nonbasic total"
        );

        let actual_missing: BTreeSet<String> = std::iter::once(commander.to_string())
            .chain(entries.iter().map(|(name, _)| name.clone()))
            .filter(|name| catalog::by_name(name).is_none())
            .collect();
        let declared: BTreeSet<String> = starter["missing_cards"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            declared, actual_missing,
            "{id}: missing_cards must match the current catalog"
        );

        if id == "jasmine-vanilla-stompy" {
            // Slice 10: the one list whose every card is supported is certified
            // by an exact-snapshot four-seat certificate, not by catalog count.
            assert_eq!(status, "certified");
            let certification = &starter["certification"];
            assert_eq!(certification["verdict"], "certified");
            assert_eq!(certification["total_cards"], 100);
            assert_eq!(certification["seats"], 4);
            assert!(declared.is_empty(), "a certified list has no missing cards");
            continue;
        }
        planned += 1;
        assert_eq!(status, "planned", "{id} is planned, not supported");
        assert!(!declared.is_empty(), "{id} must name its unsupported cards");

        let deck_json = read(&format!("fixtures/starter-corpus/planned/{id}.deck.json"));
        let cards_json = read(&format!("fixtures/starter-corpus/planned/{id}.cards.json"));
        let report = analyze_snapshots(&deck_json, &cards_json, NOW)
            .unwrap_or_else(|e| panic!("{id} analyzer: {e}"));
        assert_eq!(report.deck.total, 100, "{id} analyzer total");
        assert_eq!(
            report.status,
            LegalityStatus::Legal,
            "{id} format: {:#?}",
            report.findings
        );
        assert_eq!(
            report.engine_support.status,
            SupportStatus::Unsupported,
            "{id} must not claim support it does not have"
        );
        assert!(!report.engine_support.unsupported_cards.is_empty());
        assert!(!report.gameplay.available);
    }
    assert_eq!(planned, 7);
}
