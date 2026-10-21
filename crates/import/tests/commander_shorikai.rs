//! Assignment-5 regression: CR 903.3 permits a Vehicle commander with a
//! power/toughness box. The slice does not model that category, so it must be
//! `not_certified` rather than falsely `illegal`.

use aperture_import::commander::{LegalityStatus, analyze_snapshots};

const CARDS: &str = include_str!("fixtures/commander/real/cards-scryfall.json");
const DECK: &str = include_str!("fixtures/commander/real-shorikai/deck.json");
const NOW: u64 = 1_792_560_000;

#[test]
fn a_vehicle_commander_is_not_certified_not_illegal() {
    let report = analyze_snapshots(DECK, CARDS, NOW).expect("Shorikai fixtures bind");
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "a CR 903.3 Vehicle commander is unsupported, not illegal: {:#?}",
        report.findings
    );
    assert_eq!(
        report.commanders[0].eligibility,
        LegalityStatus::NotCertified
    );
    let reason = format!("{} {:#?}", report.commanders[0].reason, report.findings).to_lowercase();
    assert!(
        reason.contains("vehicle") || reason.contains("903.3"),
        "the refusal must name the unsupported Vehicle category, got: {reason}"
    );
}
