//! Assignment-5 regression: Grist, the Hunger Tide is a legendary planeswalker
//! whose static ability makes it a 1/1 Insect creature while it isn't on the
//! battlefield. It is a legal Commander, but this slice must classify its
//! zone-dependent/type-altering eligibility as `not_certified`, not `illegal`.

use aperture_import::commander::{LegalityStatus, analyze_snapshots};

const CARDS: &str = include_str!("fixtures/commander/real-grist/cards.json");
const DECK: &str = include_str!("fixtures/commander/real-grist/deck.json");
const NOW: u64 = 1_792_560_000;

#[test]
fn grist_zone_dependent_eligibility_is_not_certified_not_illegal() {
    let report = analyze_snapshots(DECK, CARDS, NOW).expect("Grist fixtures bind");
    assert_eq!(
        report.status,
        LegalityStatus::NotCertified,
        "Grist is Commander-legal but its eligibility class is unsupported: {:#?}",
        report.findings
    );
    let commander = &report.commanders[0];
    assert_eq!(commander.name, "Grist, the Hunger Tide");
    assert_eq!(commander.eligibility, LegalityStatus::NotCertified);
    let reason = format!("{} {:#?}", commander.reason, report.findings).to_lowercase();
    assert!(
        reason.contains("off-battlefield") || reason.contains("zone-dependent"),
        "the refusal must name the unsupported class, got: {reason}"
    );
}
