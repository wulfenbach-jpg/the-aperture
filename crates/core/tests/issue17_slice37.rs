//! Slice 37: Druid's Familiar (AVR 175) — the pool's first Soulbond card.
//!
//! Printed text (Scryfall AVR 175, immutable id `1d8c794a-7964-41f0-bc9c-27af7cf87aaa`,
//! oracle id `6a5b1cd4-e597-4122-8fc7-4dce241d7c01`, retrieved 2026-10-27, both
//! printings — avr 175 and plst AVR-175 — carrying one identical text; raw
//! responses in `evidence/issue17/slice37/sources/`, fixture
//! `fixtures/starter-corpus/slice37-sourced.json`):
//!
//! "Soulbond (You may pair this creature with another unpaired creature when
//! either enters. They remain paired for as long as you control both of them.)
//! As long as this creature is paired with another creature, each of those
//! creatures gets +2/+2."
//!
//! CR 702.94a makes pairing a state of two permanents: the "you may pair"
//! trigger is optional, the pairing has no duration of its own, and it ends the
//! moment its controller stops controlling both creatures. Every assertion below
//! goes through the engine's own offers, resolutions, combat and state-based
//! actions; none reads a test-only shortcut.
mod common;

use aperture_core::ability::{self, Ability};
use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn familiar() -> CardId {
    catalog::by_name("Druid's Familiar").expect("Druid's Familiar must be supported")
}

fn stats(g: &Game, id: ObjectId) -> (i32, i32) {
    let object = g.state.object(id);
    (object.power(), object.toughness())
}

/// Pass priority until a triggering ability of the source waits for its pairing
/// choice, or until the stack settles. Returns every event the engine emitted.
fn pass_until_pairing_choice(g: &mut Game, limit: usize) -> Vec<Event> {
    let mut log = Vec::new();
    for _ in 0..limit {
        if matches!(g.state.pending, Some(Pending::TriggerTarget { .. })) {
            return log;
        }
        assert!(
            !g.state.is_over(),
            "the game ended while the trigger waited"
        );
        let Some(player) = g.decision_owner() else {
            return log;
        };
        log.extend(submit_ok(g, player, Action::Pass));
    }
    panic!(
        "the soulbond trigger never waited for its pairing choice (pending {:?})",
        g.state.pending
    );
}

fn pass_until_settled(g: &mut Game, limit: usize) -> Vec<Event> {
    let mut log = Vec::new();
    for _ in 0..limit {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return log;
        }
        let Some(player) = g.decision_owner() else {
            return log;
        };
        log.extend(submit_ok(g, player, Action::Pass));
    }
    panic!("the stack did not settle");
}

fn pay(g: &mut Game, seat: PlayerId, green: u32, colorless: u32) {
    let pool = &mut g.state.players[seat as usize].mana;
    pool.add(ManaColor::Green, green);
    pool.add(ManaColor::Colorless, colorless);
}

#[test]
fn druids_familiar_is_sourced_and_supported() {
    let def = familiar().def();
    assert_eq!(def.name, "Druid's Familiar");
    assert_eq!(def.cost, Some("{3}{G}"));
    assert_eq!(def.type_line, "Creature — Bear");
    assert_eq!((def.power, def.toughness), (Some(2), Some(2)));
    assert_eq!(
        def.oracle,
        "Soulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)\nAs long as this creature is paired with another creature, each of those creatures gets +2/+2."
    );

    let triggered: Vec<_> = ability::abilities(familiar())
        .iter()
        .filter_map(Ability::triggered)
        .collect();
    assert_eq!(triggered.len(), 2, "the printed 'when either enters' pair");
    assert_eq!(
        triggered[0].event,
        ability::TriggerEvent::EntersBattlefield,
        "this creature's own entry"
    );
    assert_eq!(
        triggered[1].event,
        ability::TriggerEvent::AnotherCreatureEnters,
        "another creature's entry"
    );
    for trigger in &triggered {
        assert_eq!(
            trigger.target,
            TargetSpec::UnpairedAnotherCreatureYouControl,
            "the choice is another unpaired creature you control"
        );
        assert_eq!(trigger.effects, &[Effect::PairSourceWithChosenCreature]);
    }
    let statics = ability::static_abilities(familiar());
    assert_eq!(statics.len(), 1);
    assert_eq!(
        statics[0].text(),
        "As long as this creature is paired with another creature, each of those creatures gets +2/+2."
    );
}

#[test]
fn entering_alone_pairs_with_nothing_and_stays_a_two_two() {
    let mut g = fixture(3701)
        .hand(P0, &[familiar()])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    pay(&mut g, P0, 1, 3);
    let card = hand_card(&g, P0, familiar());
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    let log = pass_until_settled(&mut g, 64);
    let id = battlefield_card(&g, P0, familiar());
    assert_eq!(
        stats(&g, id),
        (2, 2),
        "an unpaired Druid's Familiar is the printed 2/2"
    );
    assert!(g.paired_with(id).is_none(), "nothing to pair with");
    assert!(
        !log
            .iter()
            .any(|event| matches!(event, Event::StateBasedAction { description } if description.contains("pairs"))),
        "no pairing happens without another creature: {log:?}"
    );
}

#[test]
fn entering_beside_an_unpaired_creature_offers_pairing_and_both_get_two_two() {
    let mut g = fixture(3702)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .hand(P0, &[familiar()])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    let bears = battlefield_card(&g, P0, CardId::GrizzlyBears);
    pay(&mut g, P0, 1, 3);
    let card = hand_card(&g, P0, familiar());
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    pass_until_pairing_choice(&mut g, 64);
    let id = battlefield_card(&g, P0, familiar());
    assert_eq!(stats(&g, id), (2, 2), "unpaired until the choice resolves");

    let offers: Vec<Action> = g.legal_actions().into_iter().map(|a| a.action).collect();
    assert!(
        offers.contains(&Action::ChooseTriggerTarget {
            target: Target::Permanent(bears)
        }),
        "the unpaired creature is the pairing choice: {offers:?}"
    );
    assert!(
        !offers.contains(&Action::ChooseTriggerTarget {
            target: Target::Permanent(id)
        }),
        "soulbond pairs with *another* creature, never with the source"
    );
    assert!(
        offers.contains(&Action::DeclineTriggerTarget),
        "\"you may pair\" is optional, so declining is a legal answer"
    );

    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(bears),
        },
    );
    let log = pass_until_settled(&mut g, 64);
    assert_eq!(g.paired_with(id), Some(bears), "the pair is real state");
    assert_eq!(g.paired_with(bears), Some(id), "and it is mutual");
    assert_eq!(
        stats(&g, id),
        (4, 4),
        "the paired Druid's Familiar gets +2/+2"
    );
    assert_eq!(
        stats(&g, bears),
        (4, 4),
        "and so does the creature it is paired with"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description } if description.contains("soulbond, CR 702.94a")
        )),
        "the pairing is reported as a rules event: {log:?}"
    );
}

#[test]
fn declining_the_pairing_choice_leaves_both_creatures_unpaired() {
    let mut g = fixture(3703)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .hand(P0, &[familiar()])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    let bears = battlefield_card(&g, P0, CardId::GrizzlyBears);
    pay(&mut g, P0, 1, 3);
    let card = hand_card(&g, P0, familiar());
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    pass_until_pairing_choice(&mut g, 64);
    submit_ok(&mut g, P0, Action::DeclineTriggerTarget);
    pass_until_settled(&mut g, 64);
    let id = battlefield_card(&g, P0, familiar());
    assert!(g.paired_with(id).is_none(), "declined: no pair either way");
    assert!(g.paired_with(bears).is_none());
    assert_eq!(stats(&g, id), (2, 2));
    assert_eq!(stats(&g, bears), (2, 2));
}

#[test]
fn a_later_creature_entering_pairs_with_the_unpaired_familiar() {
    let mut g = fixture(3704)
        .battlefield(P0, &[familiar()])
        .hand(P0, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = battlefield_card(&g, P0, familiar());
    pay(&mut g, P0, 1, 1);
    let card = hand_card(&g, P0, CardId::GrizzlyBears);
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    pass_until_pairing_choice(&mut g, 64);
    let bears = battlefield_card(&g, P0, CardId::GrizzlyBears);
    // "when either enters": the familiar was already on the battlefield, so the
    // second half of the printed trigger is what offers the pairing here.
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(bears),
        },
    );
    pass_until_settled(&mut g, 64);
    assert_eq!(g.paired_with(id), Some(bears));
    assert_eq!(stats(&g, id), (4, 4));
    assert_eq!(stats(&g, bears), (4, 4));
}

#[test]
fn the_pair_ends_when_its_partner_leaves_the_battlefield() {
    let mut g = fixture(3705)
        .battlefield(P0, &[CardId::EliteVanguard])
        .hand(P0, &[familiar(), CardId::LightningBolt])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    let vanguard = battlefield_card(&g, P0, CardId::EliteVanguard);
    pay(&mut g, P0, 1, 3);
    let card = hand_card(&g, P0, familiar());
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    pass_until_pairing_choice(&mut g, 64);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(vanguard),
        },
    );
    pass_until_settled(&mut g, 64);
    let id = battlefield_card(&g, P0, familiar());
    assert_eq!(g.paired_with(id), Some(vanguard));
    assert_eq!(stats(&g, id), (4, 4));
    assert_eq!(stats(&g, vanguard), (4, 3), "a pumped 2/1");

    // CR 702.94a: the pair lasts only while one player controls both. Killing
    // the partner ends it, and the familiar drops back to its printed 2/2.
    pay(&mut g, P0, 0, 0);
    g.state.players[P0 as usize].mana.add(ManaColor::Red, 1);
    let bolt = hand_card(&g, P0, CardId::LightningBolt);
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Permanent(vanguard)],
        },
    );
    pass_until_settled(&mut g, 64);
    let log = pass_until_settled(&mut g, 4);
    assert!(
        log.is_empty()
            || !log
                .iter()
                .any(|event| matches!(event, Event::Destroyed { .. })),
        "sanity: settlement does not re-destroy anything"
    );
    assert!(
        g.state.try_object(vanguard).is_none()
            || g.state.object(vanguard).zone != Zone::Battlefield,
        "the partner is no longer on the battlefield"
    );
    assert!(
        g.paired_with(id).is_none(),
        "the pair ended with its partner (CR 702.94a)"
    );
    assert_eq!(stats(&g, id), (2, 2), "an ended pair grants no +2/+2");
}
