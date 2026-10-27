//! Slice 26: Ursine Champion — the pool's first layer 4 type-changing effect and
//! its first "Activate only once each turn" restriction.
//!
//! The printed ability is "{5}{G}: This creature gets +3/+3 and becomes a Bear
//! Berserker until end of turn. Activate only once each turn." (M19 304,
//! retrieved 2026-10-27, recorded in fixtures/starter-corpus/slice26-sourced.json).
//!
//! The interesting rules surface is that the subtype change is a *layer 4*
//! continuous effect (CR 613.1d): it replaces the printed subtype line for every
//! rules question, so Little Bear's "if that creature is a Bear" branch and every
//! Bear target filter must read the effective subtype. These tests exercise that
//! through real resolutions rather than through an internal accessor.
mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn champion() -> CardId {
    catalog::by_name("Ursine Champion").expect("Ursine Champion must be supported")
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

/// Activate the printed ability through the action the engine actually offers.
fn activate_champion(g: &mut Game, source: ObjectId) {
    let action = g
        .legal_actions()
        .into_iter()
        .find(|offered| {
            matches!(
                &offered.action,
                Action::ActivateAbility { source: offered_source, .. } if *offered_source == source
            )
        })
        .expect("the printed {5}{G} ability must be offered with six mana available")
        .action;
    submit_ok(g, P0, action);
    drain(g);
}

/// Cast Little Bear and point its enters trigger ("untap another target creature
/// you control; if that creature is a Bear, put a +1/+1 counter on it") at
/// `target`. The counter is the engine's own answer to "is this a Bear?".
fn cast_little_bear_at(g: &mut Game, target: ObjectId) {
    let id = hand_card(g, P0, CardId::LittleBear);
    let action = g
        .legal_actions()
        .into_iter()
        .find(|offered| matches!(&offered.action, Action::CastSpell { card, .. } if *card == id))
        .expect("Little Bear must be castable")
        .action;
    submit_ok(g, P0, action);
    for _ in 0..64 {
        if g.state.stack.is_empty() && g.state.pending.is_none() {
            return;
        }
        let Some(player) = g.decision_owner() else {
            return;
        };
        if matches!(g.state.pending, Some(Pending::TriggerTarget { .. })) {
            submit_ok(
                g,
                player,
                Action::ChooseTriggerTarget {
                    target: Target::Permanent(target),
                },
            );
        } else {
            submit_ok(g, player, Action::Pass);
        }
    }
    panic!("Little Bear's trigger did not settle");
}

fn champion_game(seed: u64) -> Game {
    fixture(seed)
        .battlefield(
            P0,
            &[
                champion(),
                CardId::EliteVanguard,
                // {5}{G} is six mana: six untapped Forests.
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        // Two Little Bears: one for the Bear case, one for the contrast case.
        .hand(P0, &[CardId::LittleBear, CardId::LittleBear])
        .mana(P0, green(8, 4))
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Forest; 10])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build()
}

fn typed_layers(g: &Game) -> Vec<aperture_core::layer::ContinuousKind> {
    g.state
        .effects
        .iter()
        .filter(|effect| effect.layer == aperture_core::layer::Layer::Type)
        .map(|effect| effect.kind)
        .collect()
}

#[test]
fn the_catalog_string_is_the_recorded_printing_and_the_ability_is_typed() {
    let def = champion().def();
    assert_eq!(def.cost, Some("{1}{G}"));
    assert_eq!(def.type_line, "Creature — Human Berserker");
    assert_eq!((def.power, def.toughness), (Some(2), Some(2)));
    assert_eq!(
        def.oracle,
        "{5}{G}: This creature gets +3/+3 and becomes a Bear Berserker until end of turn. Activate only once each turn."
    );
    let abilities = ability::abilities(champion());
    assert_eq!(abilities.len(), 1, "one printed activated ability");
    let activated = abilities[0].activated().expect("an activated ability");
    assert_eq!(
        activated.text,
        "{5}{G}: This creature gets +3/+3 and becomes a Bear Berserker until end of turn. Activate only once each turn."
    );
    assert_eq!(activated.cost.mana, Some("{5}{G}"));
    assert!(activated.once_per_turn, "the printed once-each-turn limit");
    assert!(
        !activated.sorcery_timing,
        "no timing restriction is printed"
    );
    assert_eq!(
        activated.effects,
        &[
            Effect::PumpSource(3, 3),
            Effect::SetSourceCreatureSubtypes(aperture_core::layer::SubtypeLine::BearBerserker),
        ]
    );
}

#[test]
fn activation_pumps_the_source_and_makes_it_a_bear_for_a_real_bear_check() {
    let mut game = champion_game(2601);
    let source = battlefield_card(&game, P0, champion());

    activate_champion(&mut game, source);

    assert_eq!(
        (
            game.state.object(source).power(),
            game.state.object(source).toughness()
        ),
        (5, 5),
        "2/2 plus the printed +3/+3 in layer 7c"
    );
    assert_eq!(
        typed_layers(&game).len(),
        1,
        "exactly one layer 4 type-changing effect is active"
    );

    // CR 205.3 through a real resolution: Little Bear untaps the target and puts
    // a +1/+1 counter on it only if the target *is* a Bear. Before the type
    // change Ursine Champion is a Human Berserker, so this counter is the
    // behavioural proof that the layer 4 line replaced the printed subtypes.
    cast_little_bear_at(&mut game, source);
    assert_eq!(
        game.state
            .object(source)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        1,
        "the layer 4 subtype change makes it a Bear for Little Bear"
    );
    assert_eq!(
        (
            game.state.object(source).power(),
            game.state.object(source).toughness()
        ),
        (6, 6),
        "the counter is applied on top of the pump"
    );

    // The contrast case: a non-Bear creature still gets no counter.
    let vanguard = battlefield_card(&game, P0, CardId::EliteVanguard);
    cast_little_bear_at(&mut game, vanguard);
    assert_eq!(
        game.state
            .object(vanguard)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        0,
        "a Human Soldier is not a Bear"
    );
}

#[test]
fn the_type_change_and_pump_both_end_with_the_turn() {
    let mut game = champion_game(2602);
    let source = battlefield_card(&game, P0, champion());
    activate_champion(&mut game, source);
    assert_eq!(typed_layers(&game).len(), 1);

    // CR 514.2: the until-end-of-turn effects are gone once cleanup runs.
    game.state.turn.step = Step::EndStep;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    for _ in 0..8 {
        if !matches!(game.state.turn.step, Step::EndStep) {
            break;
        }
        let Some(player) = game.decision_owner() else {
            break;
        };
        let _ = game.submit_action(player, Action::Pass);
    }
    assert_ne!(game.state.turn.step, Step::EndStep, "cleanup ran");
    assert!(
        typed_layers(&game).is_empty(),
        "no layer 4 effect survives cleanup"
    );
    assert_eq!(
        (
            game.state.object(source).power(),
            game.state.object(source).toughness()
        ),
        (2, 2),
        "the pump expires with the type change"
    );

    // The printed subtype line is back: Little Bear no longer counts it as a Bear.
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    assert!(
        !game
            .legal_targets(P0, TargetSpec::BearYouControl, ObjectId(u32::MAX))
            .contains(&Target::Permanent(source)),
        "after cleanup the printed Human Berserker line answers the Bear filter again"
    );
}

#[test]
fn the_printed_once_each_turn_restriction_is_enforced() {
    let mut game = champion_game(2603);
    let source = battlefield_card(&game, P0, champion());
    // Twelve Forests worth of mana: two activations would be affordable if the
    // restriction did not exist.
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
    activate_champion(&mut game, source);

    assert!(
        !game
            .legal_actions()
            .iter()
            .any(|offered| matches!(offered.action, Action::ActivateAbility { source: s, .. } if s == source)),
        "CR 602.5b: the ability is not offered a second time this turn"
    );
    let before = game.state.clone();
    let err = game
        .submit_action(
            P0,
            Action::ActivateAbility {
                source,
                ability: 0,
                mode: 0,
                targets: Vec::new(),
            },
        )
        .expect_err("the second activation must be refused");
    assert!(
        format!("{err:?}").to_lowercase().contains("once"),
        "the refusal names the once-each-turn restriction: {err:?}"
    );
    assert_eq!(game.state, before, "a refused activation changes nothing");
    assert_eq!(
        game.state.object(source).power(),
        5,
        "no second +3/+3 is applied"
    );
}

#[test]
fn the_activation_is_not_offered_without_the_printed_six_mana() {
    let mut game = fixture(2604)
        .battlefield(
            P0,
            &[champion(), CardId::Forest, CardId::Forest, CardId::Forest],
        )
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Forest; 10])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, champion());
    assert!(
        !game
            .legal_actions()
            .iter()
            .any(|offered| matches!(offered.action, Action::ActivateAbility { source: s, .. } if s == source)),
        "three Forests cannot pay {{5}}{{G}}"
    );
    let err = game
        .submit_action(
            P0,
            Action::ActivateAbility {
                source,
                ability: 0,
                mode: 0,
                targets: Vec::new(),
            },
        )
        .expect_err("an unaffordable activation is refused");
    assert!(
        format!("{err:?}").to_lowercase().contains("cannotpay"),
        "the refusal is about the mana cost: {err:?}"
    );
    assert_eq!(game.state.object(source).power(), 2);
}

#[test]
fn a_bear_target_filter_sees_the_new_subtype() {
    // CR 613.1d is not private to one effect: the engine's Bear target filter
    // must answer with the effective subtype too. Ayula's counters mode targets
    // "a Bear you control", so the offer list is a direct probe.
    let mut game = champion_game(2605);
    let source = battlefield_card(&game, P0, champion());
    let offered_before = game.legal_targets(P0, TargetSpec::BearYouControl, ObjectId(u32::MAX));
    assert!(
        !offered_before.contains(&Target::Permanent(source)),
        "a Human Berserker is not a Bear target"
    );
    activate_champion(&mut game, source);
    let offered_after = game.legal_targets(P0, TargetSpec::BearYouControl, ObjectId(u32::MAX));
    assert!(
        offered_after.contains(&Target::Permanent(source)),
        "after the layer 4 change the same object is a legal Bear target"
    );
}
