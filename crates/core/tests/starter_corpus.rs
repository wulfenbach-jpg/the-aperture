//! Issue 17, first starter package: sourced green/white vanilla creatures and the
//! Jasmine Boreal commander. The deck/card declarations are captured Scryfall
//! printing data (`fixtures/starter-corpus/sourced-cards.json`); this test is the
//! engine-side conformance check, not a second legality engine.

mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;
use serde_json::Value;

const SOURCED: &str = include_str!("../../../fixtures/starter-corpus/sourced-cards.json");
const DECK: &str = include_str!("../../../decks/starter-corpus/jasmine-vanilla-stompy.txt");
const MANIFEST: &str = include_str!("../../../decks/starter-corpus/manifest.json");

fn sourced_cards() -> Vec<Value> {
    let value: Value = serde_json::from_str(SOURCED).expect("sourced-cards.json parses");
    assert_eq!(value["schema"], "aperture.starter-sourced-cards.v1");
    assert_eq!(value["provider"], "scryfall");
    value["cards"].as_array().expect("cards array").to_vec()
}

fn role(card: &Value) -> &str {
    card["role"].as_str().expect("role")
}

fn text<'a>(card: &'a Value, key: &str) -> &'a str {
    card[key].as_str().unwrap_or_else(|| panic!("{key}"))
}

fn number(card: &Value, key: &str) -> i32 {
    card[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key}"))
        .parse()
        .unwrap_or_else(|_| panic!("{key} is an integer"))
}

fn starter_deck() -> DeckList {
    DeckList::parse_practice("jasmine-vanilla-stompy", DECK).expect("the starter library parses")
}

fn add_permanent(game: &mut Game, p: PlayerId, card: CardId) -> ObjectId {
    let id = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card,
            owner: p,
            controller: p,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: Default::default(),
        },
    );
    id
}

fn commander_game() -> Game {
    let decks = vec![starter_deck(), starter_deck()];
    let specs = [
        (P0, catalog::by_name("Jasmine Boreal").expect("commander")),
        (P1, catalog::by_name("Jasmine Boreal").expect("commander")),
    ];
    let mut game = Game::new_commander_combat(41, &decks, &specs).expect("bounded profile");
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    game
}

/// The 47 new starter creatures are genuine vanilla printings: the catalog entry
/// must carry the sourced cost, type line, power/toughness and no rules text.
#[test]
fn every_sourced_vanilla_creature_matches_its_printing() {
    let cards = sourced_cards();
    let vanilla: Vec<&Value> = cards.iter().filter(|c| role(c) == "vanilla").collect();
    assert_eq!(vanilla.len(), 47, "the starter adds 47 vanilla creatures");
    for card in vanilla {
        let name = text(card, "name");
        let id =
            catalog::by_name(name).unwrap_or_else(|| panic!("{name} is not in the supported pool"));
        let def = id.def();
        assert_eq!(def.cost, Some(text(card, "mana_cost")), "{name} cost");
        assert_eq!(def.type_line, text(card, "type_line"), "{name} type line");
        assert_eq!(def.power, Some(number(card, "power")), "{name} power");
        assert_eq!(
            def.toughness,
            Some(number(card, "toughness")),
            "{name} toughness"
        );
        assert_eq!(def.oracle, "", "{name} must print no rules text");
        assert_eq!(
            card["keywords"].as_array().map(Vec::len),
            Some(0),
            "{name} must not need keyword semantics"
        );
        let type_line = text(card, "type_line");
        for forbidden in ["Legendary", "Artifact", "Enchantment", "Snow"] {
            assert!(
                !type_line.contains(forbidden),
                "{name} must not rely on unsupported {forbidden} semantics"
            );
        }
        assert_eq!(
            card["legalities_commander"], "legal",
            "{name} must be Commander-legal in the captured snapshot"
        );
        assert!(!def.is_land() && def.is_creature(), "{name} is a creature");
        assert!(
            def.spell.is_none(),
            "{name} is vanilla: the creature spell has no effects"
        );
    }
}

/// Casting and resolving each sourced vanilla creature is actual behavior, not a
/// count assertion: the spell reaches the stack and becomes a permanent with the
/// printed characteristics.
#[test]
fn every_sourced_vanilla_creature_casts_and_resolves() {
    let cards = sourced_cards();
    let mut lands = vec![CardId::Forest; 10];
    lands.extend(std::iter::repeat_n(CardId::Plains, 10));
    for card in cards.iter().filter(|c| role(c) == "vanilla") {
        let name = text(card, "name");
        let id = catalog::by_name(name).expect("supported vanilla");
        let mut game = fixture(7)
            .battlefield(P0, &lands)
            .hand(P0, &[id])
            .main(P0, Step::PrecombatMain)
            .build();
        let in_hand = hand_card(&game, P0, id);
        let events = submit_ok(
            &mut game,
            P0,
            Action::CastSpell {
                card: in_hand,
                targets: Vec::new(),
            },
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Cast { card, .. } if *card == id)),
            "{name} must be cast from the hand: {events:?}"
        );
        submit_ok(&mut game, P0, Action::Pass);
        submit_ok(&mut game, P1, Action::Pass);
        assert!(game.state.stack.is_empty(), "{name} resolves off the stack");
        let permanent = game
            .state
            .battlefield(P0)
            .into_iter()
            .find(|object| object.card == id)
            .unwrap_or_else(|| panic!("{name} did not become a permanent"));
        assert_eq!(permanent.power(), number(card, "power"), "{name} power");
        assert_eq!(
            permanent.toughness(),
            number(card, "toughness"),
            "{name} toughness"
        );
    }
}

#[test]
fn jasmine_boreal_is_a_typed_supported_commander() {
    let jasmine = catalog::by_name("Jasmine Boreal").expect("Jasmine Boreal is supported");
    let def = jasmine.def();
    assert_eq!(def.cost, Some("{3}{G}{W}"));
    assert_eq!(def.type_line, "Legendary Creature — Human");
    assert_eq!((def.power, def.toughness), (Some(4), Some(5)));
    assert_eq!(def.oracle, "");
    assert!(jasmine.is_supported_commander());
    // The existing commander registry entry is preserved.
    assert!(
        catalog::by_name("Isamaru, Hound of Konda")
            .expect("Isamaru")
            .is_supported_commander()
    );
    assert!(
        !catalog::by_name("Grizzly Bears")
            .expect("bear")
            .is_supported_commander()
    );

    let game = commander_game();
    assert_eq!(game.state.profile, Profile::CommanderCombat40);
    assert_eq!(game.state.profile.label(), "commander-combat-practice-40");
    assert_eq!(game.state.player(P0).life, 40);
    assert_eq!(game.state.player(P0).command_zone.len(), 1);
    assert_eq!(game.state.commanders.len(), 2);
    assert_eq!(game.state.commanders[0].card, jasmine);
    // The exact 99-card library is untouched by commander setup.
    assert_eq!(game.state.player(P0).library.len(), 92);
}

#[test]
fn jasmine_is_refused_in_every_ordinary_library() {
    let err = DeckList::parse_practice("bad", "1 Jasmine Boreal\n39 Forest\n")
        .expect_err("a commander-only legend must not enter an ordinary library");
    assert!(err.contains("commander-only"), "{err}");
    assert!(err.contains("legend rule"), "{err}");

    let mut cards = vec![(catalog::by_name("Jasmine Boreal").unwrap(), 1u8)];
    cards.push((CardId::Forest, 39));
    let deck = DeckList::new("bad", cards);
    let err = Game::new(1, &[deck.clone(), deck]).expect_err("engine must refuse it too");
    assert!(
        matches!(err, EngineError::UnsupportedCommander { card } if card.name() == "Jasmine Boreal"),
        "{err}"
    );
}

#[test]
fn jasmine_commander_damage_uses_the_existing_ledger() {
    let mut game = commander_game();
    let commander = game
        .state
        .commanders
        .iter()
        .find(|commander| commander.owner == P0)
        .expect("P0 commander registry")
        .clone();
    game.state.players[P0 as usize]
        .command_zone
        .retain(|id| *id != commander.object);
    {
        let object = game
            .state
            .objects
            .get_mut(&commander.object)
            .expect("commander object");
        object.zone = Zone::Battlefield;
        object.controller = P0;
        object.tapped = false;
        object.summoning_sick = false;
    }
    game.state.turn.number = 2;
    game.state.turn.active = P0;
    game.state.turn.step = Step::DeclareAttackers;
    game.state.turn.priority = None;
    game.state.turn.passed = 0;
    game.state.turn.attackers.clear();
    game.state.turn.blocked.clear();
    game.state.turn.blockers.clear();
    game.state.turn.combat_damage_done = false;
    game.state.turn.defender = None;
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![commander.object],
            defender: Some(P1),
        },
    );
    let pass_round = game.state.live_count();
    for _ in 0..2 {
        for _ in 0..pass_round {
            let player = game.decision_owner().expect("a seat has priority");
            submit_ok(&mut game, player, Action::Pass);
        }
    }
    assert_eq!(
        game.state.commander_damage_dealt(commander.id, P1),
        4,
        "Jasmine's identity must be credited with her combat damage"
    );
    assert_eq!(game.state.player(P1).life, 36);
}

#[test]
fn jasmine_commander_cast_tax_and_return_use_the_existing_paths() {
    let mut game = commander_game();
    // {3}{G}{W}: three generic, one green and one white.
    for _ in 0..3 {
        add_permanent(&mut game, P0, CardId::Forest);
    }
    for _ in 0..2 {
        add_permanent(&mut game, P0, CardId::Plains);
    }
    let commander = game.state.player(P0).command_zone[0];
    let events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: commander,
            targets: Vec::new(),
        },
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Cast { object, .. } if *object != commander
        )),
        "the commander is cast as a fresh object: {events:?}"
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 1);
    assert_eq!(
        game.state.object(game.state.commanders[0].object).zone,
        Zone::Battlefield
    );

    // CR 903.9a: a commander that dies gets one owner return choice.
    let battlefield_id = game.state.commanders[0].object;
    game.state
        .objects
        .get_mut(&battlefield_id)
        .expect("commander on battlefield")
        .damage = 5;
    submit_ok(&mut game, P0, Action::Pass);
    assert!(
        matches!(game.state.pending, Some(Pending::CommanderReturn { .. })),
        "the owner must get the return choice"
    );
    submit_ok(
        &mut game,
        P0,
        Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    );
    let returned = game.state.player(P0).command_zone[0];
    assert_eq!(game.state.object(returned).zone, Zone::Command);
    assert_eq!(game.state.object(returned).card.name(), "Jasmine Boreal");

    // CR 903.8: the second cast costs {5}{G}{W}. Six fresh lands cannot pay it.
    for _ in 0..4 {
        add_permanent(&mut game, P0, CardId::Forest);
    }
    for _ in 0..2 {
        add_permanent(&mut game, P0, CardId::Plains);
    }
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    let err = game
        .submit_action(
            P0,
            Action::CastSpell {
                card: returned,
                targets: Vec::new(),
            },
        )
        .expect_err("six lands cannot pay seven mana");
    assert!(matches!(err, EngineError::CannotPay { .. }), "{err}");

    add_permanent(&mut game, P0, CardId::Forest);
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: returned,
            targets: Vec::new(),
        },
    );
    assert_eq!(game.state.commanders[0].casts_from_command_zone, 2);
    assert!(game.state.player(P0).mana.is_empty());
}

#[test]
fn the_first_starter_library_is_exact_and_fully_supported() {
    let deck = starter_deck();
    assert_eq!(deck.size(), 99, "99 library cards, commander separate");
    let lands = deck.lands();
    assert!(
        (40..=45).contains(&lands),
        "basic land count must stay bounded, got {lands}"
    );
    let mut nonland_names = Vec::new();
    for (card, count) in &deck.cards {
        if card.def().is_land() {
            assert!(
                card.def().basic_mana.is_some(),
                "{} must be a basic land",
                card.name()
            );
            continue;
        }
        assert_eq!(*count, 1, "{} must be singleton", card.name());
        assert!(
            !card.is_supported_commander(),
            "{} is commander-only",
            card.name()
        );
        nonland_names.push(card.name());
    }
    nonland_names.sort_unstable();
    assert!(
        (54..=59).contains(&nonland_names.len()),
        "unique nonland cards must be 54..=59, got {}",
        nonland_names.len()
    );
    assert_eq!(
        nonland_names.len(),
        deck.cards
            .iter()
            .filter(|(card, _)| !card.def().is_land())
            .count(),
        "each nonland card appears once"
    );

    let sourced = sourced_cards();
    for card in sourced.iter().filter(|c| role(c) == "vanilla") {
        let name = text(card, "name");
        let id = catalog::by_name(name).expect("vanilla supported");
        assert_eq!(
            deck.count_of(id),
            1,
            "{name} must be in the exact starter library"
        );
    }
    assert_eq!(
        deck.count_of(catalog::by_name("Jasmine Boreal").expect("commander")),
        0,
        "the commander is supplied separately, never in the library"
    );
}

#[test]
fn the_starter_manifest_names_eight_targets_and_claims_only_one() {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest parses");
    assert_eq!(manifest["version"], 1);
    let starters = manifest["starters"].as_array().expect("starters");
    assert_eq!(starters.len(), 8);
    let ids: Vec<&str> = starters
        .iter()
        .map(|starter| starter["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        ids,
        vec![
            "isamaru-white-aggro",
            "jasmine-vanilla-stompy",
            "talrand-draw-go",
            "krenko-goblin-tokens",
            "ayula-bear-counters",
            "elas-sacrifice",
            "tatyova-landfall",
            "zada-spells",
        ]
    );
    for starter in starters {
        let status = starter["status"].as_str().expect("status");
        assert!(
            matches!(status, "planned" | "implemented" | "certified"),
            "{status}"
        );
        assert!(starter["missing_cards"].is_array());
        assert!(starter["missing_interactions"].is_array());
        if starter["id"] == "jasmine-vanilla-stompy" {
            // Slice 10 certified the list: the exact snapshot, not just the
            // catalog membership, is what the status now claims.
            assert_eq!(status, "certified");
            let certification = &starter["certification"];
            assert_eq!(certification["verdict"], "certified");
            assert_eq!(certification["library_cards"], 99);
            assert_eq!(certification["seats"], 4);
            assert!(certification["interaction_exclusions"].is_array());
            assert_eq!(starter["commander"], "Jasmine Boreal");
            assert_eq!(
                starter["deck_path"],
                "decks/starter-corpus/jasmine-vanilla-stompy.txt"
            );
            assert_eq!(starter["missing_cards"].as_array().unwrap().len(), 0);
            assert_eq!(starter["missing_interactions"].as_array().unwrap().len(), 0);
        } else {
            assert_eq!(status, "planned", "only Jasmine is certified now");
            let path = starter["deck_path"]
                .as_str()
                .expect("every planned target has an exact desired list");
            assert!(
                path.starts_with("decks/starter-corpus/planned/"),
                "{path} must not look like a supported default"
            );
            assert!(
                !starter["missing_cards"]
                    .as_array()
                    .expect("missing_cards")
                    .is_empty(),
                "planned targets must name unsupported cards"
            );
        }
    }
}
