//! Live provider journey for issue21. `#[ignore]` because it needs the network;
//! run it deliberately and record the result:
//!
//! ```text
//! cargo test -p aperture-import --locked --test start_flow_network -- --ignored --nocapture
//! ```
//!
//! It fetches one real public Archidekt Commander deck through the ordinary
//! guarded URL path and prints the certification. The expected outcome for any
//! real deck at this stage of the roadmap is an honest refusal: the supported
//! pool is 31 cards, so the flow's job is to say exactly which cards are missing
//! and why, not to seat the deck anyway.

use aperture_core::state::Profile;
use aperture_import::commander::{LegalityStatus, SupportStatus};
use aperture_import::start::certify_url;

/// Public Archidekt deck used as the live sample. Fixed so the recorded result
/// is reproducible; if the owner edits or deletes it the test reports that
/// plainly instead of passing on a cached copy.
const REAL_DECK: &str = "https://archidekt.com/decks/27701327/menace";

#[test]
#[ignore = "network: fetches one real public Archidekt deck"]
fn a_real_archidekt_commander_deck_is_certified_with_exact_unsupported_cards() {
    let cache = std::env::var("APERTURE_IMPORT_CACHE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("cache"));
    let cert = certify_url(REAL_DECK, &cache, Profile::CommanderCombat40)
        .expect("the live provider request must succeed (or report the provider error)");

    println!("{}", cert.render());
    let json = serde_json::to_string_pretty(&cert).expect("certification serializes");
    if let Ok(dir) = std::env::var("APERTURE_EVIDENCE_DIR") {
        let path = std::path::Path::new(&dir).join("real-deck-27701327-certification.json");
        std::fs::create_dir_all(&dir).expect("evidence dir");
        std::fs::write(&path, json.clone()).expect("write certification");
        println!("certification written to {}", path.display());
    }

    assert_eq!(cert.provider, "archidekt");
    assert_eq!(cert.deck_id, Some(27_701_327));
    let commander = cert.commander.as_ref().expect("a designated commander");
    assert_eq!(commander.name, "Seifer, Balamb Rival");
    // One real commander, recorded from the provider, and not implemented here.
    assert!(!commander.supported);
    assert!(!commander.reason.is_empty());
    assert_eq!(cert.support.status, SupportStatus::Unsupported);
    // The counts are internally consistent and every unsupported card carries a
    // reason: "exactly which cards and why" is the deliverable, not a number.
    assert_eq!(
        cert.support.supported_cards + cert.support.unsupported_cards,
        cert.support.total_cards
    );
    assert!(
        cert.support.unsupported_cards > 0,
        "a real Commander deck cannot be fully supported by a 31-card pool"
    );
    assert!(
        cert.support.supported_cards > 0,
        "the real deck should contain at least one supported basic land"
    );
    assert!(!cert.startable);
    assert!(
        cert.support
            .unsupported
            .iter()
            .all(|card| !card.name.trim().is_empty() && !card.reason.trim().is_empty()),
        "every unsupported card must be named with its reason"
    );
    let refusal = cert.refusals.join(" ");
    assert!(refusal.contains("never substituted"), "refusal: {refusal}");
    // All four Commander construction checks are evaluated, and the axes stay
    // apart: the format verdict never decides seatability here.
    for code in [
        "commander_present",
        "deck_size",
        "singleton",
        "color_identity",
    ] {
        assert!(
            cert.format.checks.iter().any(|check| check.code == code),
            "missing format check {code}: {:?}",
            cert.format.checks
        );
    }
    assert_eq!(
        cert.format
            .checks
            .iter()
            .find(|c| c.code == "singleton")
            .expect("singleton check")
            .status,
        LegalityStatus::Legal,
        "a real singleton deck must pass the singleton check"
    );
}
