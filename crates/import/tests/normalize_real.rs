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
    assert!(
        report
            .unsupported
            .iter()
            .any(|u| u.reason.contains("no card data"))
    );
    assert!(plan_deck(&report).is_err());
}
