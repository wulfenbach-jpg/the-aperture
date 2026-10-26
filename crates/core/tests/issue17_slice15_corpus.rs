//! Issue 17 slice 15: the manifest's per-list `missing_cards` must equal the
//! pool's actual membership.
//!
//! Until now nothing recomputed that field in-tree, so a slice could add cards
//! to the catalog and leave a stale `missing_cards` entry behind (or claim a
//! name was missing after supporting it). This guard reads every desired list,
//! derives the unsupported names from the compiled pool, and requires the
//! manifest to say exactly that. It also pins each desired library at 99 cards,
//! so bookkeeping cannot quietly edit a snapshot.

use std::collections::BTreeSet;

use aperture_core::catalog;
use serde_json::Value;

const MANIFEST: &str = include_str!("../../../decks/starter-corpus/manifest.json");

/// Every distinct card name in a desired list, ignoring comments and headers.
/// The commander is supplied separately by the manifest, never in the library.
fn library_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut total = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        let (count, name) = line
            .split_once(' ')
            .unwrap_or_else(|| panic!("unparseable deck line: {line}"));
        let count: usize = count
            .parse()
            .unwrap_or_else(|_| panic!("bad count: {line}"));
        total += count;
        names.push(name.trim().to_string());
    }
    assert_eq!(total, 99, "every desired list is exactly 99 library cards");
    names
}

#[test]
fn every_desired_list_missing_cards_equals_pool_membership() {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest JSON");
    let starters = manifest["starters"].as_array().expect("starters");
    assert_eq!(starters.len(), 8);

    for starter in starters {
        let id = starter["id"].as_str().expect("id");
        let path = starter["deck_path"].as_str().expect("deck_path");
        let absolute = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        let text = std::fs::read_to_string(&absolute).unwrap_or_else(|e| panic!("{path}: {e}"));
        let mut names: BTreeSet<String> = library_names(&text).into_iter().collect();
        names.insert(
            starter["commander"]
                .as_str()
                .expect("commander")
                .to_string(),
        );

        let recorded: BTreeSet<String> = starter["missing_cards"]
            .as_array()
            .expect("missing_cards")
            .iter()
            .map(|name| name.as_str().expect("name").to_string())
            .collect();

        // The same card can appear twice in one list only if the list itself
        // prints it twice, which a singleton-99 Commander list never does.
        let mut counts = std::collections::BTreeMap::new();
        for name in library_names(&text) {
            *counts.entry(name).or_insert(0usize) += 1;
        }
        let duplicates: Vec<&str> = counts
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(name, _)| name.as_str())
            .collect();
        assert!(
            duplicates.is_empty(),
            "{id} lists a card twice: {duplicates:?}"
        );

        let actual: BTreeSet<String> = names
            .into_iter()
            .filter(|name| catalog::by_name(name).is_none())
            .collect();

        let stale: Vec<&String> = recorded.difference(&actual).collect();
        let unrecorded: Vec<&String> = actual.difference(&recorded).collect();
        assert!(
            stale.is_empty() && unrecorded.is_empty(),
            "{id}: manifest missing_cards disagrees with the pool\n  \
             recorded but now supported: {stale:?}\n  unsupported but unrecorded: {unrecorded:?}"
        );
    }
}

#[test]
fn the_three_slice15_cards_leave_the_ayula_gap() {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest JSON");
    let ayula = manifest["starters"]
        .as_array()
        .expect("starters")
        .iter()
        .find(|starter| starter["id"] == "ayula-bear-counters")
        .expect("ayula list");
    let missing: Vec<&str> = ayula["missing_cards"]
        .as_array()
        .expect("missing_cards")
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    for name in ["Ayula's Influence", "Bearscape", "Mother Bear"] {
        assert!(
            !missing.contains(&name),
            "{name} is supported now, so it cannot be listed as missing"
        );
    }
    assert_eq!(missing.len(), 36, "Ayula's remaining sourced gap");
    assert_eq!(catalog::all().len(), 129, "the compiled pool");
}
