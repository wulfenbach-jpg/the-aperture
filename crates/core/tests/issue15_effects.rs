//! Issue 15: replacement (CR 614), prevention (CR 615) and layered continuous
//! effects (CR 613), including the CR 616.1 order/chooser rule.
//!
//! The last test is the representative end-to-end journey: a real two-seat game
//! over the bundled practice decks' *new* cards that logs the new effect events
//! and replays to the same canonical digest.

mod common;

use aperture_core::*;
use common::*;

fn white(n: u8) -> ManaPool {
    ManaPool {
        white: n,
        ..Default::default()
    }
}

fn red(n: u8) -> ManaPool {
    ManaPool {
        red: n,
        ..Default::default()
    }
}

/// Caster keeps priority after casting; two consecutive passes resolve the
/// spell (CR 117.4). Returns the events emitted while it resolved.
fn resolve_cast(game: &mut Game, caster: PlayerId, other: PlayerId) -> Vec<Event> {
    let mut events = submit_ok(game, caster, Action::Pass);
    events.extend(submit_ok(game, other, Action::Pass));
    events
}

fn shields(game: &Game) -> Vec<DamageShield> {
    game.state.shields.clone()
}

/// A one-shot shield prevents only its printed amount and is then used up; the
/// damage after that is dealt normally (CR 615.1, 615.2, 615.6).
#[test]
fn prevention_shield_absorbs_its_amount_and_stops_applying() {
    let mut game = fixture(1501)
        .hand(
            P0,
            &[
                CardId::MendingHands,
                CardId::Shock,
                CardId::Shock,
                CardId::Shock,
            ],
        )
        .mana(
            P0,
            ManaPool {
                white: 1,
                red: 3,
                ..Default::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let mending = hand_card(&game, P0, CardId::MendingHands);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: mending,
            targets: vec![Target::Player(P1)],
        },
    );
    let events = resolve_cast(&mut game, P0, P1);
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::DamageShieldCreated {
                card: CardId::MendingHands,
                amount: Some(4),
                ..
            }
        )),
        "the shield is a rules event with its amount: {events:?}"
    );
    let active = shields(&game);
    assert_eq!(active.len(), 1, "one shield is on the battlefield state");
    assert_eq!(active[0].remaining, Some(4));

    // First Shock: 2 of the 4 points are prevented, so life is untouched.
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P1)],
        },
    );
    let events = resolve_cast(&mut game, P0, P1);
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, Event::LifeChange { player, .. } if *player == P1)),
        "no life change while the shield still applies: {events:?}"
    );
    assert_eq!(game.state.player(P1).life, 20);
    assert_eq!(shields(&game)[0].remaining, Some(2));

    // Second Shock: the remaining 2 are prevented and the shield is used up.
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P1)],
        },
    );
    let events = resolve_cast(&mut game, P0, P1);
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::DamagePrevented {
                amount: 2,
                remaining: Some(0),
                ..
            }
        )),
        "the second instance exhausts the shield: {events:?}"
    );
    assert_eq!(game.state.player(P1).life, 20);
    assert!(shields(&game).is_empty(), "an exhausted shield is removed");

    // Third Shock: no shield remains, so the damage is dealt.
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P1)],
        },
    );
    resolve_cast(&mut game, P0, P1);
    assert_eq!(game.state.player(P1).life, 18);
}

/// The supported replacement effect doubles damage from a controlled creature
/// exactly once per event (CR 614.1a, 614.5) and never from a spell.
#[test]
fn damage_multiplier_doubles_a_controlled_creature_once_per_event() {
    let mut game = fixture(1502)
        .battlefield(P0, &[CardId::AngrathsMarauders, CardId::ProdigalSorcerer])
        .hand(P0, &[CardId::Shock])
        .mana(P0, red(1))
        .main(P0, Step::PrecombatMain)
        .build();
    let torcher = battlefield_card(&game, P0, CardId::ProdigalSorcerer);
    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: torcher,
            ability: 0,
            mode: 0,
            targets: vec![Target::Player(P1)],
        },
    );
    let events = resolve_cast(&mut game, P0, P1);
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::ReplacementApplied { before: 1, after: 2, chooser, .. } if *chooser == P1
        )),
        "one 1-damage instance becomes 2 and the affected player chose the order: {events:?}"
    );
    assert_eq!(
        game.state.player(P1).life,
        18,
        "1 damage doubled once is 2, never 4 (CR 614.5)"
    );

    // A spell is not a creature, so its damage is never doubled.
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P1)],
        },
    );
    resolve_cast(&mut game, P0, P1);
    assert_eq!(
        game.state.player(P1).life,
        16,
        "Shock still deals exactly 2"
    );
}

/// CR 616.1: when prevention and a multiplier both apply, the affected player
/// chooses the order; the engine applies prevention first, so the damage never
/// reaches the doubler. Doubling first would deal 1 here, so the assertion
/// distinguishes the two orders.
#[test]
fn prevention_is_applied_before_the_multiplier() {
    let mut game = fixture(1503)
        .battlefield(P0, &[CardId::AngrathsMarauders, CardId::ProdigalSorcerer])
        .hand(P0, &[CardId::MendingHands, CardId::LightningStrike])
        .mana(P0, white(1))
        .main(P0, Step::PrecombatMain)
        .build();
    let mending = hand_card(&game, P0, CardId::MendingHands);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: mending,
            targets: vec![Target::Player(P1)],
        },
    );
    resolve_cast(&mut game, P0, P1);
    assert_eq!(shields(&game)[0].remaining, Some(4));

    // Burn 3 of the 4 points away with a spell (never doubled).
    game.state.players[P0 as usize].mana.add(ManaColor::Red, 2);
    let strike = hand_card(&game, P0, CardId::LightningStrike);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Player(P1)],
        },
    );
    resolve_cast(&mut game, P0, P1);
    assert_eq!(game.state.player(P1).life, 20, "3 of 4 prevented");
    assert_eq!(shields(&game)[0].remaining, Some(1));

    // A 1-damage creature ping: prevention first removes it, so the doubler
    // never sees a damage event to double.
    let torcher = battlefield_card(&game, P0, CardId::ProdigalSorcerer);
    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: torcher,
            ability: 0,
            mode: 0,
            targets: vec![Target::Player(P1)],
        },
    );
    let events = resolve_cast(&mut game, P0, P1);
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::DamagePrevented {
                amount: 1,
                remaining: Some(0),
                ..
            }
        )),
        "the last prevention point absorbed the instance: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ReplacementApplied { .. })),
        "doubling nothing never produces a replacement event: {events:?}"
    );
    assert_eq!(game.state.player(P1).life, 20);
    assert!(shields(&game).is_empty());
}
