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

use aperture_core::CardId;
use aperture_core::catalog;
use serde_json::Value;

/// Issue-17 starter package provenance: the captured Scryfall printing records
/// for the 47 vanilla creatures and Jasmine Boreal. Unlike the hand-recorded
/// entries above, this list is data-driven; the fields are the provider's and
/// the same fixture drives the catalog conformance test.
const STARTER_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/sourced-cards.json");
const KRENKO_SOURCED: &str =
    include_str!("../../../fixtures/starter-corpus/krenko-tokens-sourced.json");
const TALRAND_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/talrand-sourced.json");
const TATYOVA_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/tatyova-sourced.json");
const CROSS_SOURCED: &str =
    include_str!("../../../fixtures/starter-corpus/cross-package-sourced.json");
const SLICE4_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice4-sourced.json");
const SLICE8_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice8-sourced.json");
const SLICE9_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice9-sourced.json");
const SLICE12_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice12-sourced.json");
const SLICE13_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice13-sourced.json");
const SLICE14_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice14-sourced.json");
const SLICE17_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice17-sourced.json");
const SLICE18_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice18-sourced.json");
const SLICE20_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice20-sourced.json");
const SLICE21_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice21-sourced.json");
const SLICE24_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice24-sourced.json");
const SLICE25_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice25-sourced.json");
const SLICE30_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice30-sourced.json");
const SLICE41_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice41-sourced.json");
/// Slice 26: Ursine Champion (M19 304), fetched 2026-10-27.
const SLICE26_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice26-sourced.json");
const SLICE22_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice22-sourced.json");
/// Slice 28: Circle of the Moon Druid (AFR 177), fetched 2026-10-27.
const SLICE28_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice28-sourced.json");
/// Slice 29: Stuffed Bear (MID 259), fetched 2026-10-27.
const SLICE29_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice29-sourced.json");
/// Slice 23: Goreclaw, Terror of Qal Sisma (CMM 293), fetched 2026-10-27.
const SLICE23_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice23-sourced.json");
/// Slice 31: Beorn the Fierce (HOB 119), fetched 2026-10-27.
const SLICE31_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice31-sourced.json");
/// Slice 32: Kamahl's Summons (ONS 269) and Surrak and Goreclaw (MOM 337), fetched 2026-10-27.
const SLICE32_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice32-sourced.json");
/// Slice 33: Beorn's Hospitality (HOB 120), fetched 2026-10-27.
const SLICE33_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice33-sourced.json");
const SLICE35_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice35-sourced.json");
/// Slice 37: Druid's Familiar (AVR 175), fetched 2026-10-27.
const SLICE37_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice37-sourced.json");
/// Slice 38: Spider-Ham, Peter Porker (SPM 114), fetched 2026-10-28.
const SLICE38_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice38-sourced.json");
const SLICE15_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice15-sourced.json");
const SLICE16_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice16-sourced.json");
const SLICE5_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice5-sourced.json");
const SLICE6_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice6-sourced.json");
const SLICE11_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice11-sourced.json");
const SLICE19_SOURCED: &str = include_str!("../../../fixtures/starter-corpus/slice19-sourced.json");

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

fn krenko_new_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(KRENKO_SOURCED).expect("sourced Krenko JSON");
    value["cards"]
        .as_array()
        .expect("cards")
        .iter()
        .filter(|card| matches!(card["role"].as_str(), Some("commander" | "token_package")))
        .cloned()
        .collect()
}

fn talrand_card() -> Vec<Value> {
    let value: Value = serde_json::from_str(TALRAND_SOURCED).expect("sourced Talrand JSON");
    value["cards"]
        .as_array()
        .expect("cards")
        .iter()
        .filter(|card| card["role"] == "commander")
        .cloned()
        .collect()
}

fn tatyova_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(TATYOVA_SOURCED).expect("sourced Tatyova JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn cross_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(CROSS_SOURCED).expect("sourced cross JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice4_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE4_SOURCED).expect("sourced slice4 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice5_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE5_SOURCED).expect("sourced slice5 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice6_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE6_SOURCED).expect("sourced slice6 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice8_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE8_SOURCED).expect("sourced slice8 JSON");
    value["cards"]
        .as_array()
        .expect("cards")
        .iter()
        .filter(|card| card["role"] != "excluded")
        .cloned()
        .collect()
}

fn slice9_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE9_SOURCED).expect("sourced slice9 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice11_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE11_SOURCED).expect("sourced slice11 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice12_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE12_SOURCED).expect("sourced slice12 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice13_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE13_SOURCED).expect("sourced slice13 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice14_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE14_SOURCED).expect("sourced slice14 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice15_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE15_SOURCED).expect("sourced slice15 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice16_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE16_SOURCED).expect("sourced slice16 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice17_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE17_SOURCED).expect("sourced slice17 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice18_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE18_SOURCED).expect("sourced slice18 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice19_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE19_SOURCED).expect("sourced slice19 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice20_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE20_SOURCED).expect("sourced slice20 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice21_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE21_SOURCED).expect("sourced slice21 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice22_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE22_SOURCED).expect("sourced slice22 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice23_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE23_SOURCED).expect("sourced slice23 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice24_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE24_SOURCED).expect("sourced slice24 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice25_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE25_SOURCED).expect("sourced slice25 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice26_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE26_SOURCED).expect("sourced slice26 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice28_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE28_SOURCED).expect("sourced slice28 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice29_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE29_SOURCED).expect("sourced slice29 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice30_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE30_SOURCED).expect("sourced slice30 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice31_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE31_SOURCED).expect("sourced slice31 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice32_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE32_SOURCED).expect("sourced slice32 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice33_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE33_SOURCED).expect("sourced slice33 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice35_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE35_SOURCED).expect("sourced slice35 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice37_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE37_SOURCED).expect("sourced slice37 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

fn slice38_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE38_SOURCED).expect("sourced slice38 JSON");
    value["cards"].as_array().expect("cards").to_vec()
}

/// Slice 38 added Spider-Ham, Peter Porker (SPM 114). Its printing is the
/// authority for the cost, the type line, the power/toughness and the whole
/// two-ability oracle text — including the Food token's quoted ability — so a
/// hand-authored string can never drift from the card. All five Scryfall
/// printings (spm 114/201, pspm 114s, pw25 10, om1 118) carry one identical
/// text, type line, cost and P/T (`evidence/issue17/slice38/sources/printings.json`).
#[test]
fn slice38_spider_ham_matches_its_recorded_printing() {
    let cards = slice38_cards();
    assert_eq!(cards.len(), 1, "slice 38 records one card");
    let record = &cards[0];
    assert_eq!(sourced_text(record, "name"), "Spider-Ham, Peter Porker");
    let def = catalog::by_name("Spider-Ham, Peter Porker")
        .expect("in the pool")
        .def();
    assert_eq!(def.oracle, sourced_text(record, "oracle_text"), "oracle");
    assert_eq!(def.type_line, sourced_text(record, "type_line"), "type");
    assert_eq!(def.cost, Some(sourced_text(record, "mana_cost")), "cost");
    assert_eq!(sourced_text(record, "set"), "spm");
    assert_eq!(sourced_text(record, "collector_number"), "114");
    assert_eq!(record["power"].as_str(), Some("2"));
    assert_eq!(record["toughness"].as_str(), Some("2"));
    assert_eq!(
        sourced_text(record, "scryfall_id"),
        "41f18f42-b86b-4a12-9f0d-76b761571195",
        "the immutable printing id, not the oracle id"
    );
    assert!(
        def.oracle.contains(
            "(It's an artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")"
        ),
        "the Food token's quoted ability is part of the printed text"
    );
}

/// Slice 37 added Druid's Familiar (AVR 175). Its printing is the authority for
/// the cost, the type line, the power/toughness and the whole two-ability oracle
/// text, so a hand-authored string can never drift from the card.
#[test]
fn slice37_druids_familiar_matches_its_recorded_printing() {
    let cards = slice37_cards();
    assert_eq!(cards.len(), 1, "slice 37 records one card");
    let record = &cards[0];
    assert_eq!(sourced_text(record, "name"), "Druid's Familiar");
    let def = catalog::by_name("Druid's Familiar")
        .expect("in the pool")
        .def();
    assert_eq!(def.oracle, sourced_text(record, "oracle_text"), "oracle");
    assert_eq!(def.type_line, sourced_text(record, "type_line"), "type");
    assert_eq!(def.cost, Some(sourced_text(record, "mana_cost")), "cost");
    assert_eq!(sourced_text(record, "set"), "avr");
    assert_eq!(sourced_text(record, "collector_number"), "175");
    assert_eq!(record["power"].as_str(), Some("2"));
    assert_eq!(record["toughness"].as_str(), Some("2"));
    assert_eq!(
        sourced_text(record, "scryfall_id"),
        "1d8c794a-7964-41f0-bc9c-27af7cf87aaa",
        "the immutable printing id, not the oracle id"
    );
}

/// Slice 35 added Words of Wilding (ONS 305). Its printing is the authority for
/// the cost, the type line and the whole one-ability oracle text.
#[test]
fn slice35_words_of_wilding_matches_its_recorded_printing() {
    let cards = slice35_cards();
    assert_eq!(cards.len(), 1, "slice 35 records one card");
    let record = &cards[0];
    assert_eq!(sourced_text(record, "name"), "Words of Wilding");
    let def = catalog::by_name("Words of Wilding")
        .expect("in the pool")
        .def();
    assert_eq!(def.oracle, sourced_text(record, "oracle_text"), "oracle");
    assert_eq!(def.type_line, sourced_text(record, "type_line"), "type");
    assert_eq!(def.cost, Some(sourced_text(record, "mana_cost")), "cost");
    assert_eq!(sourced_text(record, "set"), "ons");
    assert_eq!(sourced_text(record, "collector_number"), "305");
    assert_eq!(
        sourced_text(record, "scryfall_id"),
        "fdb9565f-5b09-4127-b169-3146079dab84"
    );
    assert_eq!(
        sourced_text(record, "oracle_id"),
        "888582df-4fe2-4d05-9b62-10c90aaa76f2"
    );
    assert!(def.is_enchantment(), "printed type line");
    assert!(!def.is_creature(), "a printed Enchantment");
}

/// Slice 33 added Beorn's Hospitality (HOB 120). Its printing is the authority
/// for the cost, the type line, the whole two-ability oracle text and the
/// colour; the retrieved stamp and printing URL are recorded so the text can be
/// re-checked against the provider instead of against this file.
#[test]
fn slice33_beorns_hospitality_matches_its_recorded_printing() {
    let cards = slice33_cards();
    assert_eq!(cards.len(), 1, "slice 33 records one card");
    let record = &cards[0];
    assert_eq!(sourced_text(record, "name"), "Beorn's Hospitality");
    let def = catalog::by_name("Beorn's Hospitality")
        .expect("in the pool")
        .def();
    assert_eq!(def.oracle, sourced_text(record, "oracle_text"), "oracle");
    assert_eq!(def.type_line, sourced_text(record, "type_line"), "type");
    assert_eq!(def.cost, Some(sourced_text(record, "mana_cost")), "cost");
    assert_eq!(sourced_text(record, "set"), "hob");
    assert_eq!(sourced_text(record, "collector_number"), "120");
    assert_eq!(
        sourced_text(record, "scryfall_id"),
        "153ca57e-30f0-4ad7-ae9d-c55cbf0fd4c9"
    );
    assert_eq!(def.power, None, "an enchantment prints no power");
    assert_eq!(def.toughness, None, "an enchantment prints no toughness");
    assert_eq!(
        def.colors,
        &[aperture_core::ManaColor::Green],
        "the printing's colour identity is green"
    );
    assert!(def.kinds.contains(&aperture_core::Kind::Enchantment));
    assert_eq!(sourced_text(record, "legalities_commander"), "legal");
    assert!(sourced_text(record, "printing_url").starts_with("https://api.scryfall.com/cards/"));
    assert!(!sourced_text(record, "retrieved_at").is_empty());
    // The two printed sentences are what the typed abilities say.
    let abilities =
        aperture_core::ability::abilities(catalog::by_name("Beorn's Hospitality").unwrap());
    assert_eq!(abilities.len(), 2);
    assert!(
        abilities[0]
            .text()
            .starts_with("Landfall — Whenever a land you control enters"),
        "the trigger text is the printing's first sentence"
    );
    assert!(
        abilities[1]
            .text()
            .starts_with("{5}{G}{G}: This enchantment becomes a Bear creature in addition"),
        "the activated text is the printing's second sentence"
    );
}

/// Only the card slice 32 newly adds to the pool. Surrak and Goreclaw is in the
/// slice-32 fixture to record the printed keyword that repaired it, but its
/// catalog row already existed and is counted by the slice-24 fixture.
fn slice32_new_cards() -> Vec<Value> {
    slice32_cards()
        .into_iter()
        .filter(|card| sourced_text(card, "name") == "Kamahl's Summons")
        .collect()
}

fn slice41_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SLICE41_SOURCED).expect("sourced slice41 JSON");
    value["cards"].as_array().expect("cards").to_vec()
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
fn every_krenko_package_card_matches_its_recorded_printing() {
    let cards = krenko_new_cards();
    assert_eq!(cards.len(), 6, "Krenko plus five token-package cards");
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
        assert!(sourced_text(&card, "printing_url").starts_with("https://api.scryfall.com/cards/"));
        assert_eq!(sourced_text(&card, "legalities_commander"), "legal");
    }
    // The token is engine-only, never in the importable catalog.
    let token: Value = serde_json::from_str::<Value>(KRENKO_SOURCED).unwrap()["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["role"] == "token")
        .cloned()
        .expect("token source record");
    assert_eq!(token["type_line"], "Token Creature — Goblin");
    assert_eq!(token["power"], "1");
    assert_eq!(token["toughness"], "1");
    assert_eq!(
        CardId::GoblinToken.def().type_line,
        token["type_line"].as_str().unwrap()
    );
}

/// Slice 4 adds two cards whose printed text is a modelled keyword, so the
/// catalog must agree with the printing on the keyword *and* on the flag that
/// makes the engine execute it. A silent mismatch (a card that prints Trample
/// while the engine does not trample) is what this guard exists to catch.
#[test]
fn every_slice4_card_matches_its_recorded_printing_and_keywords() {
    let cards = slice4_cards();
    assert_eq!(cards.len(), 4, "slice 4 adds four sourced cards");
    for record in cards {
        let name = sourced_text(&record, "name");
        let def = catalog::by_name(name)
            .unwrap_or_else(|| panic!("{name} is not in the pool"))
            .def();
        assert_eq!(
            def.oracle,
            sourced_text(&record, "oracle_text"),
            "{name} text"
        );
        assert_eq!(
            def.type_line,
            sourced_text(&record, "type_line"),
            "{name} type"
        );
        assert_eq!(
            def.cost,
            Some(sourced_text(&record, "mana_cost")),
            "{name} cost"
        );
        assert_eq!(
            def.power.map(|p| p.to_string()).as_deref(),
            record["power"].as_str(),
            "{name} power"
        );
        assert_eq!(
            def.toughness.map(|t| t.to_string()).as_deref(),
            record["toughness"].as_str(),
            "{name} toughness"
        );
        let printed_flash = record["keywords"]
            .as_array()
            .expect("keywords")
            .iter()
            .any(|keyword| keyword == "Flash");
        let printed_trample = record["keywords"]
            .as_array()
            .expect("keywords")
            .iter()
            .any(|keyword| keyword == "Trample");
        assert_eq!(def.has_flash(), printed_flash, "{name} flash flag");
        assert_eq!(def.has_trample(), printed_trample, "{name} trample flag");
        assert_eq!(sourced_text(&record, "layout"), "normal");
        assert!(
            sourced_text(&record, "printing_url").starts_with("https://api.scryfall.com/cards/")
        );
        assert_eq!(sourced_text(&record, "legalities_commander"), "legal");
        assert_eq!(
            record["distinct_printing_texts"].as_u64(),
            Some(1),
            "{name} wording is identical across its printings"
        );
        assert!(record["printings_checked"].as_u64().unwrap_or(0) >= 1);
    }
}

/// Slice 11 adds two cards whose printed text is Islandwalk, so the catalog must
/// agree with the printing on the keyword *and* on the table the blocking path
/// consults. A card that prints Islandwalk while the engine lets it be blocked,
/// or one that is unblockable without printing it, fails here.
#[test]
fn every_slice11_card_matches_its_recorded_printing_and_islandwalk() {
    let cards = slice11_cards();
    assert_eq!(cards.len(), 2, "slice 11 adds two sourced cards");
    for record in cards {
        let name = sourced_text(&record, "name");
        let id = catalog::by_name(name).unwrap_or_else(|| panic!("{name} is not in the pool"));
        let def = id.def();
        assert_eq!(
            def.oracle,
            sourced_text(&record, "oracle_text"),
            "{name} text"
        );
        assert_eq!(
            def.type_line,
            sourced_text(&record, "type_line"),
            "{name} type"
        );
        assert_eq!(
            def.cost,
            Some(sourced_text(&record, "mana_cost")),
            "{name} cost"
        );
        assert_eq!(
            def.power.map(|p| p.to_string()).as_deref(),
            record["power"].as_str(),
            "{name} power"
        );
        assert_eq!(
            def.toughness.map(|t| t.to_string()).as_deref(),
            record["toughness"].as_str(),
            "{name} toughness"
        );
        assert_eq!(
            def.oracle.matches("Islandwalk").count(),
            1,
            "{name} prints exactly one evasion keyword"
        );
        assert_eq!(
            aperture_core::tokens::keywords(id),
            &[aperture_core::tokens::Keyword::Islandwalk],
            "{name} must be islandwalk in the table the blocker check reads"
        );
        assert_eq!(
            record["distinct_printed_texts"].as_u64(),
            Some(1),
            "{name} wording is identical across its printings"
        );
        assert!(record["prints"].as_u64().unwrap_or(0) >= 1);
        assert_eq!(
            sourced_text(&record, "printing_set"),
            record["printing"]["set"].as_str().expect("set"),
            "{name} records the set it was copied from"
        );
        assert!(
            sourced_text(&record, "scryfall_uri").starts_with("https://scryfall.com/card/"),
            "{name} records a printable source"
        );
    }
}

/// Slice 31 adds Beorn the Fierce. Its printing carries a keyword line
/// ("Trample"), so the catalog's keyword flag must come from the record instead
/// of being asserted by hand: the same invented-text class of defect this file
/// exists to catch (R185-1/R185-2) would otherwise reappear as an invented
/// keyword.
#[test]
fn every_slice31_card_matches_its_recorded_printing_and_trample() {
    let cards = slice31_cards();
    assert_eq!(cards.len(), 1, "slice 31 adds one sourced card");
    for record in &cards {
        let name = sourced_text(record, "name");
        let def = catalog::by_name(name)
            .unwrap_or_else(|| panic!("{name} is not in the pool"))
            .def();
        assert_eq!(
            def.oracle,
            sourced_text(record, "oracle_text"),
            "{name} oracle"
        );
        assert_eq!(
            def.type_line,
            sourced_text(record, "type_line"),
            "{name} type"
        );
        assert_eq!(
            def.cost,
            Some(sourced_text(record, "mana_cost")),
            "{name} cost"
        );
        assert_eq!(
            def.power.map(|p| p.to_string()),
            Some(sourced_text(record, "power").to_string()),
            "{name} power"
        );
        assert_eq!(
            def.toughness.map(|t| t.to_string()),
            Some(sourced_text(record, "toughness").to_string()),
            "{name} toughness"
        );
        assert_eq!(sourced_text(record, "legalities_commander"), "legal");
        assert_eq!(
            card_keywords(record),
            vec!["Trample".to_string()],
            "{name} printed keyword line"
        );
        assert!(def.has_trample(), "{name} prints Trample");
        assert!(
            sourced_text(record, "printing_url").starts_with("https://api.scryfall.com/cards/")
        );
        assert!(!sourced_text(record, "retrieved_at").is_empty());
    }
}

#[test]
fn every_slice32_card_matches_its_recorded_printing() {
    let cards = slice32_cards();
    assert_eq!(cards.len(), 2, "slice 32 records two cards");
    let kamahl = cards
        .iter()
        .find(|card| sourced_text(card, "name") == "Kamahl's Summons")
        .expect("Kamahl's Summons record");
    let def = catalog::by_name("Kamahl's Summons")
        .expect("in the pool")
        .def();
    assert_eq!(def.oracle, sourced_text(kamahl, "oracle_text"), "oracle");
    assert_eq!(def.type_line, sourced_text(kamahl, "type_line"), "type");
    assert_eq!(
        def.cost,
        Some(sourced_text(kamahl, "mana_cost")),
        "mana cost"
    );
    assert_eq!(sourced_text(kamahl, "set"), "ons");
    assert_eq!(sourced_text(kamahl, "collector_number"), "269");
    assert_eq!(def.power, None, "a sorcery has no power");
    assert_eq!(sourced_text(kamahl, "legalities_commander"), "legal");
    assert!(sourced_text(kamahl, "printing_url").starts_with("https://api.scryfall.com/cards/"));
    assert!(!sourced_text(kamahl, "retrieved_at").is_empty());
}

/// Slice 32 repaired an included card's own printed keyword: Surrak and
/// Goreclaw prints Trample, so the pool row must model it (it previously did
/// not, and its static grants trample only to *other* creatures).
#[test]
fn surrak_and_goreclaw_models_the_trample_its_printing_records() {
    let cards = slice32_cards();
    let surrak = cards
        .iter()
        .find(|card| sourced_text(card, "name") == "Surrak and Goreclaw")
        .expect("Surrak and Goreclaw record");
    assert_eq!(sourced_text(surrak, "set"), "mom");
    assert_eq!(sourced_text(surrak, "collector_number"), "337");
    assert_eq!(
        card_keywords(surrak),
        vec!["Trample".to_string()],
        "the printing's keyword list"
    );
    let def = catalog::by_name("Surrak and Goreclaw")
        .expect("in the pool")
        .def();
    assert_eq!(def.oracle, sourced_text(surrak, "oracle_text"));
    assert!(def.has_trample(), "the printed keyword is modelled");
}

#[test]
fn every_pool_card_added_after_issue_15_has_a_recorded_printing() {
    let starter: Vec<Value> = starter_new_cards()
        .into_iter()
        .chain(krenko_new_cards())
        .chain(talrand_card())
        .chain(tatyova_cards())
        .chain(cross_cards())
        .chain(slice4_cards())
        .chain(slice5_cards())
        .chain(slice6_cards())
        .chain(slice8_cards())
        .chain(slice9_cards())
        .chain(slice11_cards())
        .chain(slice12_cards())
        .chain(slice13_cards())
        .chain(slice14_cards())
        .chain(slice15_cards())
        .chain(slice16_cards())
        .chain(slice17_cards())
        .chain(slice18_cards())
        .chain(slice19_cards())
        .chain(slice20_cards())
        .chain(slice21_cards())
        .chain(slice22_cards())
        .chain(slice23_cards())
        .chain(slice24_cards())
        .chain(slice25_cards())
        .chain(slice26_cards())
        .chain(slice28_cards())
        .chain(slice29_cards())
        .chain(slice30_cards())
        .chain(slice41_cards())
        .chain(slice31_cards())
        .chain(slice32_new_cards())
        .chain(slice33_cards())
        .chain(slice35_cards())
        .chain(slice37_cards())
        .chain(slice38_cards())
        .collect();
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

/// Slice 5 adds six cards whose behaviour is new (mill, counter target
/// noncreature spell, scry-then-draw, red rituals), so the catalog string must
/// equal the printing's `oracle_text` and the typed ability/effect must match
/// the printed wording. Seething Song has one prepare-layout printing whose
/// top-level `oracle_text` is empty; every single-face printing carries the same
/// text, which is what the check below asserts.
#[test]
fn every_slice5_card_matches_its_recorded_printing() {
    let cards = slice5_cards();
    assert_eq!(cards.len(), 6, "slice 5 adds six sourced cards");
    for record in cards {
        let name = sourced_text(&record, "name");
        let def = catalog::by_name(name)
            .unwrap_or_else(|| panic!("{name} is not in the pool"))
            .def();
        assert_eq!(
            def.oracle,
            sourced_text(&record, "oracle_text"),
            "{name} text"
        );
        assert_eq!(
            def.type_line,
            sourced_text(&record, "type_line"),
            "{name} type"
        );
        assert_eq!(
            def.cost,
            Some(sourced_text(&record, "mana_cost")),
            "{name} cost"
        );
        assert_eq!(
            def.power.map(|p| p.to_string()).as_deref(),
            record["power"].as_str(),
            "{name} power"
        );
        assert_eq!(
            def.toughness.map(|t| t.to_string()).as_deref(),
            record["toughness"].as_str(),
            "{name} toughness"
        );
        let texts: Vec<&str> = record["distinct_printing_texts_list"]
            .as_array()
            .expect("distinct printing texts")
            .iter()
            .filter_map(|text| text.as_str())
            .filter(|text| !text.is_empty())
            .collect();
        assert_eq!(
            texts,
            vec![def.oracle],
            "{name} prints one rules text across its single-face printings"
        );
        assert!(record["printings_checked"].as_u64().unwrap_or(0) >= 1);
        assert!(
            sourced_text(&record, "printing_url").starts_with("https://api.scryfall.com/cards/")
        );
        assert_eq!(sourced_text(&record, "legalities_commander"), "legal");
        assert!(
            !sourced_text(&record, "retrieved_at").is_empty(),
            "{name} records when it was read"
        );
    }
}

/// Slice 15 adds three cards whose cost is paid with cards, so the catalog text
/// must equal the printing *and* the typed ability must cost exactly what the
/// printing says: one land discarded, two graveyard cards exiled, or the card
/// itself exiled from the graveyard — and only Mother Bear prints "activate only
/// as a sorcery". A hand-authored ability that paid a different cost would fail
/// here rather than in review.
#[test]
fn every_slice15_card_prints_the_cost_its_ability_actually_pays() {
    use aperture_core::ability::{Ability, AbilityCost, abilities};

    let cards = slice15_cards();
    assert_eq!(cards.len(), 3, "slice 15 adds three sourced cards");
    for record in cards {
        let name = sourced_text(&record, "name");
        let def = catalog::by_name(name)
            .unwrap_or_else(|| panic!("{name} is not in the pool"))
            .def();
        assert_eq!(
            def.oracle,
            sourced_text(&record, "oracle_text"),
            "{name} text"
        );
        assert_eq!(
            def.type_line,
            sourced_text(&record, "type_line"),
            "{name} type"
        );
        assert_eq!(
            def.cost,
            Some(sourced_text(&record, "mana_cost")),
            "{name} cost"
        );
        assert_eq!(sourced_text(&record, "legalities_commander"), "legal");
        assert!(
            sourced_text(&record, "printing_url").starts_with("https://api.scryfall.com/cards/")
        );
        assert!(!sourced_text(&record, "retrieved_at").is_empty());
        assert_eq!(
            card_keywords(&record),
            Vec::<String>::new(),
            "{name} prints no keyword line"
        );

        let id = catalog::by_name(name).unwrap();
        let activated: Vec<_> = abilities(id)
            .iter()
            .filter_map(Ability::activated)
            .collect();
        assert_eq!(
            activated.len(),
            1,
            "{name} prints exactly one activated ability"
        );
        let cost: AbilityCost = activated[0].cost;
        let expected = match name {
            "Ayula's Influence" => AbilityCost {
                discard_land: true,
                ..AbilityCost::NONE
            },
            "Bearscape" => AbilityCost {
                mana: Some("{1}{G}"),
                exile_graveyard_cards: 2,
                ..AbilityCost::NONE
            },
            "Mother Bear" => AbilityCost {
                mana: Some("{3}{G}{G}"),
                exile_source_from_graveyard: true,
                ..AbilityCost::NONE
            },
            other => panic!("{other} has no recorded expectation"),
        };
        assert_eq!(cost, expected, "{name} pays exactly the printed cost");
        let expected_sorcery = name == "Mother Bear";
        assert_eq!(
            activated[0].sorcery_timing, expected_sorcery,
            "{name} 'activate only as a sorcery' matches its printing"
        );
        assert!(
            activated[0]
                .text
                .contains(sourced_text(&record, "oracle_text")),
            "{name} ability text is the printed sentence"
        );
    }

    // The Bear token is engine-only, never importable, and prints 2/2 green Bear.
    assert_eq!(CardId::BearToken.def().type_line, "Token Creature — Bear");
    assert_eq!(CardId::BearToken.def().power, Some(2));
    assert_eq!(CardId::BearToken.def().toughness, Some(2));
    assert!(CardId::BearToken.def().is_creature());
    assert!(catalog::by_name("Bear Token").is_none());
}

fn card_keywords(record: &Value) -> Vec<String> {
    record["keywords"]
        .as_array()
        .expect("keywords")
        .iter()
        .filter_map(|keyword| keyword.as_str().map(str::to_string))
        .collect()
}
