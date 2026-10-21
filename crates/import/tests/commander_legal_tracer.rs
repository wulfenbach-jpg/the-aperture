//! Selected vertical behavior: a real 100-card deck with a supported baseline
//! commander must be certified `legal` for the format (never playable).

use aperture_import::commander::{LegalityStatus, SupportStatus, analyze_snapshots};

const REAL_CARDS: &str = include_str!("fixtures/commander/real/cards-scryfall.json");
const LEGAL_DECK: &str = include_str!("fixtures/commander/real/deck-legal-100.json");
const NOW: u64 = 1_792_560_000;

#[test]
fn a_legal_100_card_deck_is_legal_but_not_playable() {
    let report = analyze_snapshots(LEGAL_DECK, REAL_CARDS, NOW).expect("snapshots bind");
    assert_eq!(
        report.status,
        LegalityStatus::Legal,
        "{:#?}",
        report.findings
    );
    assert_eq!(report.deck.total, 100);
    assert_eq!(report.deck.unique_canonical_names, 3);
    assert!(report.has_finding("CMD-COUNT-100", LegalityStatus::Legal));
    assert_eq!(report.format_legality.status, LegalityStatus::Legal);
    assert_eq!(report.engine_support.status, SupportStatus::Unsupported);
    assert!(!report.gameplay.available);
    assert_eq!(report.gameplay.reasons.len(), 5);
}
