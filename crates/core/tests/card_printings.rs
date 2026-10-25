//! Authoritative-printing provenance for every card this lane added to the pool.
//!
//! R185-1/R185-2 came from card text that was authored in the catalog rather
//! than copied from a printing: `every_printed_ability_maps_to_implemented_
//! semantics` could not catch it because it compares the catalog string with
//! the typed ability string, and both had been written by hand in the same
//! change. This file records the actual printing for every card added from
//! issue 15 onward, so the next invented oracle string fails here instead of in
//! review.
//!
//! Each record was read from Scryfall's `oracle_text`/`power`/`toughness` for
//! the printing named in `set`/`collector_number` on the recorded date, and the
//! whole printing list was checked to make sure the wording did not change
//! between printings (`printings_checked`).
//!
//! Any card that is added to `catalog::all()` after issue 15 must add a record
//! here; the frozen legacy list below exists only because those cards entered
//! the pool before this guard and their strings were accepted in earlier
//! reviewed slices.

use aperture_core::catalog;
use serde_json::Value;

/// Issue-17 starter package provenance: the captured Scryfall printing records
/// for the 47 vanilla creatures and Jasmine Boreal. Unlike the hand-recorded
/// entries above, this list is data-driven; the fields are the provider's and
/// the same fixture drives the catalog conformance test.
const STARTER_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/sourced-cards.json");

fn starter_new_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(STARTER_SOURCED).expect("sourced starter JSON");
    value["cards"]
        .as_array()
        .expect("cards")
        .iter()
        .filter(|card| matches!(card["role"].as_str(), Some("vanilla" | "commander")))
        .cloned()
        .collect()
}

fn sourced_text<'a>(card: &'a Value, key: &str) -> &'a str {
    card[key].as_str().unwrap_or_else(|| panic!("{key}"))
}

/// One authoritative printing of a pool card.
struct Printing {
    name: &'static str,
    /// Scryfall set code of the printing the text was copied from.
    set: &'static str,
    collector_number: &'static str,
    scryfall_uri: &'static str,
    /// How many printings Scryfall listed for this card on the recorded date,
    /// and how many distinct oracle texts they had (1 = wording never changed).
    printings_checked: u32,
    retrieved: &'static str,
    oracle_text: &'static str,
    power: Option<&'static str>,
    toughness: Option<&'static str>,
}

const PRINTINGS: &[Printing] = &[
    Printing {
        name: "Mending Hands",
        set: "9ed",
        collector_number: "29",
        scryfall_uri: "https://scryfall.com/card/9ed/29/mending-hands",
        printings_checked: 3,
        retrieved: "2026-10-24",
        oracle_text: "Prevent the next 4 damage that would be dealt to any target this turn.",
        power: None,
        toughness: None,
    },
    Printing {
        name: "Angrath's Marauders",
        set: "otc",
        collector_number: "153",
        scryfall_uri: "https://scryfall.com/card/otc/153/angraths-marauders",
        printings_checked: 4,
        retrieved: "2026-10-24",
        oracle_text: "If a source you control would deal damage to a permanent or player, it deals double that damage to that permanent or player instead.",
        power: Some("4"),
        toughness: Some("4"),
    },
    Printing {
        name: "Wight of Precinct Six",
        set: "jmp",
        collector_number: "287",
        scryfall_uri: "https://scryfall.com/card/jmp/287/wight-of-precinct-six",
        printings_checked: 8,
        retrieved: "2026-10-24",
        oracle_text: "This creature gets +1/+1 for each creature card in your opponents' graveyards.",
        power: Some("1"),
        toughness: Some("1"),
    },
];

/// Cards that entered the pool before this provenance guard existed. Their
/// strings were reviewed in earlier accepted slices; new cards cannot use this
/// escape hatch.
const LEGACY_POOL: &[&str] = &[
    "Island",
    "Mountain",
    "Forest",
    "Grizzly Bears",
    "Runeclaw Bear",
    "Centaur Courser",
    "Craw Wurm",
    "Hill Giant",
    "Giant Growth",
    "Shock",
    "Lightning Strike",
    "Counterspell",
    "Divination",
    "Opt",
    "Consider",
    "Crack the Earth",
    "Plains",
    "Isamaru, Hound of Konda",
    "Unsummon",
    "Time Ebb",
    "Swords to Plowshares",
    "Fog",
    "Prodigal Sorcerer",
    "Rootwalla",
    "Brindle Boar",
    "Elvish Visionary",
    "Festering Goblin",
    "Oculus",
];

#[test]
fn every_new_pool_card_matches_its_recorded_authoritative_printing() {
    for printing in PRINTINGS {
        let card = catalog::by_name(printing.name)
            .unwrap_or_else(|| panic!("{} is not in the pool", printing.name))
            .def();
        assert_eq!(card.oracle, printing.oracle_text, "{}", printing.name);
        assert_eq!(
            card.power.map(|p| p.to_string()),
            printing.power.map(str::to_string),
            "{} printed power",
            printing.name
        );
        assert_eq!(
            card.toughness.map(|t| t.to_string()),
            printing.toughness.map(str::to_string),
            "{} printed toughness",
            printing.name
        );
        assert!(
            !printing.scryfall_uri.is_empty() && !printing.retrieved.is_empty(),
            "{} records no source",
            printing.name
        );
        assert!(
            !printing.set.is_empty() && !printing.collector_number.is_empty(),
            "{} records no printing identity",
            printing.name
        );
        assert!(
            printing.printings_checked >= 1,
            "{} records no printing check",
            printing.name
        );
    }
}

#[test]
fn every_starter_corpus_card_matches_its_recorded_printing() {
    let cards = starter_new_cards();
    assert_eq!(cards.len(), 48, "47 vanilla creatures plus the commander");
    for card in cards {
        let name = sourced_text(&card, "name");
        let def = catalog::by_name(name)
            .unwrap_or_else(|| panic!("{name} is not in the pool"))
            .def();
        assert_eq!(
            def.oracle,
            sourced_text(&card, "oracle_text"),
            "{name} text"
        );
        assert_eq!(
            def.type_line,
            sourced_text(&card, "type_line"),
            "{name} type"
        );
        assert_eq!(
            def.cost,
            Some(sourced_text(&card, "mana_cost")),
            "{name} cost"
        );
        assert_eq!(
            def.power.map(|p| p.to_string()).as_deref(),
            card["power"].as_str(),
            "{name} power"
        );
        assert_eq!(
            def.toughness.map(|t| t.to_string()).as_deref(),
            card["toughness"].as_str(),
            "{name} toughness"
        );
        assert_eq!(sourced_text(&card, "layout"), "normal");
        assert!(
            sourced_text(&card, "printing_url").starts_with("https://api.scryfall.com/cards/"),
            "{name} records no concrete Scryfall printing"
        );
        assert!(!sourced_text(&card, "retrieved_at").is_empty());
        assert_eq!(sourced_text(&card, "legalities_commander"), "legal");
        assert_eq!(card["keywords"].as_array().map(Vec::len), Some(0));
    }
}

#[test]
fn every_pool_card_added_after_issue_15_has_a_recorded_printing() {
    let starter = starter_new_cards();
    let starter_names: Vec<&str> = starter
        .iter()
        .map(|card| sourced_text(card, "name"))
        .collect();
    for card in catalog::all() {
        let recorded = PRINTINGS.iter().any(|printing| printing.name == card.name);
        let legacy = LEGACY_POOL.contains(&card.name);
        let sourced = starter_names.contains(&card.name);
        assert!(
            recorded || legacy || sourced,
            "{} has no recorded authoritative printing: add a record to \
             crates/core/tests/card_printings.rs or to \
             fixtures/starter-corpus/sourced-cards.json instead of authoring card text by hand",
            card.name
        );
        assert!(
            !(recorded && legacy),
            "{} is both legacy and recorded",
            card.name
        );
        assert!(
            !(sourced && (recorded || legacy)),
            "{} is both a starter-sourced card and an older record",
            card.name
        );
    }
    assert_eq!(
        catalog::all().len(),
        PRINTINGS.len() + LEGACY_POOL.len() + starter.len(),
        "the pool size must equal the recorded, legacy and starter-sourced cards"
    );
}
