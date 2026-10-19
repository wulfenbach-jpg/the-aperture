//! Counterspell resolution, targeting rules, and target revalidation.
//! CR 701.6a, 608.2b, 115.1.

mod common;

use aperture_core::*;
use common::*;

fn counter_fixture() -> Game {
    fixture(30)
        .hand(P0, &[CardId::Counterspell])
        .hand(P1, &[CardId::Shock])
        .battlefield(P0, &[CardId::Island, CardId::Island])
        .battlefield(P1, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build()
}

#[test]
fn counterspell_counters_the_target_spell_and_nothing_else_happens() {
    let mut game = counter_fixture();
    // P0 passes; P1 casts Shock at P0 in response.
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
    let shock_object = game.state.stack.last().unwrap().object;
    submit_ok(&mut game, P1, Action::Pass);

    // P0 counters it.
    let counter = hand_card(&game, P0, CardId::Counterspell);
    let events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: counter,
            targets: vec![Target::Spell(shock_object)],
        },
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::Cast {
                card: CardId::Counterspell,
                ..
            }
        )),
        "Counterspell is on the stack"
    );
    assert_eq!(game.state.stack.len(), 2);

    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.stack.len(), 0, "both spells left the stack");
    assert_eq!(game.state.player(P0).life, 20, "the Shock never resolved");
    assert_eq!(
        names(&game, &game.state.player(P1).graveyard.to_vec()),
        vec!["Shock"],
        "countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert_eq!(
        names(&game, &game.state.player(P0).graveyard.to_vec()),
        vec!["Counterspell"]
    );
}

#[test]
fn counterspell_needs_a_spell_target() {
    let mut game = counter_fixture();
    let counter = hand_card(&game, P0, CardId::Counterspell);
    // Empty stack: no legal spell target, so Counterspell is not offered.
    assert!(
        game.legal_actions()
            .iter()
            .all(|a| !matches!(&a.action, Action::CastSpell { card, .. } if *card == counter))
    );

    // After P1 casts, exactly one legal target exists.
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
    submit_ok(&mut game, P1, Action::Pass);
    let offered: Vec<String> = game
        .legal_actions()
        .into_iter()
        .filter(|a| a.label.starts_with("Cast Counterspell"))
        .map(|a| a.label)
        .collect();
    assert_eq!(offered.len(), 1, "{offered:?}");
    assert!(offered[0].contains("Shock"), "{offered:?}");
}

#[test]
fn a_spell_whose_targets_died_does_not_resolve() {
    // CR 608.2b: the second Shock finds its target already gone.
    let mut game = fixture(31)
        .hand(P0, &[CardId::Shock, CardId::Shock])
        .battlefield(P0, &[CardId::Mountain, CardId::Mountain])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P1, CardId::GrizzlyBears);

    // Shock #1 targets the 2/2 Bear.
    let first = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: first,
            targets: vec![Target::Permanent(bear)],
        },
    );
    // The caster keeps priority (CR 117.3c), so a response can be cast before
    // the first spell resolves.
    let second = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: second,
            targets: vec![Target::Permanent(bear)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    // Top Shock resolves: 2 damage to a 2/2 is lethal, so the Bear dies before
    // the first Shock resolves.
    assert_eq!(game.state.turn.priority, Some(P0));
    assert!(
        game.state.player(P1).graveyard.len() == 1,
        "the Bear is dead from state-based actions"
    );

    let events = submit_ok(&mut game, P0, Action::Pass);
    let events2 = submit_ok(&mut game, P1, Action::Pass);
    let all: Vec<&Event> = events.iter().chain(events2.iter()).collect();
    assert!(
        all.iter()
            .any(|e| matches!(e, Event::NoLegalTargets { .. })),
        "the second Shock reports illegal targets: {all:?}"
    );
    assert_eq!(game.state.player(P1).life, 20, "no damage went anywhere");
    assert_eq!(game.state.stack.len(), 0);
    assert_eq!(
        game.state.player(P0).graveyard.len(),
        2,
        "both Shocks are gone"
    );
}

#[test]
fn counterspell_resolution_revalidates_its_target() {
    // If the target spell is no longer on the stack when Counterspell resolves,
    // Counterspell does nothing (CR 608.2b) and the state stays consistent.
    let mut game = counter_fixture();
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
    let shock_object = game.state.stack.last().unwrap().object;
    submit_ok(&mut game, P1, Action::Pass);
    let counter = hand_card(&game, P0, CardId::Counterspell);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: counter,
            targets: vec![Target::Spell(shock_object)],
        },
    );
    // Simulate another effect removing the Shock from the stack first.
    game.state.stack.retain(|item| item.object != shock_object);

    let events = submit_ok(&mut game, P0, Action::Pass);
    let events2 = submit_ok(&mut game, P1, Action::Pass);
    let all: Vec<&Event> = events.iter().chain(events2.iter()).collect();
    assert!(
        !all.iter().any(|e| matches!(e, Event::Countered { .. })),
        "nothing was countered: {all:?}"
    );
    assert!(
        all.iter()
            .any(|e| matches!(e, Event::NoLegalTargets { .. })),
        "Counterspell reported illegal targets: {all:?}"
    );
    assert_eq!(game.state.player(P0).life, 20);
    assert_eq!(
        names(&game, &game.state.player(P0).graveyard.to_vec()),
        vec!["Counterspell"]
    );
    assert!(
        game.state.player(P1).graveyard.is_empty(),
        "the removed Shock was not put into a graveyard by the engine"
    );
    assert_eq!(game.state.stack.len(), 0);
}
