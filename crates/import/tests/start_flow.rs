//! Issue21 start-flow coverage: one certification, two independent axes, and a
//! refusal that never substitutes.
//!
//! These cases are the contract of `aperture_import::start`: the Commander
//! *format* verdict and the *engine support* verdict are produced separately,
//! the seat fit is a third plain answer, and a start is allowed only when the
//! supported list and the selected profile can actually seat it.

use aperture_core::state::Profile;
use aperture_import::commander::{LegalityStatus, SupportStatus};
use aperture_import::start::{certify, certify_text, certify_url, plan_start};
use aperture_import::{ImportError, normalize};
use serde_json::json;

/// 40 supported library cards plus a supported commander: the bounded
/// command-zone seat contract.
const COMMAND_LIST: &str = "\
Commander
1 Isamaru, Hound of Konda
Deck
12 Plains
8 Forest
8 Island
4 Grizzly Bears
4 Centaur Courser
2 Unsummon
2 Swords to Plowshares
";

/// The same shape but with a card the pool does not implement.
const COMMAND_LIST_WITH_UNSUPPORTED: &str = "\
Commander
1 Isamaru, Hound of Konda
Deck
12 Plains
8 Forest
8 Island
4 Grizzly Bears
4 Centaur Courser
2 Unsummon
1 Sol Ring
1 Swords to Plowshares
";

fn archidekt_deck(commander: (&str, &[&str]), library: &[(&str, u8, &[&str])]) -> String {
    let mut cards = vec![json!({
        "quantity": 1,
        "categories": ["Commander"],
        "card": {"oracleCard": {
            "name": commander.0,
            "layout": "normal",
            "colorIdentity": commander.1,
            "superTypes": ["Legendary"],
            "types": ["Creature"],
            "faces": []
        }}
    })];
    for (name, quantity, identity) in library {
        cards.push(json!({
            "quantity": quantity,
            "categories": ["Deck"],
            "card": {"oracleCard": {
                "name": name,
                "layout": "normal",
                "colorIdentity": identity,
                "superTypes": [],
                "types": ["Creature"],
                "faces": []
            }}
        }));
    }
    json!({
        "id": 424242,
        "name": "Start flow probe",
        "deckFormat": 3,
        "private": false,
        "categories": [{"name": "Deck", "includedInDeck": true}],
        "cards": cards
    })
    .to_string()
}

#[test]
fn command_seat_certifies_and_plans_the_exact_list_with_its_commander() {
    let cert = certify_text("P0 command probe", COMMAND_LIST, Profile::CommanderCombat40).unwrap();
    assert!(cert.startable, "refusals: {:?}", cert.refusals);
    assert_eq!(cert.support.status, SupportStatus::Supported);
    assert_eq!(cert.support.supported_cards, 40);
    assert_eq!(cert.support.unsupported_cards, 0);
    assert_eq!(cert.seat.library_cards, 40);
    assert!(cert.seat.compatible);
    assert_eq!(cert.seat.starting_life, 40);
    let commander = cert.commander.as_ref().expect("commander recorded");
    assert_eq!(commander.name, "Isamaru, Hound of Konda");
    assert!(commander.supported);

    let plan = plan_start(&cert).unwrap();
    assert_eq!(plan.deck.size(), 40);
    assert_eq!(plan.deck.name, "P0 command probe");
    assert_eq!(
        plan.commander.map(|c| c.name()),
        Some("Isamaru, Hound of Konda")
    );

    // The format axis is separate and honest: a 41-card deck is not a legal
    // Commander deck even though the bounded profile can seat it.
    assert_eq!(cert.format.status, LegalityStatus::Illegal);
    assert!(
        cert.format
            .checks
            .iter()
            .any(|c| c.code == "deck_size" && c.status == LegalityStatus::Illegal)
    );
}

#[test]
fn format_legality_and_engine_support_are_reported_as_separate_outcomes() {
    let cert = certify_text("P0 command probe", COMMAND_LIST, Profile::CommandZone20).unwrap();
    // Legal-format findings never hide a supported list, and a supported list
    // never pretends the deck is Commander-legal.
    assert_eq!(cert.format.status, LegalityStatus::Illegal);
    assert_eq!(cert.support.status, SupportStatus::Supported);
    assert!(cert.startable);
    assert!(
        cert.notes.iter().any(|n| n.contains("not Commander-legal")),
        "notes must disclose the format verdict: {:?}",
        cert.notes
    );
}

#[test]
fn unsupported_card_is_named_with_its_reason_and_start_is_refused() {
    let cert = certify_text(
        "P0 command probe",
        COMMAND_LIST_WITH_UNSUPPORTED,
        Profile::CommanderCombat40,
    )
    .unwrap();
    assert!(!cert.startable);
    assert_eq!(cert.support.status, SupportStatus::Unsupported);
    let sol_ring = cert
        .support
        .unsupported
        .iter()
        .find(|c| c.name == "Sol Ring")
        .expect("Sol Ring listed as unsupported");
    assert!(
        sol_ring.reason.contains("not in the supported pool"),
        "reason: {}",
        sol_ring.reason
    );
    let refusal = cert.refusals.join(" ");
    assert!(refusal.contains("engine support"), "refusal: {refusal}");
    assert!(refusal.contains("Sol Ring"), "refusal: {refusal}");
    // The refusal never claims a substitution.
    assert!(refusal.contains("never substituted"));

    match plan_start(&cert) {
        Err(ImportError::NotLaunchable(message)) => {
            assert!(message.contains("Sol Ring"), "message: {message}");
        }
        other => panic!("expected a typed refusal, got {other:?}"),
    }
}

#[test]
fn practice_profile_refuses_a_commander_deck_instead_of_dropping_the_commander() {
    let cert = certify_text("P0 command probe", COMMAND_LIST, Profile::Practice20).unwrap();
    assert!(!cert.startable);
    assert_eq!(cert.support.status, SupportStatus::Supported);
    assert!(!cert.seat.compatible);
    let reason = cert.seat.reason.clone().unwrap_or_default();
    assert!(reason.contains("no command zone"), "reason: {reason}");
    assert!(plan_start(&cert).is_err());
}

#[test]
fn practice_profile_still_certifies_a_plain_40_card_list() {
    let text = "\
Deck
12 Forest
8 Mountain
8 Island
4 Grizzly Bears
4 Centaur Courser
4 Shock
";
    let cert = certify_text("P0 practice", text, Profile::Practice20).unwrap();
    assert!(cert.startable, "refusals: {:?}", cert.refusals);
    assert_eq!(cert.support.total_cards, 40);
    assert!(cert.commander.is_none());
    assert_eq!(cert.seat.library_cards, 40);
    let plan = plan_start(&cert).unwrap();
    assert_eq!(plan.deck.size(), 40);
    assert!(plan.commander.is_none());
    // 40 cards is a fine practice seat and a wildly illegal Commander deck.
    assert_eq!(cert.format.status, LegalityStatus::Illegal);
}

#[test]
fn a_wrong_deck_size_is_a_seat_fit_refusal_with_the_numbers() {
    let text = "Deck\n10 Forest\n10 Mountain\n";
    let cert = certify_text("short", text, Profile::Practice20).unwrap();
    assert!(!cert.startable);
    let reason = cert.seat.reason.clone().unwrap_or_default();
    assert!(reason.contains("exactly 40"), "reason: {reason}");
    assert!(reason.contains("has 20"), "reason: {reason}");
}

#[test]
fn provider_color_identity_is_normalized_and_checked_against_the_commander() {
    let legal = archidekt_deck(
        ("Isamaru, Hound of Konda", &["White"]),
        &[
            ("Plains", 36, &["White"]),
            ("Swords to Plowshares", 4, &["White"]),
        ],
    );
    let cert = certify(&normalize(&legal).unwrap(), Profile::CommanderCombat40);
    let identity = cert
        .format
        .checks
        .iter()
        .find(|c| c.code == "color_identity")
        .expect("identity check present");
    assert_eq!(
        identity.status,
        LegalityStatus::Legal,
        "detail: {}",
        identity.detail
    );

    let illegal = archidekt_deck(
        ("Isamaru, Hound of Konda", &["White"]),
        &[
            ("Plains", 36, &["White"]),
            ("Lightning Strike", 4, &["Red"]),
        ],
    );
    let cert = certify(&normalize(&illegal).unwrap(), Profile::CommanderCombat40);
    let identity = cert
        .format
        .checks
        .iter()
        .find(|c| c.code == "color_identity")
        .expect("identity check present");
    assert_eq!(identity.status, LegalityStatus::Illegal);
    assert!(
        identity.detail.contains("Lightning Strike"),
        "{}",
        identity.detail
    );
    // The engine can still seat every card: the two axes disagree, honestly.
    assert_eq!(cert.support.status, SupportStatus::Supported);
    assert!(cert.startable);
}

#[test]
fn a_missing_provider_identity_is_not_certified_rather_than_assumed_legal() {
    // No `colorIdentity` field at all: the provider sent no data, which is not
    // the same as a colorless card.
    let payload = json!({
        "id": 424243,
        "name": "Identity-less probe",
        "deckFormat": 3,
        "private": false,
        "categories": [{"name": "Deck", "includedInDeck": true}],
        "cards": [
            {"quantity": 1, "categories": ["Commander"], "card": {"oracleCard": {
                "name": "Isamaru, Hound of Konda", "layout": "normal",
                "superTypes": ["Legendary"], "types": ["Creature"], "faces": []}}},
            {"quantity": 39, "categories": ["Deck"], "card": {"oracleCard": {
                "name": "Plains", "layout": "normal", "superTypes": ["Basic"],
                "types": ["Land"], "faces": []}}}
        ]
    })
    .to_string();
    let cert = certify(&normalize(&payload).unwrap(), Profile::CommandZone20);
    let identity = cert
        .format
        .checks
        .iter()
        .find(|c| c.code == "color_identity")
        .expect("identity check present");
    assert_eq!(identity.status, LegalityStatus::NotCertified);
    // The aggregate can only be as good as its worst check: a deck with an
    // unknown identity is never reported as a certified-legal deck.
    assert_ne!(cert.format.status, LegalityStatus::Legal);
}

#[test]
fn a_colorless_provider_identity_is_known_and_passes_the_identity_check() {
    // An empty `colorIdentity` array is a real answer: the card is colorless.
    let payload = archidekt_deck(
        ("Isamaru, Hound of Konda", &["White"]),
        &[("Plains", 36, &["White"]), ("Sol Ring", 4, &[])],
    );
    let cert = certify(&normalize(&payload).unwrap(), Profile::CommanderCombat40);
    let identity = cert
        .format
        .checks
        .iter()
        .find(|c| c.code == "color_identity")
        .expect("identity check present");
    assert_eq!(
        identity.status,
        LegalityStatus::Legal,
        "detail: {}",
        identity.detail
    );
}

#[test]
fn a_commander_outside_the_supported_pool_is_refused_with_its_own_reason() {
    let text = "\
Commander
1 Omnath, Locus of Mana
Deck
12 Forest
8 Plains
8 Island
12 Grizzly Bears
";
    let cert = certify_text("P0 unsupported commander", text, Profile::CommanderCombat40).unwrap();
    assert!(!cert.startable);
    let commander = cert.commander.as_ref().expect("commander recorded");
    assert!(!commander.supported);
    assert!(
        commander.reason.contains("not in the supported pool"),
        "reason: {}",
        commander.reason
    );
    assert!(
        cert.refusals.iter().any(|r| r.contains("commander:")),
        "refusals: {:?}",
        cert.refusals
    );
}

#[test]
fn multiple_commanders_are_refused_instead_of_silently_picking_one() {
    let text = "\
Commander
1 Isamaru, Hound of Konda
1 Omnath, Locus of Mana
Deck
40 Plains
";
    let cert = certify_text("P0 partners", text, Profile::CommanderCombat40).unwrap();
    assert!(!cert.startable);
    assert_eq!(cert.format.status, LegalityStatus::Illegal);
    assert_eq!(cert.support.status, SupportStatus::Unsupported);
    assert!(
        cert.refusals
            .iter()
            .any(|r| r.contains("additional designated commander") && r.contains("Omnath")),
        "refusals: {:?}",
        cert.refusals
    );
    // Extra commanders are never folded into the main-deck counts.
    assert_eq!(cert.support.total_cards, 40);
    assert_eq!(cert.support.unsupported_cards, 0);
    assert!(cert.support.unsupported.is_empty());
}

#[test]
fn url_guard_still_runs_before_any_network_work() {
    for bad in [
        "http://archidekt.com/decks/1/",
        "https://evil.example/decks/1/",
        "https://archidekt.com/decks/notanumber/",
    ] {
        let err = certify_url(bad, std::path::Path::new("cache"), Profile::Practice20)
            .expect_err("must be refused");
        assert!(
            matches!(
                err,
                ImportError::BadScheme { .. }
                    | ImportError::HostNotAllowed { .. }
                    | ImportError::BadPath
                    | ImportError::BadDeckId { .. }
                    | ImportError::BadUrl { .. }
            ),
            "unexpected error for {bad}: {err}"
        );
    }
}

#[test]
fn sidelined_and_excluded_cards_never_reach_the_seat_list() {
    let payload = json!({
        "id": 7,
        "name": "Sideboard probe",
        "deckFormat": 3,
        "private": false,
        "categories": [{"name": "Maybeboard", "includedInDeck": false}],
        "cards": [
            {"quantity": 1, "categories": ["Commander"], "card": {"oracleCard": {
                "name": "Isamaru, Hound of Konda", "layout": "normal",
                "colorIdentity": ["White"], "superTypes": ["Legendary"],
                "types": ["Creature"], "faces": []}}},
            {"quantity": 40, "categories": ["Deck"], "card": {"oracleCard": {
                "name": "Plains", "layout": "normal", "colorIdentity": ["White"],
                "superTypes": ["Basic"], "types": ["Land"], "faces": []}}},
            {"quantity": 15, "categories": ["Sideboard"], "card": {"oracleCard": {
                "name": "Swords to Plowshares", "layout": "normal",
                "colorIdentity": ["White"], "superTypes": [], "types": ["Instant"],
                "faces": []}}},
            {"quantity": 3, "categories": ["Maybeboard"], "card": {"oracleCard": {
                "name": "Sol Ring", "layout": "normal", "colorIdentity": [],
                "superTypes": [], "types": ["Artifact"], "faces": []}}}
        ]
    })
    .to_string();
    let cert = certify(&normalize(&payload).unwrap(), Profile::CommanderCombat40);
    assert!(cert.startable, "refusals: {:?}", cert.refusals);
    assert_eq!(cert.support.total_cards, 40);
    assert_eq!(cert.sideboard.len(), 1);
    assert_eq!(cert.excluded.len(), 1);
    assert_eq!(cert.excluded_categories, vec!["Maybeboard".to_string()]);
    let plan = plan_start(&cert).unwrap();
    assert_eq!(plan.deck.size(), 40);
    // Neither the sideboard nor the excluded Maybeboard card is in the seat.
    assert!(
        plan.deck
            .cards
            .iter()
            .all(|(card, _)| card.name() != "Swords to Plowshares" && card.name() != "Sol Ring")
    );
}

#[test]
fn commander_eligibility_is_judged_from_the_provider_type_line_only() {
    let legal = archidekt_deck(
        ("Isamaru, Hound of Konda", &["White"]),
        &[("Plains", 39, &["White"])],
    );
    let cert = certify(&normalize(&legal).unwrap(), Profile::CommanderCombat40);
    let eligibility = cert
        .format
        .checks
        .iter()
        .find(|c| c.code == "commander_eligibility")
        .expect("eligibility check present");
    assert_eq!(eligibility.status, LegalityStatus::Legal);

    // A designated card the provider does not describe as a legendary creature:
    // reported as not_certified, because another commander form needs the card's
    // own permission text.
    let payload = json!({
        "id": 424244,
        "name": "Eligibility probe",
        "deckFormat": 3,
        "private": false,
        "categories": [{"name": "Deck", "includedInDeck": true}],
        "cards": [
            {"quantity": 1, "categories": ["Commander"], "card": {"oracleCard": {
                "name": "Grizzly Bears", "layout": "normal", "colorIdentity": ["Green"],
                "superTypes": [], "types": ["Creature"], "faces": []}}},
            {"quantity": 39, "categories": ["Deck"], "card": {"oracleCard": {
                "name": "Forest", "layout": "normal", "colorIdentity": ["Green"],
                "superTypes": ["Basic"], "types": ["Land"], "faces": []}}}
        ]
    })
    .to_string();
    let cert = certify(&normalize(&payload).unwrap(), Profile::CommanderCombat40);
    let eligibility = cert
        .format
        .checks
        .iter()
        .find(|c| c.code == "commander_eligibility")
        .expect("eligibility check present");
    assert_eq!(eligibility.status, LegalityStatus::NotCertified);
    assert!(
        eligibility.detail.contains("permission text"),
        "detail: {}",
        eligibility.detail
    );
}
