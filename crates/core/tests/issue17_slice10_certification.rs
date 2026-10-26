//! Issue 17, slice 10: certify one whole starter list end to end.
//!
//! `jasmine-vanilla-stompy` is the first list whose every card is supported, so
//! this suite is what lets the manifest call it *certified* rather than merely
//! *implemented*: the exact 99-card library on disk is parsed as-is, every card
//! in it is proven supported, every non-vanilla card is actually cast and
//! resolved from that exact card set, and the manifest's certification record is
//! checked against the real file bytes instead of trusted as prose.

mod common;

use std::path::Path;

use aperture_core::catalog;
use aperture_core::*;
use common::*;
use serde_json::Value;
use sha2::{Digest, Sha256};

const DECK: &str = include_str!("../../../decks/starter-corpus/jasmine-vanilla-stompy.txt");
const MANIFEST: &str = include_str!("../../../decks/starter-corpus/manifest.json");

/// The snapshot's text with line endings normalized to LF. A git checkout gives
/// this file CRLF on Windows and LF elsewhere, so the certificate must identify
/// the list's content rather than the checkout's line endings.
fn deck_bytes() -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../decks/starter-corpus/jasmine-vanilla-stompy.txt");
    let raw =
        std::fs::read(&path).unwrap_or_else(|err| panic!("{} is readable: {err}", path.display()));
    let text = String::from_utf8(raw).expect("the snapshot is UTF-8");
    text.replace("\r\n", "\n").into_bytes()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn exact_deck() -> DeckList {
    DeckList::parse_practice("jasmine-vanilla-stompy", DECK).expect("the exact starter parses")
}

fn manifest() -> Value {
    serde_json::from_str(MANIFEST).expect("manifest parses")
}

fn jasmine_entry(manifest: &Value) -> &Value {
    manifest["starters"]
        .as_array()
        .expect("starters")
        .iter()
        .find(|entry| entry["id"] == "jasmine-vanilla-stompy")
        .expect("the jasmine starter entry")
}

/// The exact snapshot is 99 singleton nonland cards plus 41 basic lands, the
/// commander is supplied separately, and every single name resolves in the
/// supported pool. A certification that rests on a bundled replacement or a
/// hand-maintained list would fail here.
#[test]
fn the_exact_certified_snapshot_is_ninety_nine_supported_cards() {
    let deck = exact_deck();
    assert_eq!(
        deck.size(),
        99,
        "99 library cards plus a separate commander"
    );
    let mut nonland = 0usize;
    let mut lands = 0usize;
    for (card, count) in &deck.cards {
        if card.def().is_land() {
            lands += *count as usize;
            assert!(
                card.def().basic_mana.is_some(),
                "{} must be a basic land: this list claims no nonbasic support",
                card.name()
            );
            continue;
        }
        nonland += 1;
        assert_eq!(*count, 1, "{} must be singleton", card.name());
        assert!(
            !card.is_supported_commander(),
            "{} is commander-only and must not sit in the library",
            card.name()
        );
    }
    assert_eq!(
        (nonland, lands),
        (58, 41),
        "58 nonland names + 41 basic lands"
    );
    assert_eq!(nonland + lands, 99);

    let commander = catalog::by_name("Jasmine Boreal").expect("commander supported");
    assert!(commander.is_supported_commander());
    assert_eq!(
        deck.count_of(commander),
        0,
        "the commander is supplied separately, never bundled into the library"
    );

    // Exactly one legendary card across the whole 100-card snapshot: the
    // commander. That is why the unmodelled legend rule is an exclusion rather
    // than a silent gap for this list.
    let mut legendary: Vec<&str> = deck
        .cards
        .iter()
        .map(|(card, _)| card.name())
        .filter(|name| {
            aperture_core::catalog::by_name(name)
                .expect("supported")
                .def()
                .type_line
                .contains("Legendary")
        })
        .collect();
    legendary.push(commander.name());
    assert_eq!(
        legendary,
        vec!["Jasmine Boreal"],
        "the certified snapshot must hold exactly one legendary card"
    );
}

/// Every non-vanilla card in the exact list really is castable and resolves in
/// the real engine from that exact set: membership in the pool is not evidence
/// by itself.
#[test]
fn every_non_vanilla_card_in_the_exact_list_casts_and_resolves() {
    let deck = exact_deck();
    let vanilla: Vec<&str> = [
        "Savannah Lions",
        "Elite Vanguard",
        "Devoted Hero",
        "Eager Cadet",
        "Staunch Shieldmate",
        "Valiant Guard",
        "Yoked Ox",
        "Silvercoat Lion",
        "Glory Seeker",
        "Knight Errant",
        "Oreskos Swiftclaw",
        "Blade of the Sixth Pride",
        "Raptor Companion",
        "Armored Warhorse",
        "Alaborn Trooper",
        "Bastion Enforcer",
        "Loxodon Line Breaker",
        "Pearled Unicorn",
        "Pillarfield Ox",
        "Siege Mastodon",
        "Silent Artisan",
        "Indomitable Ancients",
        "Looming Altisaur",
        "Norwood Ranger",
        "Willow Elf",
        "Trained Jackal",
        "Balduvian Bears",
        "Bear Cub",
        "Cylian Elf",
        "Kalonian Tusker",
        "Swordwise Centaur",
        "Terrain Elemental",
        "Alpine Grizzly",
        "Nessian Courser",
        "Trained Armodon",
        "Golden Bear",
        "Axebane Beast",
        "Hollowhenge Beast",
        "Durkwood Boars",
        "Feral Krushok",
        "Blanchwood Treefolk",
        "Alpha Tyrranax",
        "Vastwood Gorger",
        "Enormous Baloth",
        "Watchwolf",
        "Armored Wolf-Rider",
        "Grizzled Leotau",
    ]
    .to_vec();
    assert_eq!(
        vanilla.len(),
        47,
        "the 47 sourced vanilla creatures are covered by starter_corpus.rs"
    );

    let mut checked = 0usize;
    for &(card, _) in &deck.cards {
        if card.def().is_land() {
            continue;
        }
        let def = card.def();
        let name = card.name();
        let expected_target = match def.spell.map(|spell| spell.target) {
            None => None,
            Some(TargetSpec::None) => Some(Vec::new()),
            Some(TargetSpec::Creature) => Some(vec![Target::Permanent(ObjectId(u32::MAX))]),
            Some(TargetSpec::AnyTarget) | Some(TargetSpec::Player) => {
                Some(vec![Target::Player(P1)])
            }
            Some(other) => panic!("{name} needs an unmodelled target kind: {other:?}"),
        };
        if vanilla.contains(&name) {
            assert_eq!(def.spell, None, "{name} is vanilla");
            continue;
        }
        checked += 1;

        let lands = [CardId::Forest; 14]
            .into_iter()
            .chain([CardId::Plains; 14])
            .collect::<Vec<_>>();
        let mut builder = fixture(31)
            .battlefield(P0, &lands)
            .battlefield(P0, &[CardId::GrizzlyBears])
            .library(P0, &[CardId::Forest; 3])
            .library(P1, &[CardId::Forest; 3])
            .hand(P0, &[card]);
        builder = builder.main(P0, Step::PrecombatMain);
        let mut game = builder.build();
        let target_creature = battlefield_card(&game, P0, CardId::GrizzlyBears);
        let in_hand = hand_card(&game, P0, card);
        let targets = expected_target
            .map(|targets| {
                targets
                    .into_iter()
                    .map(|target| match target {
                        Target::Permanent(_) => Target::Permanent(target_creature),
                        other => other,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let events = submit_ok(
            &mut game,
            P0,
            Action::CastSpell {
                card: in_hand,
                targets: targets.clone(),
            },
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Cast { card: cast, .. } if *cast == card)),
            "{name} must be castable from the exact list's own card set: {events:?}"
        );
        // Entering triggers (Elvish Visionary draws) resolve above the spell, so
        // drain every priority/choice the resolution leaves behind.
        for _ in 0..8 {
            if game.state.stack.is_empty() {
                break;
            }
            let Some(player) = game.decision_owner() else {
                break;
            };
            let action = auto_answer(&game);
            submit_ok(&mut game, player, action);
        }
        assert!(game.state.stack.is_empty(), "{name} resolves off the stack");
        if def.is_creature() {
            let permanent = game
                .state
                .battlefield(P0)
                .into_iter()
                .find(|object| object.card == card)
                .unwrap_or_else(|| panic!("{name} did not become a permanent"));
            assert_eq!(permanent.power(), def.power.expect("power"), "{name} power");
            assert_eq!(
                permanent.toughness(),
                def.toughness.expect("toughness"),
                "{name} toughness"
            );
        }
    }
    assert_eq!(
        checked, 11,
        "the exact list has 11 supported non-vanilla cards to prove"
    );
}

/// The manifest's certification record must describe the file that is actually
/// in the repository. Editing the list without re-certifying fails here, and so
/// does a record whose evidence does not exist or does not back the claim.
#[test]
fn the_manifest_certification_record_matches_the_real_snapshot() {
    let manifest = manifest();
    let entry = jasmine_entry(&manifest);
    assert_eq!(entry["status"], "certified", "jasmine is certified now");
    assert!(entry["missing_cards"].as_array().unwrap().is_empty());
    assert!(entry["missing_interactions"].as_array().unwrap().is_empty());

    let certification = &entry["certification"];
    assert_eq!(certification["verdict"], "certified");
    assert_eq!(
        certification["snapshot_sha256"].as_str().expect("hash"),
        sha256_hex(&deck_bytes()),
        "the recorded snapshot hash must be the exact deck file on disk"
    );
    assert_eq!(certification["library_cards"], 99);
    assert_eq!(certification["total_cards"], 100);
    assert_eq!(certification["commander_supported"], true);
    let seats = certification["seats"].as_u64().expect("seats");
    assert_eq!(
        seats, 4,
        "the certification bullet is about four-seat games"
    );
    let seeds = certification["seeds"].as_array().expect("seeds");
    assert!(!seeds.is_empty(), "at least one certified seed");
    assert_eq!(
        certification["games_terminal"].as_u64().expect("terminal"),
        seeds.len() as u64
    );
    assert_eq!(
        certification["games_replay_verified"]
            .as_u64()
            .expect("replay"),
        seeds.len() as u64
    );

    // Every listed card is supported: the record cannot hide an unsupported name
    // behind a "certified" label.
    for (card, _) in &exact_deck().cards {
        assert!(
            catalog::by_name(card.name()).is_some(),
            "{} must resolve in the supported pool",
            card.name()
        );
    }

    let exclusions = certification["interaction_exclusions"]
        .as_array()
        .expect("exclusions are documented, not implied");
    assert!(
        exclusions
            .iter()
            .filter_map(Value::as_str)
            .any(|text| text.contains("legend rule")),
        "the unmodelled legend rule must be an explicit written exclusion: {exclusions:?}"
    );
    assert!(
        exclusions
            .iter()
            .filter_map(Value::as_str)
            .any(|text| text.contains("equipment")),
        "equipment/Auras must be an explicit written exclusion: {exclusions:?}"
    );

    // The certificate file the record points at must exist, agree with the
    // record, and contain at least one terminal, replay-verified game.
    let evidence_path = certification["evidence"].as_str().expect("evidence path");
    let evidence = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(evidence_path);
    let text = std::fs::read_to_string(&evidence)
        .unwrap_or_else(|err| panic!("{} is readable: {err}", evidence.display()));
    let certificate: Value = serde_json::from_str(&text).expect("certificate parses");
    assert_eq!(certificate["schema"], "aperture.corpus-certificate/1");
    assert_eq!(certificate["verdict"], "certified");
    assert_eq!(certificate["starter"]["id"], "jasmine-vanilla-stompy");
    assert_eq!(
        certificate["starter"]["snapshot_sha256"]
            .as_str()
            .expect("hash"),
        certification["snapshot_sha256"]
            .as_str()
            .expect("record hash")
    );
    let games = certificate["games"].as_array().expect("games");
    assert_eq!(games.len(), seeds.len(), "one certified game per seed");
    for game in games {
        assert_eq!(game["status"], "terminal", "certified games are terminal");
        assert_eq!(
            game["replay_digest_matches"], true,
            "a certified game must replay to the same digest"
        );
        assert_eq!(game["seat_count"], 4);
        assert_eq!(game["starting_life"], 40);
    }
}
