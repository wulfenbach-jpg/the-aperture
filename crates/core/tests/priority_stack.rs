//! Increment 1: priority scheduling and one-item-per-two-passes resolution.
//! CR 117.3a/117.3b/117.3c/117.3d/117.4/117.5, 502.4, 608.2b.

mod common;

use aperture_core::*;
use common::*;

fn shock_fixture() -> Game {
    fixture(11)
        .hand(P0, &[CardId::Shock])
        .hand(P1, &[CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .battlefield(P1, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build()
}

#[test]
fn caster_receives_priority_after_casting() {
    // CR 117.3c
    let mut game = shock_fixture();
    let shock = hand_card(&game, P0, CardId::Shock);
    let events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P1)],
        },
    );
    assert_eq!(game.decision_owner(), Some(P0), "caster keeps priority");
    assert_eq!(game.state.stack.len(), 1);
    assert_eq!(game.state.player(P1).life, 20, "nothing resolves yet");
    assert!(matches!(events.first().unwrap(), Event::Cast { .. }));
}

#[test]
fn single_pass_does_not_resolve_the_stack() {
    // CR 117.3d: one pass moves priority, it does not resolve.
    let mut game = shock_fixture();
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P1)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.decision_owner(), Some(P1));
    assert_eq!(game.state.stack.len(), 1);
    assert_eq!(game.state.player(P1).life, 20);
    assert_eq!(game.state.turn.passed, 1);
}

#[test]
fn two_passes_resolve_exactly_one_stack_item() {
    // CR 117.4 — the requested regression test for first-passed one-item stack.
    let mut game = shock_fixture();
    let shock0 = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock0,
            targets: vec![Target::Player(P1)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    let shock1 = hand_card(&game, P1, CardId::Shock);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: shock1,
            targets: vec![Target::Player(P0)],
        },
    );
    assert_eq!(game.state.stack.len(), 2);
    assert_eq!(
        game.decision_owner(),
        Some(P1),
        "P1 cast, so P1 keeps priority"
    );
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.stack.len(), 2, "only one pass so far");

    // The second consecutive pass resolves only the top item.
    let events = submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.stack.len(), 1, "exactly one item resolved");
    assert_eq!(game.state.player(P0).life, 18, "P1's Shock resolved");
    assert_eq!(
        game.state.player(P1).life,
        20,
        "P0's Shock still on the stack"
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Resolve { .. }))
            .count(),
        1
    );
    assert_eq!(
        game.decision_owner(),
        Some(P0),
        "active player has priority (CR 117.3b)"
    );

    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.stack.len(), 0);
    assert_eq!(game.state.player(P1).life, 18);
}

#[test]
fn active_player_receives_priority_after_resolution() {
    // CR 117.3b
    let mut game = shock_fixture();
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P1)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.player(P1).life, 18);
    assert_eq!(game.state.stack.len(), 0);
    assert_eq!(game.state.turn.active, P0);
    assert_eq!(game.decision_owner(), Some(P0));
}

#[test]
fn no_priority_is_granted_during_the_untap_step() {
    // CR 502.4
    let mut game = fixture(3)
        .hand(P0, &[CardId::Forest])
        .main(P0, Step::Untap)
        .step_no_priority(P0, Step::Untap)
        .build();
    assert_eq!(game.decision_owner(), None);
    assert!(game.legal_actions().is_empty());
    let err = submit_err(&mut game, P0, Action::Pass);
    assert_eq!(err, EngineError::NoDecisionPoint);
}

#[test]
fn two_passes_with_an_empty_stack_end_the_step() {
    // CR 117.4 second clause.
    let mut game = fixture(5).main(P0, Step::PrecombatMain).build();
    assert_eq!(game.state.turn.step, Step::PrecombatMain);
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.turn.step, Step::BeginCombat);
    assert_eq!(game.state.turn.priority, Some(P0));
}

#[test]
fn untap_step_untaps_only_the_active_players_permanents() {
    // CR 502.3 — played through the real scheduler: end P0's turn and check
    // that P1's untap step touches only P1's permanents.
    let mut game = fixture(9)
        .battlefield(P0, &[CardId::Forest])
        .battlefield(P1, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let p0_forest = battlefield_card(&game, P0, CardId::Forest);
    let p1_forest = battlefield_card(&game, P1, CardId::Forest);
    object_mut(&mut game.state, p0_forest).tapped = true;
    object_mut(&mut game.state, p1_forest).tapped = true;
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.number == 2 && g.state.turn.step == Step::Upkeep,
        128
    ));
    assert!(!game.state.object(p1_forest).tapped, "P1 untaps");
    assert!(game.state.object(p0_forest).tapped, "P0 does not untap");
}
