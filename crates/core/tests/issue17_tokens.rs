//! Issue 17 slice 2: Krenko goblin-token package. Covers token identity,
//! characteristics, Krenko's Goblin count, ETB triggers, token cessation and
//! state serialization.

mod common;

use aperture_core::ability::abilities;
use aperture_core::catalog;
use aperture_core::*;
use common::*;

const GOBDECK: &str = "40 Mountain\n";

fn red_game(seed: u64) -> (Game, Vec<Event>) {
    let deck = DeckList::parse_practice("goblins", GOBDECK).expect("deck");
    let (mut game, log) = Game::new_commander_combat_logged(
        seed,
        &[deck.clone(), deck],
        &[
            (P0, catalog::by_name("Krenko, Mob Boss").expect("Krenko")),
            (P1, catalog::by_name("Krenko, Mob Boss").expect("Krenko")),
        ],
    )
    .expect("command game");
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    (game, log)
}

fn token_objects(game: &Game, controller: PlayerId) -> Vec<&aperture_core::state::Object> {
    game.state
        .battlefield(controller)
        .into_iter()
        .filter(|object| object.card.is_token())
        .collect()
}

#[test]
fn sourced_token_package_matches_its_recorded_printings() {
    let expected = [
        (
            "Krenko, Mob Boss",
            "{2}{R}{R}",
            "Legendary Creature — Goblin Warrior",
            Some(3),
            Some(3),
            "{T}: Create X 1/1 red Goblin creature tokens, where X is the number of Goblins you control.",
        ),
        (
            "Dragon Fodder",
            "{1}{R}",
            "Sorcery",
            None,
            None,
            "Create two 1/1 red Goblin creature tokens.",
        ),
        (
            "Krenko's Command",
            "{1}{R}",
            "Sorcery",
            None,
            None,
            "Create two 1/1 red Goblin creature tokens.",
        ),
        (
            "Hordeling Outburst",
            "{1}{R}{R}",
            "Sorcery",
            None,
            None,
            "Create three 1/1 red Goblin creature tokens.",
        ),
        (
            "Goblin Instigator",
            "{1}{R}",
            "Creature — Goblin Rogue",
            Some(1),
            Some(1),
            "When this creature enters, create a 1/1 red Goblin creature token.",
        ),
        (
            "Beetleback Chief",
            "{2}{R}{R}",
            "Creature — Goblin Warrior",
            Some(2),
            Some(2),
            "When this creature enters, create two 1/1 red Goblin creature tokens.",
        ),
    ];
    for (name, cost, type_line, power, toughness, oracle) in expected {
        let def = catalog::by_name(name)
            .unwrap_or_else(|| panic!("{name} must be in the pool"))
            .def();
        assert_eq!(def.cost, Some(cost), "{name}");
        assert_eq!(def.type_line, type_line, "{name}");
        assert_eq!((def.power, def.toughness), (power, toughness), "{name}");
        assert_eq!(def.oracle, oracle, "{name}");
    }
    assert!(
        catalog::by_name("Krenko, Mob Boss")
            .expect("Krenko")
            .is_supported_commander()
    );
    // Krenko is commander-only, never an ordinary library card.
    let err = DeckList::parse_practice("bad", "1 Krenko, Mob Boss\n39 Mountain\n")
        .expect_err("commander-only");
    assert!(err.contains("commander-only"), "{err}");
}

#[test]
fn token_cards_are_not_importable() {
    assert!(catalog::by_name("Goblin").is_none());
    assert!(catalog::by_name("Goblin Token").is_none());
    assert!(
        !catalog::all()
            .iter()
            .any(|def| def.id == CardId::GoblinToken),
        "tokens must not be in the importable catalog"
    );
    let err = DeckList::parse_practice("bad", "1 Goblin\n39 Mountain\n").unwrap_err();
    assert!(err.contains("not in the supported pool"), "{err}");
    let def = CardId::GoblinToken.def();
    assert_eq!(def.type_line, "Token Creature — Goblin");
    assert_eq!((def.power, def.toughness), (Some(1), Some(1)));
    assert_eq!(def.colors, &[ManaColor::Red]);
}

#[test]
fn token_sorceries_create_the_printed_count_with_unique_objects() {
    for (name, count) in [
        ("Dragon Fodder", 2usize),
        ("Krenko's Command", 2),
        ("Hordeling Outburst", 3),
    ] {
        let id = catalog::by_name(name).expect("card");
        let mut game = fixture(11)
            .hand(P0, &[id])
            .battlefield(P0, &[CardId::Mountain; 4])
            .main(P0, Step::PrecombatMain)
            .build();
        let card = hand_card(&game, P0, id);
        submit_ok(
            &mut game,
            P0,
            Action::CastSpell {
                card,
                targets: Vec::new(),
            },
        );
        assert!(
            token_objects(&game, P0).is_empty(),
            "{name}: no priority token creation"
        );
        submit_ok(&mut game, P0, Action::Pass);
        submit_ok(&mut game, P1, Action::Pass);
        let tokens = token_objects(&game, P0);
        assert_eq!(tokens.len(), count, "{name}");
        let mut ids: Vec<_> = tokens.iter().map(|token| token.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "{name}: token object ids must be unique");
        for token in tokens {
            assert_eq!(token.controller, P0);
            assert_eq!(token.card, CardId::GoblinToken);
            assert_eq!(token.card.def().colors, &[ManaColor::Red]);
            assert_eq!((token.power(), token.toughness()), (1, 1));
            assert!(token.card.def().is_goblin());
        }
    }
}

#[test]
fn goblin_instigator_and_beetleback_chief_trigger_on_entry_in_apnap_order() {
    // Two simultaneous ETB triggers on one seat still resolve one at a time;
    // each creates its printed token count after the permanent enters.
    let mut game = fixture(12)
        .hand(P0, &[CardId::GoblinInstigator, CardId::BeetlebackChief])
        .battlefield(P0, &[CardId::Mountain; 6])
        .main(P0, Step::PrecombatMain)
        .build();
    for (card, expected) in [
        (CardId::GoblinInstigator, 1usize),
        (CardId::BeetlebackChief, 2usize),
    ] {
        let hand = hand_card(&game, P0, card);
        submit_ok(
            &mut game,
            P0,
            Action::CastSpell {
                card: hand,
                targets: Vec::new(),
            },
        );
        submit_ok(&mut game, P0, Action::Pass);
        submit_ok(&mut game, P1, Action::Pass);
        // The creature is on the battlefield; its ETB trigger is on the stack.
        assert_eq!(
            game.state
                .battlefield(P0)
                .iter()
                .filter(|o| o.card == card)
                .count(),
            1
        );
        let before = token_objects(&game, P0).len();
        submit_ok(&mut game, P0, Action::Pass);
        submit_ok(&mut game, P1, Action::Pass);
        assert_eq!(token_objects(&game, P0).len(), before + expected, "{card}");
    }
}

#[test]
fn krenko_counts_goblins_including_tokens_and_needs_untapped_untaunted_source() {
    let (mut game, _) = red_game(13);
    let krenko = game
        .state
        .commanders
        .iter()
        .find(|commander| commander.owner == P0)
        .expect("P0 commander")
        .object;
    game.state.players[P0 as usize]
        .command_zone
        .retain(|id| *id != krenko);
    {
        let object = game.state.objects.get_mut(&krenko).unwrap();
        object.zone = Zone::Battlefield;
        object.controller = P0;
        object.summoning_sick = true;
    }
    // A second Goblin permanent makes X = 2 (Krenko + Instigator).
    let helper = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        helper,
        aperture_core::state::Object {
            id: helper,
            card: CardId::GoblinInstigator,
            owner: P0,
            controller: P0,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
        },
    );
    let err = game
        .submit_action(
            P0,
            Action::ActivateAbility {
                source: krenko,
                ability: 0,
                mode: 0,
                targets: Vec::new(),
            },
        )
        .expect_err("summoning sickness forbids the tap activation");
    assert!(matches!(err, EngineError::BadTiming { .. }), "{err}");

    game.state.objects.get_mut(&krenko).unwrap().summoning_sick = false;
    let events = submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: krenko,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::AbilityActivated { source, .. } if *source == krenko
    )));
    assert!(game.state.object(krenko).tapped, "the tap cost is paid");
    assert!(
        token_objects(&game, P0).is_empty(),
        "no tokens before resolution"
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    let tokens = token_objects(&game, P0);
    assert_eq!(tokens.len(), 2, "X counts Krenko and the other Goblin");
    assert_eq!(
        game.state
            .battlefield(P0)
            .iter()
            .filter(|object| object.card.def().is_goblin())
            .count(),
        4
    );

    // The new tokens are Goblin permanents; a second activation would count 4,
    // but Krenko is tapped and cannot be activated again.
    let err = game
        .submit_action(
            P0,
            Action::ActivateAbility {
                source: krenko,
                ability: 0,
                mode: 0,
                targets: Vec::new(),
            },
        )
        .expect_err("tapped Krenko cannot activate");
    assert!(matches!(err, EngineError::Illegal { .. }), "{err}");
    assert_eq!(abilities(CardId::KrenkoMobBoss).len(), 1);
}

#[test]
fn a_token_ceases_to_exist_instead_of_going_to_the_graveyard() {
    let mut game = fixture(14)
        .hand(P0, &[CardId::DragonFodder])
        .battlefield(P0, &[CardId::Mountain; 3])
        .main(P0, Step::PrecombatMain)
        .build();
    let card = hand_card(&game, P0, CardId::DragonFodder);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    for _ in 0..2 {
        submit_ok(&mut game, P0, Action::Pass);
        submit_ok(&mut game, P1, Action::Pass);
    }
    let token = token_objects(&game, P0)[0].id;
    assert_eq!(
        token_objects(&game, P0)[0].name(),
        "Goblin Token",
        "CR 111.4: an unnamed token is its subtype(s) plus Token"
    );
    game.state.objects.get_mut(&token).unwrap().damage = 1;
    for _ in 0..4 {
        if game
            .state
            .try_object(token)
            .is_none_or(|object| object.zone == Zone::Removed)
        {
            break;
        }
        let player = game.decision_owner().expect("a seat has priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert_eq!(
        game.state.try_object(token).map(|object| object.zone),
        Some(Zone::Removed),
        "a token ceases to exist (CR 111.7)"
    );
    assert!(
        game.state
            .players
            .iter()
            .all(|player| !player.graveyard.contains(&token)),
        "a token never goes to a graveyard"
    );
}

fn add_goblin_permanent(game: &mut Game, controller: PlayerId, card: CardId) -> ObjectId {
    let id = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card,
            owner: controller,
            controller,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
        },
    );
    id
}

#[test]
fn krenko_never_silently_truncates_a_goblin_count_above_255() {
    let (mut game, _) = red_game(21);
    let krenko = game
        .state
        .commanders
        .iter()
        .find(|commander| commander.owner == P0)
        .expect("P0 commander")
        .object;
    game.state.players[P0 as usize]
        .command_zone
        .retain(|id| *id != krenko);
    {
        let object = game.state.objects.get_mut(&krenko).unwrap();
        object.zone = Zone::Battlefield;
        object.controller = P0;
        object.summoning_sick = false;
    }
    // 300 other Goblins: X must be 301 (all controlled Goblins, Krenko included).
    for _ in 0..300 {
        add_goblin_permanent(&mut game, P0, CardId::GoblinInstigator);
    }
    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: krenko,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(
        token_objects(&game, P0).len(),
        301,
        "the printed X is not clamped"
    );
    assert_eq!(
        game.state
            .battlefield(P0)
            .iter()
            .filter(|object| object.card.def().is_goblin())
            .count(),
        602
    );
}

#[test]
fn token_death_is_a_real_graveyard_change_then_cessation() {
    let mut game = fixture(22)
        .hand(P0, &[CardId::DragonFodder])
        .battlefield(P0, &[CardId::Mountain; 3])
        .main(P0, Step::PrecombatMain)
        .build();
    let card = hand_card(&game, P0, CardId::DragonFodder);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    for _ in 0..2 {
        submit_ok(&mut game, P0, Action::Pass);
        submit_ok(&mut game, P1, Action::Pass);
    }
    let token = token_objects(&game, P0)[0].id;
    game.state.objects.get_mut(&token).unwrap().damage = 1;
    let mut events = Vec::new();
    for _ in 0..6 {
        if game
            .state
            .try_object(token)
            .is_some_and(|o| o.zone == Zone::Removed)
        {
            break;
        }
        let player = game.decision_owner().expect("priority");
        events.extend(submit_ok(&mut game, player, Action::Pass));
    }
    let destroyed = events.iter().position(|event| {
        matches!(event, Event::Destroyed { object, card } if *object == token && *card == CardId::GoblinToken)
    });
    let ceased = events.iter().position(|event| {
        matches!(event, Event::StateBasedAction { description } if description.contains("ceases to exist"))
    });
    assert!(
        destroyed.is_some(),
        "the token really dies (CR 700.4): {events:?}"
    );
    assert!(
        ceased.is_some(),
        "the token ceases as an SBA (CR 111.7): {events:?}"
    );
    assert!(
        destroyed < ceased,
        "death is recorded before cessation: {events:?}"
    );
    assert_eq!(game.state.try_object(token).unwrap().zone, Zone::Removed);
    assert!(
        game.state
            .players
            .iter()
            .all(|player| !player.graveyard.contains(&token)),
        "no graveyard residue after the SBA checkpoint"
    );
}

#[test]
fn token_bounce_and_exile_record_the_destination_before_cessation() {
    let mut game = fixture(23)
        .hand(
            P0,
            &[
                CardId::DragonFodder,
                CardId::Unsummon,
                CardId::SwordsToPlowshares,
            ],
        )
        .battlefield(
            P0,
            &[
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Island,
                CardId::Plains,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let fodder = hand_card(&game, P0, CardId::DragonFodder);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: fodder,
            targets: Vec::new(),
        },
    );
    for _ in 0..2 {
        submit_ok(&mut game, P0, Action::Pass);
        submit_ok(&mut game, P1, Action::Pass);
    }
    let token = token_objects(&game, P0)[0].id;
    let unsummon = hand_card(&game, P0, CardId::Unsummon);
    let mut events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: unsummon,
            targets: vec![Target::Permanent(token)],
        },
    );
    for _ in 0..2 {
        events.extend(submit_ok(&mut game, P0, Action::Pass));
        events.extend(submit_ok(&mut game, P1, Action::Pass));
    }
    let returned = events.iter().position(|event| {
        matches!(event, Event::StateBasedAction { description } if description.contains("returned to its owner's hand"))
    });
    let ceased = events.iter().position(|event| {
        matches!(event, Event::StateBasedAction { description } if description.contains("ceases to exist"))
    });
    assert!(
        returned.is_some(),
        "bounce records the hand zone change: {events:?}"
    );
    assert!(ceased.is_some(), "the token then ceases: {events:?}");
    assert!(
        returned < ceased,
        "destination precedes cessation: {events:?}"
    );
    assert_eq!(game.state.try_object(token).unwrap().zone, Zone::Removed);
    assert!(
        game.state
            .players
            .iter()
            .all(|player| !player.hand.contains(&token) && !player.exile.contains(&token)),
        "no hand/exile residue"
    );

    // A second token is exiled by Swords; life gain uses last-known power and
    // the exile record is removed by the next SBA check.
    let token2 = token_objects(&game, P0).first().expect("second token").id;
    let life = game.state.player(P0).life;
    let swords = hand_card(&game, P0, CardId::SwordsToPlowshares);
    let mut events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: swords,
            targets: vec![Target::Permanent(token2)],
        },
    );
    for _ in 0..2 {
        events.extend(submit_ok(&mut game, P0, Action::Pass));
        events.extend(submit_ok(&mut game, P1, Action::Pass));
    }
    let exiled = events.iter().position(|event| {
        matches!(event, Event::StateBasedAction { description } if description.contains("is exiled"))
    });
    let ceased = events.iter().position(|event| {
        matches!(event, Event::StateBasedAction { description } if description.contains("ceases to exist"))
    });
    assert!(
        exiled.is_some() && ceased.is_some() && exiled < ceased,
        "exile then cease: {events:?}"
    );
    assert_eq!(
        game.state.player(P0).life,
        life + 1,
        "Swords gains last-known power"
    );
    assert_eq!(game.state.try_object(token2).unwrap().zone, Zone::Removed);
    assert!(
        game.state
            .players
            .iter()
            .all(|player| !player.exile.contains(&token2))
    );
}

#[test]
fn token_state_round_trips_through_serialization_with_an_equal_digest() {
    let mut game = fixture(15)
        .hand(P0, &[CardId::DragonFodder])
        .battlefield(P0, &[CardId::Mountain; 3])
        .main(P0, Step::PrecombatMain)
        .build();
    let card = hand_card(&game, P0, CardId::DragonFodder);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    for _ in 0..2 {
        submit_ok(&mut game, P0, Action::Pass);
        submit_ok(&mut game, P1, Action::Pass);
    }
    let digest = aperture_core::digest::digest(&game.state);
    let json = serde_json::to_string(&game.state).expect("state json");
    let restored: GameState = serde_json::from_str(&json).expect("state parses");
    assert_eq!(aperture_core::digest::digest(&restored), digest);
    assert_eq!(
        restored
            .objects
            .values()
            .filter(|object| object.card.is_token())
            .count(),
        2
    );
}
