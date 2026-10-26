//! Issue 17 slice 5: sourced mill / counter / ritual package.
//!
//! Six cards added to the supported catalog, each read from live Scryfall in
//! the run that added it (`fixtures/starter-corpus/slice5-sourced.json`):
//! Stitcher's Supplier (mill on enters **and** on dies), Ruin Crab (landfall
//! mill for each opponent), Negate (counter target noncreature spell),
//! Preordain (scry 2 then draw), Pyretic Ritual and Seething Song (add red
//! mana). Every assertion drives a real engine action; nothing here reads a
//! field back to itself.

mod common;

use aperture_core::ability::{TriggerEvent, abilities, triggers_on};
use aperture_core::catalog;
use aperture_core::*;
use common::*;

/// Seats 2 and 3 are not named in `aperture_core::ids` (that crate only names
/// P0/P1); the four-seat tests below spell them out.
const P2: PlayerId = 2;
const P3: PlayerId = 3;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn graveyard_cards(game: &Game, p: PlayerId) -> Vec<CardId> {
    game.state
        .player(p)
        .graveyard
        .iter()
        .map(|id| game.state.object(*id).card)
        .collect()
}

/// Drive the game with the deterministic scripted policy until `pred` holds.
fn advance_until(game: &mut Game, pred: impl Fn(&Game) -> bool) -> Vec<Event> {
    let mut log = Vec::new();
    let reached = run_until(game, &mut log, pred, 80);
    assert!(
        reached,
        "the scripted driver never reached the expected state"
    );
    log
}

#[test]
fn sourced_slice5_cards_match_their_live_printings() {
    let supplier = card("Stitcher's Supplier").def();
    assert_eq!(supplier.cost, Some("{B}"));
    assert_eq!(supplier.type_line, "Creature — Zombie");
    assert_eq!((supplier.power, supplier.toughness), (Some(1), Some(1)));
    assert_eq!(
        supplier.oracle,
        "When this creature enters or dies, mill three cards. (Put the top three cards of your library into your graveyard.)"
    );
    assert!(supplier.spell.is_none(), "a creature has no spell effects");

    let crab = card("Ruin Crab").def();
    assert_eq!(crab.cost, Some("{U}"));
    assert_eq!(crab.type_line, "Creature — Crab");
    assert_eq!((crab.power, crab.toughness), (Some(0), Some(3)));
    assert_eq!(
        crab.oracle,
        "Landfall — Whenever a land you control enters, each opponent mills three cards. (To mill a card, a player puts the top card of their library into their graveyard.)"
    );

    let negate = card("Negate").def();
    assert_eq!(negate.cost, Some("{1}{U}"));
    assert_eq!(negate.oracle, "Counter target noncreature spell.");
    let spec = negate.spell_spec().expect("Negate is a spell");
    assert_eq!(spec.target, TargetSpec::NoncreatureSpell);
    assert!(spec.instant);
    assert_eq!(spec.effects, &[Effect::CounterTargetSpell]);

    let preordain = card("Preordain").def();
    assert_eq!(preordain.cost, Some("{U}"));
    assert!(preordain.is_sorcery(), "Preordain is a sorcery");
    let spec = preordain.spell_spec().expect("Preordain is a spell");
    assert_eq!(spec.target, TargetSpec::None);
    assert_eq!(spec.effects, &[Effect::Scry(2), Effect::Draw(1)]);

    let pyretic = card("Pyretic Ritual").def();
    assert_eq!(pyretic.cost, Some("{1}{R}"));
    assert_eq!(pyretic.oracle, "Add {R}{R}{R}.");
    assert_eq!(
        pyretic.spell_spec().expect("ritual is a spell").effects,
        &[Effect::AddMana {
            color: ManaColor::Red,
            amount: 3
        }]
    );

    let seething = card("Seething Song").def();
    assert_eq!(seething.cost, Some("{2}{R}"));
    assert_eq!(seething.oracle, "Add {R}{R}{R}{R}{R}.");
    assert_eq!(
        seething.spell_spec().expect("ritual is a spell").effects,
        &[Effect::AddMana {
            color: ManaColor::Red,
            amount: 5
        }]
    );
}

#[test]
fn stitchers_supplier_mills_three_when_it_enters() {
    let supplier = card("Stitcher's Supplier");
    let mut game = fixture_n(101, 4)
        .hand(P0, &[supplier])
        .battlefield(P0, &[CardId::Swamp])
        .library(
            P0,
            &[
                CardId::Island,
                CardId::Forest,
                CardId::Mountain,
                CardId::Plains,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&game, P0, supplier);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: id,
            targets: vec![],
        },
    );
    let log = advance_until(&mut game, |g| g.state.player(P0).graveyard.len() == 3);

    // CR 701.13a: the top three cards, in library order, are now in the
    // graveyard and only the fourth card is left in the library.
    assert_eq!(
        graveyard_cards(&game, P0),
        vec![CardId::Island, CardId::Forest, CardId::Mountain]
    );
    let library: Vec<CardId> = game
        .state
        .player(P0)
        .library
        .iter()
        .map(|id| game.state.object(*id).card)
        .collect();
    assert_eq!(library, vec![CardId::Plains]);
    let milled = log
        .iter()
        .filter(|event| matches!(event, Event::Milled { .. }))
        .count();
    assert_eq!(milled, 3, "one Milled event per card");
    assert_eq!(game.state.player(P0).hand.len(), 0, "no draw happened");
}

#[test]
fn stitchers_supplier_mills_three_again_when_it_dies() {
    let supplier = card("Stitcher's Supplier");
    let mut game = fixture_n(102, 4)
        .hand(P0, &[CardId::LightningBolt])
        .battlefield(P0, &[CardId::Mountain, supplier])
        .library(
            P0,
            &[
                CardId::Island,
                CardId::Forest,
                CardId::Mountain,
                CardId::Plains,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let permanent = battlefield_card(&game, P0, supplier);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Permanent(permanent)],
        },
    );
    let log = advance_until(&mut game, |g| {
        g.state.player(P0).graveyard.len() >= 4 && g.state.player(P0).hand.is_empty()
    });

    // The 1/1 dies to three damage, then the dies trigger mills three more.
    assert!(
        !game.state.battlefield(P0).iter().any(|o| o.id == permanent),
        "the 1/1 is no longer on the battlefield"
    );
    let graveyard = graveyard_cards(&game, P0);
    assert_eq!(
        graveyard,
        vec![
            CardId::LightningBolt,
            supplier,
            CardId::Island,
            CardId::Forest,
            CardId::Mountain
        ],
        "Bolt, then the dead Supplier, then the three milled cards"
    );
    assert!(
        log.iter()
            .any(|event| matches!(event, Event::Destroyed { .. })),
        "the dies trigger comes from a real battlefield departure"
    );
}

#[test]
fn ruin_crab_mills_each_opponent_on_landfall_but_not_its_controller() {
    let crab = card("Ruin Crab");
    let mut game = fixture_n(103, 4)
        .hand(P0, &[CardId::Forest])
        .battlefield(P0, &[crab])
        .library(
            P0,
            &[
                CardId::Island,
                CardId::Forest,
                CardId::Mountain,
                CardId::Plains,
            ],
        )
        .library(P1, &[CardId::Forest; 4])
        .library(P2, &[CardId::Forest; 4])
        .library(P3, &[CardId::Forest; 4])
        .main(P0, Step::PrecombatMain)
        .build();
    let land = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: land });
    advance_until(&mut game, |g| {
        [P1, P2, P3]
            .iter()
            .all(|p| g.state.player(*p).graveyard.len() == 3)
    });

    for opponent in [P1, P2, P3] {
        assert_eq!(
            game.state.player(opponent).library.len(),
            1,
            "P{opponent} lost exactly three cards"
        );
    }
    // "each opponent" never includes the controller (CR 102.1, 109.5).
    assert_eq!(game.state.player(P0).graveyard.len(), 0);
    assert_eq!(
        game.state.player(P0).library.len(),
        4,
        "the controller's library is untouched"
    );
}

#[test]
fn negate_counters_a_noncreature_spell() {
    let negate = card("Negate");
    let mut game = fixture(104)
        .hand(P0, &[negate])
        .mana(
            P0,
            ManaPool {
                blue: 2,
                ..ManaPool::default()
            },
        )
        .hand(P1, &[CardId::LightningBolt])
        .battlefield(P1, &[CardId::Mountain])
        .main(P1, Step::PrecombatMain)
        .build();
    let bolt = hand_card(&game, P1, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Player(P0)],
        },
    );
    // CR 117.3d: the caster receives priority again after casting, so P1 must
    // pass before P0 may respond.
    submit_ok(&mut game, P1, Action::Pass);
    let on_stack = game
        .state
        .stack
        .last()
        .expect("Bolt is on the stack")
        .object;
    let offered = game.legal_targets(P0, TargetSpec::NoncreatureSpell, ObjectId(u32::MAX));
    assert!(
        offered.contains(&Target::Spell(on_stack)),
        "Negate may target the Bolt on the stack"
    );
    let id = hand_card(&game, P0, negate);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: id,
            targets: vec![Target::Spell(on_stack)],
        },
    );
    let log = advance_until(&mut game, |g| {
        g.state.stack.is_empty()
            && g.state.player(P1).graveyard.len() == 1
            && g.state.player(P0).graveyard.len() == 1
    });

    // CR 701.6a: the countered Bolt never resolves, so no damage is dealt.
    assert_eq!(game.state.player(P0).life, 20);
    assert!(
        log.iter()
            .any(|event| matches!(event, Event::Countered { .. })),
        "the counter is a real stack event"
    );
    assert!(graveyard_cards(&game, P1).contains(&CardId::LightningBolt));
    assert!(graveyard_cards(&game, P0).contains(&negate));
}

#[test]
fn negate_never_offers_a_creature_spell_and_refuses_a_forged_one() {
    let negate = card("Negate");
    let mut game = fixture(105)
        .hand(P0, &[negate])
        .mana(
            P0,
            ManaPool {
                blue: 2,
                ..ManaPool::default()
            },
        )
        .hand(P1, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::Forest, CardId::Forest])
        .main(P1, Step::PrecombatMain)
        .build();
    let bears = hand_card(&game, P1, CardId::GrizzlyBears);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: bears,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P1, Action::Pass);
    let on_stack = game
        .state
        .stack
        .last()
        .expect("Bears is on the stack")
        .object;
    let offered = game.legal_targets(P0, TargetSpec::NoncreatureSpell, ObjectId(u32::MAX));
    assert!(
        !offered.contains(&Target::Spell(on_stack)),
        "a creature spell is not a noncreature spell (CR 205.2a)"
    );
    let id = hand_card(&game, P0, negate);
    let err = submit_err(
        &mut game,
        P0,
        Action::CastSpell {
            card: id,
            targets: vec![Target::Spell(on_stack)],
        },
    );
    assert!(
        matches!(err, EngineError::BadTarget { .. }),
        "a forged creature-spell target is refused, got {err:?}"
    );
}

#[test]
fn preordain_scries_two_then_draws_one() {
    let preordain = card("Preordain");
    let mut game = fixture(106)
        .hand(P0, &[preordain])
        .mana(
            P0,
            ManaPool {
                blue: 1,
                ..ManaPool::default()
            },
        )
        .library(P0, &[CardId::Island, CardId::Forest, CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&game, P0, preordain);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: id,
            targets: vec![],
        },
    );
    advance_until(&mut game, |g| {
        g.state.player(P0).hand.len() == 1 && g.state.player(P0).library.len() == 2
    });

    // The scripted scry policy keeps both cards on top (CR 701.18a), so the
    // drawn card is the first library card and the next two are unchanged.
    let library: Vec<CardId> = game
        .state
        .player(P0)
        .library
        .iter()
        .map(|id| game.state.object(*id).card)
        .collect();
    assert_eq!(library, vec![CardId::Forest, CardId::Mountain]);
    let hand: Vec<CardId> = game
        .state
        .player(P0)
        .hand
        .iter()
        .map(|id| game.state.object(*id).card)
        .collect();
    assert_eq!(hand, vec![CardId::Island]);
}

#[test]
fn pyretic_ritual_and_seething_song_add_red_mana_to_the_pool() {
    for (name, cost_mana, expected) in [("Pyretic Ritual", 2u8, 3u8), ("Seething Song", 3, 5)] {
        let ritual = card(name);
        let mut game = fixture(107)
            .hand(P0, &[ritual])
            .mana(
                P0,
                ManaPool {
                    red: cost_mana,
                    ..ManaPool::default()
                },
            )
            .main(P0, Step::PrecombatMain)
            .build();
        let id = hand_card(&game, P0, ritual);
        submit_ok(
            &mut game,
            P0,
            Action::CastSpell {
                card: id,
                targets: vec![],
            },
        );
        let log = advance_until(&mut game, |g| {
            g.state.stack.is_empty() && g.state.player(P0).mana.red > 0
        });
        assert_eq!(
            game.state.player(P0).mana.red,
            expected,
            "{name}: the cost is paid from the pool and the ritual adds {expected} red"
        );
        let added = log
            .iter()
            .find_map(|event| match event {
                Event::ManaAdded {
                    amount, pool_total, ..
                } => Some((*amount, *pool_total)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name} must emit a ManaAdded event"));
        assert_eq!(added, (expected, expected));
    }
}

#[test]
fn the_mill_counter_and_ritual_package_is_reachable_from_real_decks() {
    // The three mill/counter/ritual names parse out of a real deck list, and a
    // name the engine cannot execute (target-player mill) is still refused.
    let list = "1 Stitcher's Supplier\n1 Ruin Crab\n1 Negate\n1 Preordain\n\
                1 Pyretic Ritual\n1 Seething Song\n1 Island\n1 Swamp\n";
    let deck = DeckList::parse_practice("Slice 5 package", list).expect("the package parses");
    assert_eq!(deck.cards.len(), 8);
    for name in [
        "Stitcher's Supplier",
        "Ruin Crab",
        "Negate",
        "Preordain",
        "Pyretic Ritual",
        "Seething Song",
    ] {
        assert!(catalog::by_name(name).is_some(), "{name} is supported");
    }
    let err = DeckList::parse_practice("Mill package", "1 Hedron Crab\n")
        .expect_err("target-player mill is not implemented yet");
    assert!(
        err.to_string().contains("Hedron Crab"),
        "the refusal names the unsupported card, got {err}"
    );
}

#[test]
fn stitchers_supplier_registers_both_printed_trigger_events() {
    let supplier = card("Stitcher's Supplier");
    let enters: Vec<_> = triggers_on(supplier, TriggerEvent::EntersBattlefield).collect();
    let dies: Vec<_> = triggers_on(supplier, TriggerEvent::Dies).collect();
    assert_eq!(enters.len(), 1, "one printed 'enters' trigger");
    assert_eq!(dies.len(), 1, "one printed 'dies' trigger");
    assert_eq!(enters[0].1.effects, &[Effect::MillSelf(3)]);
    assert_eq!(dies[0].1.effects, &[Effect::MillSelf(3)]);
    assert_eq!(enters[0].1.text, dies[0].1.text);
    assert_eq!(abilities(supplier).len(), 2);

    let crab = card("Ruin Crab");
    let landfall: Vec<_> = triggers_on(crab, TriggerEvent::LandEnters).collect();
    assert_eq!(landfall.len(), 1);
    assert_eq!(landfall[0].1.effects, &[Effect::MillEachOpponent(3)]);
    assert!(!landfall[0].1.optional);
}
