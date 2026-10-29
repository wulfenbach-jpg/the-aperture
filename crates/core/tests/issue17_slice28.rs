//! Slice 28: Circle of the Moon Druid — the pool's first *printed* static
//! ability conditioned on whose turn it is.
//!
//! The printed ability is "Bear Form — During your turn, this creature is a Bear
//! with base power and toughness 4/2. (It loses all other creature types.)"
//! (AFR 177, retrieved 2026-10-27, recorded in
//! `fixtures/starter-corpus/slice28-sourced.json`).
//!
//! One printed ability spans two layers: the subtype line is layer 4
//! (CR 613.1d, replacing the printed "Human Elf Druid") and base power/toughness
//! is layer 7b (CR 613.4b). Both halves are conditioned on its controller being
//! the active player (CR 611.3a), so they are answered when the characteristic
//! is read rather than stored as an effect that would have to expire. These
//! tests exercise that through real resolutions and the engine's own target
//! offers, not through an internal accessor.
mod common;

use aperture_core::ability::{self, StaticAbility};
use aperture_core::catalog;
use aperture_core::layer::{Layer, SubtypeLine};
use aperture_core::*;
use common::*;

fn druid() -> CardId {
    catalog::by_name("Circle of the Moon Druid")
        .expect("Circle of the Moon Druid must be supported")
}

fn drain(g: &mut Game) {
    for _ in 0..128 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return;
        }
        let Some(player) = g.decision_owner() else {
            return;
        };
        submit_ok(g, player, Action::Pass);
    }
    panic!("the stack did not settle");
}

fn green(generic: u32, pips: u32) -> ManaPool {
    ManaPool {
        green: pips,
        colorless: generic,
        ..ManaPool::default()
    }
}

/// Cast the Druid through the action the engine actually offers and return the
/// permanent it became.
fn cast_druid(g: &mut Game, player: PlayerId) -> ObjectId {
    let id = hand_card(g, player, druid());
    let action = g
        .legal_actions()
        .into_iter()
        .find(|offered| matches!(&offered.action, Action::CastSpell { card, .. } if *card == id))
        .expect("the Druid must be castable with the offered mana")
        .action;
    submit_ok(g, player, action);
    drain(g);
    battlefield_card(g, player, druid())
}

/// Pass the whole table through the current turn and return the new active seat.
fn advance_turn(g: &mut Game) -> PlayerId {
    let start = g.state.turn.number;
    for _ in 0..800 {
        if g.state.turn.number != start {
            return g.state.turn.active;
        }
        assert!(
            !g.state.is_over(),
            "the game ended while advancing the turn"
        );
        let owner = g
            .decision_owner()
            .expect("a seat decides while the turn advances");
        let action = auto_answer(g);
        let _ = g.submit_action(owner, action);
    }
    panic!("the turn did not advance");
}

fn stats(g: &Game, id: ObjectId) -> (i32, i32) {
    let object = g.state.object(id);
    (object.power(), object.toughness())
}

fn bear_targets(g: &Game, player: PlayerId, spec: TargetSpec) -> Vec<Target> {
    g.legal_targets(player, spec, ObjectId(u32::MAX))
}

fn druid_game(seed: u64) -> Game {
    fixture(seed)
        .hand(P0, &[druid(), CardId::GiantGrowth, CardId::LittleBear])
        .battlefield(P0, &[CardId::EliteVanguard])
        .mana(P0, green(6, 4))
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build()
}

#[test]
fn the_catalog_string_is_the_recorded_printing_and_both_layers_are_typed() {
    let def = druid().def();
    assert_eq!(def.cost, Some("{2}{G}"));
    assert_eq!(def.type_line, "Creature — Human Elf Druid");
    assert_eq!((def.power, def.toughness), (Some(2), Some(4)));
    assert_eq!(
        def.oracle,
        "Bear Form — During your turn, this creature is a Bear with base power and toughness 4/2. (It loses all other creature types.)"
    );
    // The printed ability is not activated and it is not an enters trigger.
    assert!(
        ability::abilities(druid()).is_empty(),
        "Bear Form is neither an activated nor a triggered ability"
    );
    assert!(
        !ability::triggers_on(druid(), TriggerEvent::EntersBattlefield)
            .next()
            .is_some(),
        "the card prints no enters trigger"
    );
    let statics = ability::static_abilities(druid());
    assert_eq!(statics.len(), 2, "one printed ability, two layers");
    assert_eq!(statics[0].layer(), Some(Layer::Type));
    assert_eq!(statics[1].layer(), Some(Layer::Setting));
    assert_eq!(
        statics
            .iter()
            .map(|ability| ability.text())
            .collect::<Vec<_>>(),
        vec![def.oracle; 2],
        "both halves quote the authoritative printing verbatim"
    );
    match statics[0] {
        StaticAbility::SelfBecomesSubtypeDuringYourTurn { line, .. } => {
            assert_eq!(line, SubtypeLine::Bear);
            assert!(line.has("Bear") && !line.has("Human"));
        }
        other => panic!("unexpected layer 4 static: {other:?}"),
    }
    match statics[1] {
        StaticAbility::SelfBasePowerToughnessDuringYourTurn {
            power, toughness, ..
        } => assert_eq!((power, toughness), (4, 2)),
        other => panic!("unexpected layer 7b static: {other:?}"),
    }
}

#[test]
fn it_enters_as_a_four_two_bear_during_its_controllers_turn_and_reverts_after() {
    let mut game = druid_game(2801);
    let id = cast_druid(&mut game, P0);

    assert_eq!(stats(&game, id), (4, 2), "layer 7b base 4/2 on your turn");
    assert!(
        game.state.effects.is_empty(),
        "Bear Form is a printed static, not a stored continuous effect"
    );
    assert!(
        bear_targets(&game, P0, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "the layer 4 line makes it a Bear for the engine's Bear filter"
    );
    assert!(
        bear_targets(&game, P0, TargetSpec::BearYouControl).contains(&Target::Permanent(id)),
        "Ayula's fight mode offers it as a Bear you control"
    );
    assert!(
        !bear_targets(&game, P0, TargetSpec::HumanYouControl).contains(&Target::Permanent(id)),
        "CR 205.3: it loses all other creature types, so it is no longer a Human"
    );

    // CR 611.3a: the turn ends, so both halves stop applying.
    assert_eq!(advance_turn(&mut game), P1);
    assert_eq!(
        stats(&game, id),
        (2, 4),
        "the printed 2/4 Human Elf Druid is back on an opponent's turn"
    );
    assert!(
        !bear_targets(&game, P0, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "it is not a Bear during an opponent's turn"
    );
    assert!(
        bear_targets(&game, P0, TargetSpec::HumanYouControl).contains(&Target::Permanent(id)),
        "and its printed Human subtype answers again"
    );

    // ... and it comes back when its controller's turn begins again.
    assert_eq!(advance_turn(&mut game), P0);
    assert_eq!(stats(&game, id), (4, 2), "Bear Form reapplies on your turn");
    assert!(
        bear_targets(&game, P0, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "the Bear subtype is back too"
    );
}

#[test]
fn the_base_setting_sits_under_layer_7c_and_under_counters() {
    // CR 613.4b precedes 613.4c: the pump adds to the 4/2 base, not the printed
    // 2/4, so Giant Growth makes a 7/5 rather than a 5/7.
    let mut game = druid_game(2802);
    let id = cast_druid(&mut game, P0);
    let growth = hand_card(&game, P0, CardId::GiantGrowth);
    let action = game
        .legal_actions()
        .into_iter()
        .find(
            |offered| matches!(&offered.action, Action::CastSpell { card, .. } if *card == growth),
        )
        .expect("Giant Growth must be castable")
        .action;
    submit_ok(&mut game, P0, action);
    for _ in 0..64 {
        if game.state.pending.is_none() {
            break;
        }
        let owner = game.decision_owner().expect("a target choice is decided");
        submit_ok(
            &mut game,
            owner,
            Action::ChooseTriggerTarget {
                target: Target::Permanent(id),
            },
        );
    }
    drain(&mut game);
    assert_eq!(stats(&game, id), (7, 5), "base 4/2 then +3/+3 in layer 7c");

    // A +1/+1 counter is layer 7d, applied on top of base 4/2 as well.
    let mut counters_game = druid_game(2803);
    let counters_id = cast_druid(&mut counters_game, P0);
    let bear = hand_card(&counters_game, P0, CardId::LittleBear);
    let action = counters_game
        .legal_actions()
        .into_iter()
        .find(|offered| matches!(&offered.action, Action::CastSpell { card, .. } if *card == bear))
        .expect("Little Bear must be castable")
        .action;
    submit_ok(&mut counters_game, P0, action);
    for _ in 0..64 {
        if counters_game.state.stack.is_empty() && counters_game.state.pending.is_none() {
            break;
        }
        let owner = counters_game
            .decision_owner()
            .expect("Little Bear's trigger is answered");
        if matches!(
            counters_game.state.pending,
            Some(Pending::TriggerTarget { .. })
        ) {
            submit_ok(
                &mut counters_game,
                owner,
                Action::ChooseTriggerTarget {
                    target: Target::Permanent(counters_id),
                },
            );
        } else {
            submit_ok(&mut counters_game, owner, Action::Pass);
        }
    }
    assert_eq!(
        counters_game
            .state
            .object(counters_id)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        1,
        "the engine's own Bear check grants Little Bear's counter"
    );
    assert_eq!(
        stats(&counters_game, counters_id),
        (5, 3),
        "base 4/2 plus one +1/+1 counter"
    );
}

#[test]
fn the_condition_is_its_controllers_turn_not_seat_zero() {
    // Three seats, Druid owned by P1: it is a 4/2 Bear on P1's turn and a plain
    // 2/4 on P2's, which is what "your turn" means for a non-first seat.
    let mut game = fixture_n(2804, 3)
        .hand(P1, &[druid()])
        .mana(P1, green(4, 2))
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .library(2, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .life(2, 40)
        .main(P1, Step::PrecombatMain)
        .build();
    let id = cast_druid(&mut game, P1);
    assert_eq!(stats(&game, id), (4, 2), "P1's turn");
    assert!(
        bear_targets(&game, P1, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "a Bear for P1"
    );

    assert_eq!(advance_turn(&mut game), 2);
    assert_eq!(stats(&game, id), (2, 4), "P2's turn, P1 is not active");
    assert!(
        !bear_targets(&game, P1, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "and not a Bear for anyone during P2's turn"
    );
}

#[test]
fn a_non_bear_creature_still_gets_no_counter_so_the_check_is_discriminating() {
    // The contrast case for the layer 4 half: Little Bear's trigger on a real
    // non-Bear (Elite Vanguard, a Human Soldier) grants nothing, so the counter
    // in the test above is evidence about the subtype and not about the trigger.
    let mut game = druid_game(2805);
    let vanguard = battlefield_card(&game, P0, CardId::EliteVanguard);
    let bear = hand_card(&game, P0, CardId::LittleBear);
    let action = game
        .legal_actions()
        .into_iter()
        .find(|offered| matches!(&offered.action, Action::CastSpell { card, .. } if *card == bear))
        .expect("Little Bear must be castable")
        .action;
    submit_ok(&mut game, P0, action);
    for _ in 0..64 {
        if game.state.stack.is_empty() && game.state.pending.is_none() {
            break;
        }
        let owner = game
            .decision_owner()
            .expect("Little Bear's trigger is answered");
        if matches!(game.state.pending, Some(Pending::TriggerTarget { .. })) {
            submit_ok(
                &mut game,
                owner,
                Action::ChooseTriggerTarget {
                    target: Target::Permanent(vanguard),
                },
            );
        } else {
            submit_ok(&mut game, owner, Action::Pass);
        }
    }
    assert_eq!(
        game.state
            .object(vanguard)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        0,
        "a Human Soldier is not a Bear, on any turn"
    );
}
