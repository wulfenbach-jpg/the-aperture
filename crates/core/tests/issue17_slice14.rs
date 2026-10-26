//! Issue 17 slice 14: dynamic artifact mana and Werebear threshold.

mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn add_artifact(game: &mut Game, p: PlayerId, id: CardId) -> ObjectId {
    let oid = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        oid,
        aperture_core::state::Object {
            id: oid,
            card: id,
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
    oid
}

fn offered_colors(game: &Game, source: ObjectId) -> Vec<ManaColor> {
    game.legal_actions()
        .iter()
        .filter_map(|a| match a.action {
            Action::ActivateColorManaAbility { source: s, color } if s == source => Some(color),
            _ => None,
        })
        .collect()
}

#[test]
fn sourced_slice14_cards_match_printings() {
    let arcane = card("Arcane Signet").def();
    assert_eq!(arcane.cost, Some("{2}"));
    assert_eq!(arcane.type_line, "Artifact");
    assert_eq!(
        arcane.oracle,
        "{T}: Add one mana of any color in your commander's color identity."
    );
    let fellwar = card("Fellwar Stone").def();
    assert_eq!(
        fellwar.oracle,
        "{T}: Add one mana of any color that a land an opponent controls could produce."
    );
    let bear = card("Werebear").def();
    assert_eq!(bear.cost, Some("{1}{G}"));
    assert_eq!((bear.power, bear.toughness), (Some(1), Some(1)));
    assert_eq!(
        bear.oracle,
        "{T}: Add {G}.\nThreshold — This creature gets +3/+3 as long as there are seven or more cards in your graveyard."
    );
}

#[test]
fn arcane_signet_uses_commander_identity_and_refuses_forged_colors() {
    let deck = DeckList::parse_practice("x", "40 Forest\n").unwrap();
    let mut game = Game::new_commander_combat(
        180,
        &[deck.clone(), deck],
        &[
            (P0, card("Tatyova, Benthic Druid")),
            (P1, card("Tatyova, Benthic Druid")),
        ],
    )
    .unwrap();
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let signet = add_artifact(&mut game, P0, card("Arcane Signet"));
    let colors = offered_colors(&game, signet);
    assert_eq!(
        colors,
        vec![ManaColor::Blue, ManaColor::Green],
        "WUBRG-stable G/U order"
    );
    let before = aperture_core::digest::digest(&game.state);
    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateColorManaAbility {
            source: signet,
            color: ManaColor::Red,
        },
    );
    assert!(
        err.to_string().to_lowercase().contains("cannot produce"),
        "{err}"
    );
    assert_eq!(aperture_core::digest::digest(&game.state), before);
    submit_ok(
        &mut game,
        P0,
        Action::ActivateColorManaAbility {
            source: signet,
            color: ManaColor::Green,
        },
    );
    assert_eq!(game.state.player(P0).mana.green, 1);
    assert!(game.state.object(signet).tapped);
    assert!(game.state.stack.is_empty(), "mana ability is immediate");
    assert!(game.state.turn.priority == Some(P0));
}

#[test]
fn arcane_signet_without_a_commander_has_no_color_options() {
    let mut game = fixture(181).main(P0, Step::PrecombatMain).build();
    let signet = add_artifact(&mut game, P0, card("Arcane Signet"));
    assert!(offered_colors(&game, signet).is_empty());
    assert!(
        submit_err(
            &mut game,
            P0,
            Action::ActivateColorManaAbility {
                source: signet,
                color: ManaColor::White
            }
        )
        .to_string()
        .contains("cannot produce")
    );
}

#[test]
fn fellwar_stone_uses_opponent_lands_even_tapped_and_excludes_own() {
    let mut game = fixture(182)
        .battlefield(P0, &[CardId::Mountain])
        .battlefield(P1, &[CardId::Island, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let island = battlefield_card(&game, P1, CardId::Island);
    game.state.objects.get_mut(&island).unwrap().tapped = true;
    let stone = add_artifact(&mut game, P0, card("Fellwar Stone"));
    assert_eq!(
        offered_colors(&game, stone),
        vec![ManaColor::Blue, ManaColor::Green]
    );
    let before = aperture_core::digest::digest(&game.state);
    assert!(
        submit_err(
            &mut game,
            P0,
            Action::ActivateColorManaAbility {
                source: stone,
                color: ManaColor::Red
            }
        )
        .to_string()
        .contains("cannot produce")
    );
    assert_eq!(aperture_core::digest::digest(&game.state), before);
    game.state.objects.get_mut(&island).unwrap().zone = Zone::Graveyard;
    game.state.players[P1 as usize].graveyard.push(island);
    assert_eq!(offered_colors(&game, stone), vec![ManaColor::Green]);
}

#[test]
fn werebear_obeys_sickness_and_threshold_both_ways_with_layers() {
    let mut game = fixture(183)
        .hand(P0, &[card("Werebear")])
        .battlefield(P0, &[CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear_hand = hand_card(&game, P0, card("Werebear"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bear_hand,
            targets: Vec::new(),
        },
    );
    for _ in 0..4 {
        if game.state.stack.is_empty() && game.state.trigger_queue.is_empty() {
            break;
        }
        let player = game.decision_owner().unwrap();
        submit_ok(&mut game, player, Action::Pass);
    }
    let bear = battlefield_card(&game, P0, card("Werebear"));
    assert!(
        submit_err(&mut game, P0, Action::ActivateManaAbility { land: bear })
            .to_string()
            .contains("sickness")
            || submit_err(&mut game, P0, Action::ActivateManaAbility { land: bear })
                .to_string()
                .contains("summon")
    );
    game.state.objects.get_mut(&bear).unwrap().summoning_sick = false;
    submit_ok(&mut game, P0, Action::ActivateManaAbility { land: bear });
    assert_eq!(game.state.player(P0).mana.green, 1);

    for _ in 0..6 {
        let id = ObjectId(game.state.next_object);
        game.state.next_object += 1;
        game.state.objects.insert(
            id,
            aperture_core::state::Object {
                id,
                card: CardId::Forest,
                owner: P0,
                controller: P0,
                zone: Zone::Graveyard,
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
        game.state.players[P0 as usize].graveyard.push(id);
    }
    // Trigger a layer recompute through a pass cycle.
    let player = game.decision_owner().unwrap();
    submit_ok(&mut game, player, Action::Pass);
    assert_eq!(
        (
            game.state.object(bear).power(),
            game.state.object(bear).toughness()
        ),
        (1, 1)
    );
    let seventh = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        seventh,
        aperture_core::state::Object {
            id: seventh,
            card: CardId::Forest,
            owner: P0,
            controller: P0,
            zone: Zone::Graveyard,
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
    game.state.players[P0 as usize].graveyard.push(seventh);
    game.state
        .objects
        .get_mut(&bear)
        .unwrap()
        .counters
        .add(aperture_core::counters::CounterKind::PlusOnePlusOne, 1);
    let player = game.decision_owner().unwrap();
    submit_ok(&mut game, player, Action::Pass);
    assert_eq!(
        (
            game.state.object(bear).power(),
            game.state.object(bear).toughness()
        ),
        (5, 5)
    );
    game.state.players[P0 as usize]
        .graveyard
        .retain(|id| *id != seventh);
    game.state.objects.get_mut(&seventh).unwrap().zone = Zone::Removed;
    let player = game.decision_owner().unwrap();
    submit_ok(&mut game, player, Action::Pass);
    assert_eq!(
        (
            game.state.object(bear).power(),
            game.state.object(bear).toughness()
        ),
        (2, 2)
    );
}

#[test]
fn commanders_sphere_casts_chooses_color_and_sacrifices_while_tapped_for_draw() {
    let deck = DeckList::parse_practice("x", "40 Forest\n").unwrap();
    let mut game = Game::new_commander_combat(
        184,
        &[deck.clone(), deck],
        &[
            (P0, card("Tatyova, Benthic Druid")),
            (P1, card("Tatyova, Benthic Druid")),
        ],
    )
    .unwrap();
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    for _ in 0..3 {
        let id = ObjectId(game.state.next_object);
        game.state.next_object += 1;
        game.state.objects.insert(
            id,
            aperture_core::state::Object {
                id,
                card: CardId::Forest,
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
                counters: Default::default(),
            },
        );
    }
    let sphere_hand = {
        let id = ObjectId(game.state.next_object);
        game.state.next_object += 1;
        game.state.objects.insert(
            id,
            aperture_core::state::Object {
                id,
                card: card("Commander's Sphere"),
                owner: P0,
                controller: P0,
                zone: Zone::Hand,
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
        game.state.players[P0 as usize].hand.push(id);
        id
    };
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: sphere_hand,
            targets: Vec::new(),
        },
    );
    for _ in 0..4 {
        if game.state.stack.is_empty() && game.state.trigger_queue.is_empty() {
            break;
        }
        let player = game.decision_owner().unwrap();
        submit_ok(&mut game, player, Action::Pass);
    }
    let sphere = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == card("Commander's Sphere"))
        .unwrap()
        .id;
    assert_eq!(
        offered_colors(&game, sphere),
        vec![ManaColor::Blue, ManaColor::Green]
    );
    submit_ok(
        &mut game,
        P0,
        Action::ActivateColorManaAbility {
            source: sphere,
            color: ManaColor::Green,
        },
    );
    assert!(game.state.object(sphere).tapped);
    // Sacrifice has no {T} cost, so it works while tapped.
    let hand_before = game.state.player(P0).hand.len();
    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: sphere,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert_eq!(game.state.object(sphere).zone, Zone::Graveyard);
    assert!(!game.state.stack.is_empty());
    assert_eq!(
        game.state.player(P0).hand.len(),
        hand_before,
        "draw not yet resolved"
    );
    for _ in 0..4 {
        if game.state.stack.is_empty() && game.state.trigger_queue.is_empty() {
            break;
        }
        let player = game.decision_owner().unwrap();
        submit_ok(&mut game, player, Action::Pass);
    }
    assert_eq!(
        game.state.player(P0).hand.len(),
        hand_before + 1,
        "sacrifice draw resolved"
    );
    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: sphere,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert!(
        err.to_string().to_lowercase().contains("zone")
            || err.to_string().to_lowercase().contains("battlefield"),
        "{err}"
    );
}

#[test]
fn signets_cast_from_hand_and_identity_survives_commander_leaving_command_zone() {
    let deck = DeckList::parse_practice("x", "40 Mountain\n").unwrap();
    let mut game = Game::new_commander_combat(
        185,
        &[deck.clone(), deck],
        &[
            (P0, card("Krenko, Mob Boss")),
            (P1, card("Krenko, Mob Boss")),
        ],
    )
    .unwrap();
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    for _ in 0..3 {
        let id = ObjectId(game.state.next_object);
        game.state.next_object += 1;
        game.state.objects.insert(
            id,
            aperture_core::state::Object {
                id,
                card: CardId::Mountain,
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
                counters: Default::default(),
            },
        );
    }
    let hand_id = {
        let id = ObjectId(game.state.next_object);
        game.state.next_object += 1;
        game.state.objects.insert(
            id,
            aperture_core::state::Object {
                id,
                card: card("Arcane Signet"),
                owner: P0,
                controller: P0,
                zone: Zone::Hand,
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
        game.state.players[P0 as usize].hand.push(id);
        id
    };
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: hand_id,
            targets: Vec::new(),
        },
    );
    for _ in 0..4 {
        if game.state.stack.is_empty() {
            break;
        }
        let p = game.decision_owner().unwrap();
        submit_ok(&mut game, p, Action::Pass);
    }
    let signet = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == card("Arcane Signet"))
        .unwrap()
        .id;
    assert_eq!(offered_colors(&game, signet), vec![ManaColor::Red]);
    // Move the commander out of the command zone; registered identity remains.
    let commander = game.state.player(P0).command_zone[0];
    game.state.players[P0 as usize]
        .command_zone
        .retain(|id| *id != commander);
    game.state.objects.get_mut(&commander).unwrap().zone = Zone::Graveyard;
    assert_eq!(offered_colors(&game, signet), vec![ManaColor::Red]);
}

#[test]
fn fellwar_ignores_eliminated_opponent_and_own_lands() {
    let mut game = fixture(186)
        .battlefield(P0, &[CardId::Mountain])
        .battlefield(P1, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    game.state.players[P1 as usize].lost = true;
    let stone = add_artifact(&mut game, P0, card("Fellwar Stone"));
    assert!(
        offered_colors(&game, stone).is_empty(),
        "eliminated opponent's leftover land is ignored"
    );
    let no_land = fixture(187).main(P0, Step::PrecombatMain).build();
    let mut no_land = no_land;
    let stone2 = add_artifact(&mut no_land, P0, card("Fellwar Stone"));
    assert!(offered_colors(&no_land, stone2).is_empty());
}
