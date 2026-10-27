//! Slice 33: Beorn's Hospitality (HOB 120) — the pool's first layer 7a
//! characteristic-defining power/toughness, and the first animation that does
//! not end.
//!
//! The printed text is (HOB 120, retrieved 2026-10-27T18:21:29Z, recorded in
//! `fixtures/starter-corpus/slice33-sourced.json`):
//!
//! "Landfall — Whenever a land you control enters, put a +1/+1 counter on
//! target creature you control.
//! {5}{G}{G}: This enchantment becomes a Bear creature in addition to its other
//! types and gains \"This creature's power and toughness are each equal to the
//! number of lands you control.\" (This effect doesn't end.)"
//!
//! Two printed abilities, two new rules surfaces:
//!
//! * the animation is layer 4 (CR 613.1d) *additive* — the enchantment gains
//!   the creature card type and the Bear subtype without losing Enchantment —
//!   plus layer 7a (CR 613.4a): a characteristic-defining ability whose value is
//!   the number of lands its controller controls. A CDA is re-evaluated every
//!   time the game asks for characteristics (CR 604.3), so a land that enters
//!   *after* the animation raises the power and toughness without the ability
//!   resolving again. That is this slice's discriminating property: a snapshot
//!   taken at resolution (layer 7b) would fail the live-update test.
//! * "(This effect doesn't end.)" (CR 611.2b) means cleanup does not remove it;
//!   it lasts as long as that permanent is the same object on the battlefield.
//!
//! Every assertion below is made through the engine's own offers, resolutions
//! and queries (`is_creature`, `has_subtype`, legal actions, `power()`,
//! `toughness()`), never through a test-only shortcut.
mod common;

use aperture_core::ability::{self, Ability};
use aperture_core::catalog;
use aperture_core::layer::SubtypeLine;
use aperture_core::*;
use common::*;

fn hospitality() -> CardId {
    catalog::by_name("Beorn's Hospitality").expect("Beorn's Hospitality must be supported")
}

/// {5}{G}{G}: five generic in the colourless slot plus two green pips.
fn seven_mana() -> ManaPool {
    ManaPool {
        colorless: 5,
        green: 2,
        ..ManaPool::default()
    }
}

fn drain(g: &mut Game) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..128 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return out;
        }
        let Some(player) = g.decision_owner() else {
            return out;
        };
        out.extend(submit_ok(g, player, Action::Pass));
    }
    panic!("the stack did not settle");
}

/// The engine's own legal-target list for a spec: the public way to ask what a
/// card's effective type line is.
fn g_offered(g: &Game, spec: TargetSpec) -> Vec<Target> {
    g.legal_targets(P0, spec, ObjectId(u32::MAX))
}

fn stats(g: &Game, id: ObjectId) -> (i32, i32) {
    let object = g.state.object(id);
    (object.power(), object.toughness())
}

fn animate(g: &mut Game, player: PlayerId, source: ObjectId) -> Vec<Event> {
    let action = g
        .legal_actions()
        .into_iter()
        .find(|offered| {
            matches!(
                &offered.action,
                Action::ActivateAbility { source: s, .. } if *s == source
            )
        })
        .expect("the animation must be offered while {5}{G}{G} is payable")
        .action;
    let mut events = submit_ok(g, player, action);
    events.extend(drain(g));
    events
}

fn advance_turn(g: &mut Game) -> PlayerId {
    let start = g.state.turn.number;
    for _ in 0..800 {
        if g.state.turn.number != start {
            return g.state.turn.active;
        }
        assert!(!g.state.is_over(), "the game ended while advancing");
        let owner = g.decision_owner().expect("a seat decides while advancing");
        let action = auto_answer(g);
        let _ = g.submit_action(owner, action);
    }
    panic!("the turn did not advance");
}

/// Three lands under P0, seven mana available and one land in hand.
fn hospitality_game(seed: u64) -> Game {
    fixture(seed)
        .battlefield(
            P0,
            &[
                hospitality(),
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .battlefield(P1, &[CardId::Forest, CardId::Forest])
        .hand(P0, &[CardId::Forest])
        .mana(P0, seven_mana())
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build()
}

#[test]
fn beorns_hospitality_is_sourced_and_supported() {
    let def = hospitality().def();
    assert_eq!(def.name, "Beorn's Hospitality");
    assert_eq!(def.cost, Some("{1}{G}"));
    assert_eq!(def.type_line, "Enchantment");
    assert!(def.kinds.contains(&Kind::Enchantment));
    assert!(
        !def.is_creature(),
        "printed as a noncreature permanent (CR 303)"
    );
    assert!(def.power.is_none() && def.toughness.is_none());
    assert_eq!(
        def.oracle,
        "Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature you control.\n{5}{G}{G}: This enchantment becomes a Bear creature in addition to its other types and gains \"This creature's power and toughness are each equal to the number of lands you control.\" (This effect doesn't end.)"
    );
    // CR 202.2a: the printing is green, and once animated its colour is an
    // observable characteristic, so the pool records it.
    assert_eq!(def.colors, &[ManaColor::Green]);

    let abilities = ability::abilities(hospitality());
    assert_eq!(abilities.len(), 2, "two printed abilities");
    let Ability::Triggered(landfall) = abilities[0] else {
        panic!("the landfall trigger is a triggered ability");
    };
    assert_eq!(landfall.event, ability::TriggerEvent::LandEnters);
    assert_eq!(landfall.target, TargetSpec::CreatureYouControl);
    assert_eq!(landfall.effects, &[Effect::PutPlusOneCountersOnTarget(1)]);
    assert!(!landfall.optional);

    let Ability::Activated(animation) = abilities[1] else {
        panic!("the animation is an activated ability");
    };
    assert_eq!(animation.cost.mana, Some("{5}{G}{G}"));
    assert_eq!(animation.target, TargetSpec::None);
    assert_eq!(
        animation.effects,
        &[
            Effect::AddSourceCreatureTypeWhileOnBattlefield(SubtypeLine::Bear),
            Effect::SetSourcePowerToughnessEqualToLands,
        ]
    );
    assert!(
        !animation.sorcery_timing,
        "no sorcery-only restriction is printed"
    );
}

#[test]
fn the_animation_adds_the_bear_type_and_reads_the_land_count_live() {
    let mut game = hospitality_game(3301);
    let id = battlefield_card(&game, P0, hospitality());
    assert!(
        !game.is_creature(game.state.object(id)),
        "an enchantment is not a creature yet"
    );

    animate(&mut game, P0, id);

    assert!(
        game.is_creature(game.state.object(id)),
        "CR 613.1d: the animation added the creature card type"
    );
    assert!(
        game.state
            .object(id)
            .card
            .def()
            .kinds
            .contains(&Kind::Enchantment),
        "additive: the printed type line is untouched"
    );
    assert!(
        g_offered(&game, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "the Bear subtype was added"
    );
    assert!(
        game.has_color(id, ManaColor::Green),
        "the printing is green"
    );
    assert_eq!(stats(&game, id), (3, 3), "three lands you control");

    // The effect is 7a, not a 7b snapshot: a land that enters afterwards raises
    // the value with no second activation.
    let land = hand_card(&game, P0, CardId::Forest);
    let mut log = submit_ok(&mut game, P0, Action::PlayLand { card: land });
    log.extend(drain(&mut game));
    assert_eq!(stats(&game, id), (4, 4), "CR 604.3: re-read from the board");

    let mut layers: Vec<Layer> = game
        .state
        .effects
        .iter()
        .filter(|effect| effect.target == id)
        .map(|effect| effect.layer)
        .collect();
    layers.sort_unstable();
    assert_eq!(
        layers,
        vec![Layer::Type, Layer::CharacteristicDefining],
        "one effect per layer"
    );
    assert!(
        game.state
            .effects
            .iter()
            .any(|effect| effect.kind == ContinuousKind::AddCreatureSubtypes(SubtypeLine::Bear)),
        "layer 4 additive type change"
    );
    assert!(
        game.state
            .effects
            .iter()
            .any(|effect| effect.kind == ContinuousKind::PowerToughnessEqualToLands),
        "layer 7a characteristic-defining value"
    );
    assert!(
        game.state
            .effects
            .iter()
            .all(|effect| effect.duration == EffectDuration::WhileSourceOnBattlefield),
        "\"This effect doesn't end.\" is not an until-end-of-turn duration"
    );
}

#[test]
fn a_pump_applies_on_top_of_the_characteristic_defining_value() {
    let mut game = hospitality_game(3302);
    let id = battlefield_card(&game, P0, hospitality());
    animate(&mut game, P0, id);
    assert_eq!(stats(&game, id), (3, 3));

    // CR 613.4c: a 7c modifier is applied after 7a, and CR 122.1 counters are
    // applied after every layer, so both sit on top of the CDA value.
    object_mut(&mut game.state, id)
        .counters
        .add(CounterKind::PlusOnePlusOne, 2);
    object_mut(&mut game.state, id).pump_power += 2;
    object_mut(&mut game.state, id).pump_toughness += 2;
    assert_eq!(stats(&game, id), (7, 7), "3 lands + 2 pump + 2 counters");

    let object = object_mut(&mut game.state, id);
    object.counters = Counters::default();
    object.pump_power = 0;
    object.pump_toughness = 0;
    // A fresh characteristic pass restores exactly the land count.
    let land = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: land });
    let _ = drain(&mut game);
    assert_eq!(stats(&game, id), (4, 4));
}

#[test]
fn the_animation_outlives_cleanup_and_ends_with_the_permanent() {
    let mut game = hospitality_game(3303);
    let id = battlefield_card(&game, P0, hospitality());
    animate(&mut game, P0, id);
    assert_eq!(stats(&game, id), (3, 3));

    assert_eq!(advance_turn(&mut game), P1, "the turn ended");
    assert!(
        game.state.object(id).zone == Zone::Battlefield,
        "the enchantment is still there"
    );
    assert_eq!(
        stats(&game, id),
        (3, 3),
        "CR 611.2b: the printed effect doesn't end at cleanup"
    );
    assert!(
        game.is_creature(game.state.object(id)),
        "still an animated enchantment creature"
    );
    assert!(
        g_offered(&game, TargetSpec::Bear).contains(&Target::Permanent(id)),
        "and still a Bear"
    );

    // A departed permanent is a new object if it comes back (CR 400.7), so the
    // animation ends with this incarnation. Lethal damage is a real state-based
    // action (CR 704.5g), not a hand-written zone change, so the same pipeline
    // that runs in a game prunes the effects.
    object_mut(&mut game.state, id).damage = 3;
    let owner = game.decision_owner().expect("a seat holds priority");
    submit_ok(&mut game, owner, Action::Pass);
    let mut log = Vec::new();
    assert!(
        run_until(
            &mut game,
            &mut log,
            |g| g.state.object(id).zone == Zone::Graveyard,
            64
        ),
        "the animated 3/3 dies to three damage"
    );
    assert!(
        game.state.effects.iter().all(|effect| effect.target != id),
        "an effect on a departed permanent stops applying"
    );
}

#[test]
fn landfall_targets_a_creature_you_control_and_puts_one_counter() {
    let mut game = fixture(3304)
        .battlefield(
            P0,
            &[hospitality(), CardId::GrizzlyBears, CardId::EliteVanguard],
        )
        .battlefield(P1, &[CardId::GrizzlyBears])
        .hand(P0, &[CardId::Forest])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build();
    let own_bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let own_vanguard = battlefield_card(&game, P0, CardId::EliteVanguard);
    let their_bear = battlefield_card(&game, P1, CardId::GrizzlyBears);
    let land = hand_card(&game, P0, CardId::Forest);

    let mut log = submit_ok(&mut game, P0, Action::PlayLand { card: land });
    let Some(Pending::TriggerTarget { player, legal, .. }) = game.state.pending.clone() else {
        panic!("the mandatory landfall trigger asks its controller for a target");
    };
    assert_eq!(player, P0, "the watcher's controller chooses");
    assert_eq!(
        legal,
        vec![Target::Permanent(own_bear), Target::Permanent(own_vanguard)],
        "only that seat's own creatures are legal targets"
    );
    assert!(
        !legal.contains(&Target::Permanent(their_bear)),
        "an opponent's creature is not a legal target"
    );
    log.extend(submit_ok(
        &mut game,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(own_bear),
        },
    ));
    log.extend(drain(&mut game));
    assert_eq!(
        game.state
            .object(own_bear)
            .counters
            .get(CounterKind::PlusOnePlusOne),
        1
    );
    assert_eq!(
        stats(&game, own_bear),
        (3, 3),
        "2/2 printed plus one counter"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::TriggerStaged { card, .. } if *card == hospitality()
        )),
        "the landfall trigger was staged on the stack"
    );
}

#[test]
fn an_opponents_land_does_not_trigger_it() {
    let mut game = fixture(3305)
        .battlefield(P0, &[hospitality(), CardId::GrizzlyBears])
        .hand(P1, &[CardId::Forest])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P1, Step::PrecombatMain)
        .build();
    let own_bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let land = hand_card(&game, P1, CardId::Forest);

    let mut log = submit_ok(&mut game, P1, Action::PlayLand { card: land });
    log.extend(drain(&mut game));
    assert_eq!(
        game.state
            .object(own_bear)
            .counters
            .get(CounterKind::PlusOnePlusOne),
        0,
        "CR 603.6a: the trigger watches the source's controller's lands"
    );
    assert!(
        !log.iter().any(|event| matches!(
            event,
            Event::TriggerStaged { card, .. } if *card == hospitality()
        )),
        "no trigger for an opponent's land"
    );
}

#[test]
fn the_animation_is_not_offered_without_its_mana_cost() {
    let game = fixture(3306)
        .battlefield(P0, &[hospitality(), CardId::Forest])
        .mana(
            P0,
            ManaPool {
                colorless: 2,
                green: 1,
                ..ManaPool::default()
            },
        )
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = battlefield_card(&game, P0, hospitality());
    assert!(
        !game.legal_actions().iter().any(|offered| matches!(
            &offered.action,
            Action::ActivateAbility { source, .. } if *source == id
        )),
        "{{5}}{{G}}{{G}} is not payable, so the ability is not offered"
    );
    assert!(!game.is_creature(game.state.object(id)));
}
