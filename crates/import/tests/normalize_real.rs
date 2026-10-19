//! Real-response normalization: the checked-in fixture is the actual Archidekt
//! API response for public deck 26206384, retrieved during planning. Counts
//! below are asserted exactly, not estimated.

use aperture_import::*;

const REAL: &str = include_str!("data/archidekt-26206384.json");

#[test]
fn real_deck_metadata_and_entry_counts_are_exact() {
    let deck = normalize(REAL).expect("the real response normalizes");
    assert_eq!(deck.deck_id, Some(26206384));
    assert_eq!(deck.name, "TTS Test Suite");
    assert_eq!(deck.format_code, Some(7));
    assert!(!deck.private);
    assert_eq!(
        deck.entries.len(),
        28,
        "28 card entries in the provider data"
    );
    assert_eq!(
        deck.entries.iter().map(|e| e.quantity as u32).sum::<u32>(),
        28
    );
    let faceless = deck.entries.iter().filter(|e| e.faces.is_empty()).count();
    assert_eq!(
        faceless, 15,
        "the provider sent 15 entries with no card faces"
    );
    let excluded = deck.excluded_categories.to_vec();
    assert_eq!(excluded, vec!["Maybeboard".to_string()]);
    // No commander category in this deck, and no invented one.
    assert!(
        deck.entries
            .iter()
            .all(|e| !e.categories.iter().any(|c| c.contains("ommand")))
    );
}

#[test]
fn real_deck_is_honestly_unsupported_and_cannot_launch() {
    let deck = normalize(REAL).expect("normalizes");
    let report = analyze(&deck);
    assert_eq!(report.total_cards, 28);
    assert_eq!(
        report.supported_cards, 0,
        "none of these cards are in the pool"
    );
    assert_eq!(report.unsupported_cards, 28);
    assert!(!report.launchable);
    let refusal = report.refusal.clone().unwrap_or_default();
    assert!(refusal.contains("supported pool"), "{refusal}");
    // The old assertion expected every faceless entry to be rejected with a
    // "no card data" reason. That was invalid: a normal single-face card
    // legitimately arrives with an empty faces array and its name/layout are on
    // oracleCard, so faceless "normal" cards now get the pool reason, while the
    // two meld-layout faceless entries are genuinely multi-face.
    let normal_faceless: Vec<&NormEntry> = deck
        .entries
        .iter()
        .filter(|e| e.faces.is_empty() && e.layout.eq_ignore_ascii_case("normal"))
        .collect();
    assert_eq!(normal_faceless.len(), 13, "13 normal faceless entries");
    for entry in &normal_faceless {
        let card = report
            .unsupported
            .iter()
            .find(|u| u.name == entry.name)
            .unwrap_or_else(|| panic!("{} missing from unsupported", entry.name));
        assert!(
            card.reason.contains("not in the supported pool"),
            "{}: {}",
            entry.name,
            card.reason
        );
    }
    for name in ["Hanweir Garrison", "Hanweir Battlements"] {
        let card = report
            .unsupported
            .iter()
            .find(|u| u.name == name)
            .unwrap_or_else(|| panic!("{name} missing from unsupported"));
        assert!(
            card.reason.contains("multi-face"),
            "{name}: {}",
            card.reason
        );
    }
    assert!(plan_deck(&report).is_err());
}
