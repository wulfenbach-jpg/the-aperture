//! State-based actions and turn-based actions at the edges: losing by an empty
//! library draw or by life total, cleanup discard, cleanup damage removal.
//! CR 104.3c, 117.5, 514.1, 514.2, 704.5a, 704.5b, 704.5g.

mod common;

use aperture_core::*;
use common::*;

#[test]
fn drawing_from_an_empty_library_loses_the_game() {
    // CR 104.3c, 704.5b
    let mut game = fixture(50)
        .hand(P0, &[CardId::Divination])
        .battlefield(P0, &[CardId::Island, CardId::Island, CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let div = hand_card(&game, P0, CardId::Divination);
    let events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: div,
            targets: vec![],
        },
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::Cast { .. })),
        "Divination is cast"
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    match &game.state.outcome {
        Some(Outcome::Win { winner, reason }) => {
            assert_eq!(*winner, P1);
            assert!(reason.contains("704.5b"), "{reason}");
        }
        other => panic!("expected a loss for P0, got {other:?}"),
    }
    assert!(game.state.player(P0).drew_from_empty);
    assert!(
        game.legal_actions().is_empty(),
        "no actions after the game ends"
    );
}

#[test]
fn reaching_zero_life_loses_the_game() {
    // CR 704.5a
    let mut game = fixture(51)
        .hand(P1, &[CardId::Shock])
        .battlefield(P1, &[CardId::Mountain])
        .life(P0, 1)
        .main(P0, Step::PrecombatMain)
        .build();
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
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.player(P0).life, -1);
    match &game.state.outcome {
        Some(Outcome::Win { winner, reason }) => {
            assert_eq!(*winner, P1);
            assert!(reason.contains("704.5a"), "{reason}");
        }
        other => panic!("expected P1 to win, got {other:?}"),
    }
}

#[test]
fn cleanup_discards_down_to_seven_cards() {
    // CR 514.1
    let mut game = fixture(52)
        .hand(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Shock,
                CardId::Shock,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    assert_eq!(game.state.player(P0).hand_size(), 9);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::Discard { count: 2, .. })),
        64
    ));
    assert_eq!(
        game.state.turn.priority, None,
        "no priority while a cleanup choice is pending"
    );
    let before = game.state.player(P0).hand.clone();
    let chosen: Vec<ObjectId> = before.iter().rev().take(2).rev().copied().collect();
    let events = submit_ok(
        &mut game,
        P0,
        Action::Discard {
            cards: chosen.clone(),
        },
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Discarded { .. }))
            .count(),
        2
    );
    assert_eq!(game.state.player(P0).hand_size(), 7);
    assert_eq!(game.state.player(P0).graveyard.len(), 2);
    assert!(
        game.state
            .player(P0)
            .graveyard
            .iter()
            .all(|id| chosen.contains(id)),
        "exactly the chosen cards were discarded"
    );
    // The turn continues after cleanup.
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.number == 2 && g.state.turn.step == Step::Upkeep,
        64
    ));
}

#[test]
fn cleanup_removes_damage_and_ends_until_end_of_turn_effects() {
    // CR 514.2
    let mut game = fixture(53)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    {
        let obj = object_mut(&mut game.state, bear);
        obj.damage = 1;
        obj.pump_power = 3;
        obj.pump_toughness = 3;
    }
    assert_eq!(game.state.object(bear).toughness(), 5);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.number == 2 && g.state.turn.step == Step::Upkeep,
        128
    ));
    let obj = game.state.object(bear);
    assert_eq!(obj.damage, 0, "damage removed in cleanup");
    assert_eq!(obj.pump_power, 0);
    assert_eq!(obj.toughness(), 2);
}

#[test]
fn a_creature_with_zero_toughness_dies_to_state_based_actions() {
    // CR 704.5f
    let mut game = fixture(54)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    object_mut(&mut game.state, bear).pump_toughness = -2;
    // Issue 15: the premise is a typed layer-7c modifier, so the layer
    // recomputation keeps it instead of clearing a bare field.
    game.state.effects.push(ContinuousEffect {
        id: game.state.next_effect,
        timestamp: 1,
        source: None,
        card: CardId::GrizzlyBears,
        controller: P0,
        layer: Layer::Modifying,
        target: bear,
        kind: ContinuousKind::ModifyPowerToughness {
            power: 0,
            toughness: -2,
        },
        duration: EffectDuration::UntilEndOfTurn,
    });
    game.state.next_effect += 1;
    // Any action triggers the state-based-action check (CR 117.5).
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.battlefield(P0).len(), 0);
    assert_eq!(game.state.player(P0).graveyard.len(), 1);
}

#[test]
fn lethal_damage_from_a_noncombat_spell_kills_before_priority() {
    // Shock a 2/2: the creature dies during the SBA check that precedes the
    // next priority round (CR 117.5, 704.5g).
    let mut game = fixture(55)
        .hand(P0, &[CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .battlefield(P1, &[CardId::RuneclawBear])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P1, CardId::RuneclawBear);
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Permanent(bear)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.battlefield(P1).len(), 0);
    assert_eq!(game.state.player(P1).graveyard.len(), 1);
    assert_eq!(
        game.state.turn.priority,
        Some(P0),
        "priority continues after SBAs"
    );
}

#[test]
fn a_player_who_lost_cannot_act() {
    let mut game = fixture(56)
        .hand(P1, &[CardId::Shock])
        .battlefield(P1, &[CardId::Mountain])
        .life(P0, 2)
        .main(P0, Step::PrecombatMain)
        .build();
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
    submit_ok(&mut game, P0, Action::Pass);
    assert!(game.state.is_over());
    assert!(
        game.legal_actions().is_empty(),
        "no actions after the game ends"
    );
    let err = submit_err(&mut game, P0, Action::Pass);
    assert_eq!(err, EngineError::GameOver);
    let err = submit_err(&mut game, P1, Action::Pass);
    assert_eq!(err, EngineError::GameOver);
}
