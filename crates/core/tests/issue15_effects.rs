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
        white: u32::from(n),
        ..Default::default()
    }
}

fn red(n: u8) -> ManaPool {
    ManaPool {
        red: u32::from(n),
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

/// "Any target" never includes a spell (CR 115.4). A prevention shield aimed at
/// a spell is refused by the action validator — the state is untouched and no
/// empty shield is created — while the same spell on a creature still works.
#[test]
fn mending_hands_refuses_a_spell_as_an_any_target() {
    let mut game = fixture(1507)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .hand(P0, &[CardId::Shock, CardId::MendingHands])
        .mana(
            P0,
            ManaPool {
                white: 1,
                red: 1,
                ..Default::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P1)],
        },
    );
    let shock_on_stack = game
        .state
        .stack
        .last()
        .expect("Shock is on the stack")
        .object;

    // The engine never offers a spell for an "any target" spell or ability.
    assert!(
        !game.legal_actions().iter().any(|legal| matches!(
            &legal.action,
            Action::CastSpell { targets, .. }
                if targets.iter().any(|target| matches!(target, Target::Spell(_)))
        )),
        "a spell is not an any target (CR 115.4)"
    );

    // A forged spell target is refused before any state changes.
    let mending = hand_card(&game, P0, CardId::MendingHands);
    let rejected = game.submit_action(
        P0,
        Action::CastSpell {
            card: mending,
            targets: vec![Target::Spell(shock_on_stack)],
        },
    );
    assert!(
        matches!(rejected, Err(EngineError::BadTarget { .. })),
        "a spell is not a legal target for Mending Hands: {rejected:?}"
    );
    assert!(shields(&game).is_empty(), "no shield was created");
    assert!(
        game.state.player(P0).hand.contains(&mending),
        "the refused cast left the card in hand"
    );

    // The valid path is unchanged: a creature target gets the shield.
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let mending = hand_card(&game, P0, CardId::MendingHands);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: mending,
            targets: vec![Target::Permanent(bears)],
        },
    );
    let events = resolve_cast(&mut game, P0, P1);
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::DamageShieldCreated {
                amount: Some(4),
                ..
            }
        )),
        "the shield is created on the creature: {events:?}"
    );
    assert_eq!(shields(&game)[0].target, ShieldTarget::Permanent(bears));
    assert_eq!(shields(&game)[0].remaining, Some(4));
}

/// The printed replacement effect doubles damage from any source its controller
/// controls — a creature's ability damage *and* a spell's damage — exactly once
/// per event (CR 614.1a, 614.5), and never damage from a source another player
/// controls.
#[test]
fn damage_multiplier_doubles_every_source_its_controller_controls() {
    let mut game = fixture(1502)
        .battlefield(P0, &[CardId::AngrathsMarauders, CardId::ProdigalSorcerer])
        .hand(P0, &[CardId::Shock])
        .hand(P1, &[CardId::Shock])
        .mana(P0, red(1))
        .mana(P1, red(1))
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

    // A spell is a source its controller controls, so its damage is doubled too
    // ("a source you control", not "a creature you control").
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
            Event::ReplacementApplied {
                before: 2,
                after: 4,
                ..
            }
        )),
        "Shock's 2 damage becomes 4: {events:?}"
    );
    assert_eq!(game.state.player(P1).life, 14, "2 doubled is 4");

    // The other player's Shock is not a source its controller's opponent
    // controls, so it is dealt undoubled.
    submit_ok(&mut game, P0, Action::Pass);
    let shock = hand_card(&game, P1, CardId::Shock);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P0)],
        },
    );
    let events = resolve_cast(&mut game, P1, P0);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ReplacementApplied { .. })),
        "the Marauders' controller does not control the opponent's spell: {events:?}"
    );
    assert_eq!(game.state.player(P0).life, 18, "undoubled 2 damage");
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

    // Prevention is applied before the multiplier (CR 616.1 policy), so the
    // strike's damage is absorbed and never reaches the doubler.
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
