//! Issue 17 slice 3 cross-corpus package: Swamp, Lightning Bolt, Forest Bear,
//! Striped Bears and Guttersnipe.

mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn guttersnipe_game(seed: u64) -> Game {
    let mut game = fixture_n(seed, 4)
        .hand(P0, &[CardId::LightningBolt])
        .battlefield(P0, &[CardId::Mountain, CardId::Mountain])
        .library(P0, &[CardId::Island, CardId::Unsummon])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card: card("Guttersnipe"),
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
    game
}

#[test]
fn sourced_cross_package_cards_match_their_printings() {
    let swamp = card("Swamp").def();
    assert_eq!(swamp.type_line, "Basic Land — Swamp");
    assert_eq!(swamp.basic_mana, Some(ManaColor::Black));
    let bolt = card("Lightning Bolt").def();
    assert_eq!(bolt.cost, Some("{R}"));
    assert_eq!(bolt.type_line, "Instant");
    assert_eq!(bolt.oracle, "Lightning Bolt deals 3 damage to any target.");
    let bear = card("Forest Bear").def();
    assert_eq!(bear.cost, Some("{1}{G}"));
    assert_eq!((bear.power, bear.toughness), (Some(2), Some(2)));
    assert_eq!(bear.oracle, "");
    let striped = card("Striped Bears").def();
    assert_eq!(striped.cost, Some("{3}{G}"));
    assert_eq!(striped.oracle, "When this creature enters, draw a card.");
    let snipe = card("Guttersnipe").def();
    assert_eq!(snipe.cost, Some("{2}{R}"));
    assert_eq!(snipe.type_line, "Creature — Goblin Shaman");
    assert_eq!(
        snipe.oracle,
        "Whenever you cast an instant or sorcery spell, this creature deals 2 damage to each opponent."
    );
}

#[test]
fn swamp_taps_for_black_and_lightning_bolt_deals_three() {
    let mut game = fixture(61)
        .hand(P0, &[CardId::LightningBolt])
        .battlefield(P0, &[CardId::Swamp])
        .main(P0, Step::PrecombatMain)
        .build();
    let swamp = battlefield_card(&game, P0, CardId::Swamp);
    submit_ok(&mut game, P0, Action::ActivateManaAbility { land: swamp });
    assert_eq!(game.state.player(P0).mana.black, 1);
    let land = game
        .state
        .player(P0)
        .hand
        .iter()
        .copied()
        .find(|id| game.state.object(*id).card == CardId::LightningBolt)
        .unwrap();
    // No red mana yet: the Bolt itself needs a Mountain; use a second fixture
    // to prove its effect rather than mixing mana colors here.
    let mut game = fixture(62)
        .hand(P0, &[CardId::LightningBolt])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Player(P1)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.player(P1).life, 17);
    let _ = land;
}

#[test]
fn forest_bear_is_vanilla_and_striped_bears_draws_on_entry() {
    let mut game = fixture(63)
        .hand(P0, &[CardId::ForestBear, CardId::StripedBears])
        .battlefield(P0, &[CardId::Forest; 6])
        .library(P0, &[CardId::Forest; 5])
        .main(P0, Step::PrecombatMain)
        .build();
    for (card_id, draws) in [(CardId::ForestBear, 0usize), (CardId::StripedBears, 1)] {
        let hand = hand_card(&game, P0, card_id);
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
        let hand_before = game.state.player(P0).hand.len();
        for _ in 0..3 {
            if game.state.stack.is_empty() && game.state.trigger_queue.is_empty() {
                break;
            }
            let player = game.decision_owner().expect("priority");
            submit_ok(&mut game, player, Action::Pass);
        }
        if draws == 1 {
            assert_eq!(game.state.player(P0).hand.len(), hand_before + 1);
        }
        assert_eq!(
            game.state
                .battlefield(P0)
                .iter()
                .filter(|o| o.card == card_id)
                .count(),
            1
        );
    }
    assert_eq!(
        game.state
            .battlefield(P0)
            .iter()
            .find(|o| o.card == CardId::ForestBear)
            .unwrap()
            .power(),
        2
    );
}

#[test]
fn guttersnipe_hits_each_opponent_but_not_its_controller() {
    let mut game = guttersnipe_game(64);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    let before: Vec<i32> = game.state.players.iter().map(|p| p.life).collect();
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Player(P1)],
        },
    );
    for _ in 0..8 {
        if game.state.stack.is_empty() && game.state.trigger_queue.is_empty() {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    // Bolt hits P1 for 3; Guttersnipe hits P1, P2 and P3 for 2 each.
    assert_eq!(game.state.player(P0).life, before[0]);
    assert_eq!(game.state.player(P1).life, before[1] - 5);
    assert_eq!(game.state.player(2).life, before[2] - 2);
    assert_eq!(game.state.player(3).life, before[3] - 2);
}

#[test]
fn guttersnipe_trigger_still_deals_damage_after_the_source_leaves() {
    let mut game = guttersnipe_game(65);
    let snipe = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == card("Guttersnipe"))
        .unwrap()
        .id;
    // Give P0 an Island and Unsummon while a Bolt trigger is on the stack.
    let island = game.state.players[P0 as usize].library.pop().unwrap();
    game.state.objects.get_mut(&island).unwrap().card = CardId::Island;
    game.state.objects.get_mut(&island).unwrap().zone = Zone::Battlefield;
    let unsummon = game.state.players[P0 as usize].library.pop().unwrap();
    game.state.objects.get_mut(&unsummon).unwrap().card = CardId::Unsummon;
    game.state.objects.get_mut(&unsummon).unwrap().zone = Zone::Hand;
    game.state.players[P0 as usize].hand.push(unsummon);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    let before: Vec<i32> = game.state.players.iter().map(|p| p.life).collect();
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Player(P1)],
        },
    );
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: unsummon,
            targets: vec![Target::Permanent(snipe)],
        },
    );
    for _ in 0..10 {
        if game.state.stack.is_empty() && game.state.trigger_queue.is_empty() {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert_eq!(game.state.player(2).life, before[2] - 2);
    assert_eq!(game.state.player(3).life, before[3] - 2);
    assert!(
        game.state.players[P0 as usize]
            .hand
            .iter()
            .any(|id| game.state.object(*id).card == card("Guttersnipe")),
        "the source was bounced"
    );
}
