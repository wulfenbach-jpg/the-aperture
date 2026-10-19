//! Bounded combat: declarations, tapping, simultaneous damage, state-based
//! actions, and summoning sickness. CR 302.6, 508.1a/f, 509.1a, 510.1, 510.2.

mod common;

use aperture_core::*;
use common::*;

#[test]
fn unblocked_attacker_damages_the_defending_player() {
    let mut game = fixture(40)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let events = submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::AttackersDeclared { .. }))
    );
    assert!(
        game.state.object(bear).tapped,
        "attacking taps the creature (CR 508.1f)"
    );
    assert_eq!(game.state.turn.attackers, vec![bear]);

    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P1).life == 18,
        64
    ));
    assert_eq!(game.state.player(P0).life, 20, "no damage back");
    assert!(log.iter().any(|e| matches!(
        e,
        Event::CombatDamage {
            amount: 2,
            blocker: None,
            ..
        }
    )));
}

#[test]
fn summoning_sick_creature_cannot_attack() {
    // CR 302.6, 508.1a
    let mut game = fixture(41)
        .battlefield_sick(P0, &[CardId::CentaurCourser])
        .main(P0, Step::PrecombatMain)
        .build();
    assert!(game.legal_attackers(P0).is_empty());
    let coursers = battlefield_card(&game, P0, CardId::CentaurCourser);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_none(),
        64
    ));
    let err = submit_err(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![coursers],
            defender: Some(P1),
        },
    );
    assert!(matches!(err, EngineError::Illegal { .. }), "{err}");
    assert!(!game.state.object(coursers).tapped);
    assert_eq!(game.state.player(P1).life, 20);
}

#[test]
fn blocking_creatures_trade_and_lethal_damage_is_a_state_based_action() {
    // 3/3 blocked by a 2/2: the blocker dies, the attacker survives with damage.
    let mut game = fixture(42)
        .battlefield(P0, &[CardId::CentaurCourser])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let courser = battlefield_card(&game, P0, CardId::CentaurCourser);
    let bears = battlefield_card(&game, P1, CardId::GrizzlyBears);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![courser],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.pending.is_some() && g.state.turn.step == Step::DeclareBlockers,
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: bears,
                attacker: courser,
            }],
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P1).graveyard.len() == 1,
        64
    ));
    assert_eq!(game.state.battlefield(P1).len(), 0, "blocker died");
    let attacker = game.state.object(courser);
    assert_eq!(attacker.zone, Zone::Battlefield, "attacker survived");
    assert_eq!(attacker.damage, 2, "2 damage marked");
    assert_eq!(
        game.state.player(P1).life,
        20,
        "blocked damage does not hit the player"
    );
    assert!(log.iter().any(|e| matches!(
        e,
        Event::StateBasedAction { description } if description.contains("704.5")
    )));
}

#[test]
fn two_blockers_split_damage_lethal_first_and_all_die() {
    // CR 510.1c: 6 damage divided lethal-first among two 2/2 blockers.
    let mut game = fixture(43)
        .battlefield(P0, &[CardId::CrawWurm])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::RuneclawBear])
        .main(P0, Step::PrecombatMain)
        .build();
    let wurm = battlefield_card(&game, P0, CardId::CrawWurm);
    let bear1 = battlefield_card(&game, P1, CardId::GrizzlyBears);
    let bear2 = battlefield_card(&game, P1, CardId::RuneclawBear);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![wurm],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.pending.is_some() && g.state.turn.step == Step::DeclareBlockers,
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![
                BlockDecl {
                    blocker: bear1,
                    attacker: wurm,
                },
                BlockDecl {
                    blocker: bear2,
                    attacker: wurm,
                },
            ],
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P1).graveyard.len() == 2,
        64
    ));
    assert_eq!(game.state.battlefield(P0).len(), 0, "the Wurm also dies");
    assert_eq!(game.state.battlefield(P1).len(), 0);
    assert_eq!(game.state.player(P0).graveyard.len(), 1);
    assert_eq!(game.state.player(P1).life, 20);
}

#[test]
fn a_blocker_can_only_block_one_attacker_and_must_be_untapped() {
    let mut game = fixture(44)
        .battlefield(P0, &[CardId::GrizzlyBears, CardId::CentaurCourser])
        .battlefield(P1, &[CardId::RuneclawBear, CardId::HillGiant])
        .main(P0, Step::PrecombatMain)
        .build();
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let courser = battlefield_card(&game, P0, CardId::CentaurCourser);
    let blocker = battlefield_card(&game, P1, CardId::RuneclawBear);
    let giant = battlefield_card(&game, P1, CardId::HillGiant);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![bears, courser],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.pending.is_some() && g.state.turn.step == Step::DeclareBlockers,
        64
    ));
    // Same blocker twice is illegal (CR 509.1a).
    let err = submit_err(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![
                BlockDecl {
                    blocker,
                    attacker: bears,
                },
                BlockDecl {
                    blocker,
                    attacker: courser,
                },
            ],
        },
    );
    assert!(err.reason().contains("one attacker"), "{err}");
    // A tapped creature cannot block.
    object_mut(&mut game.state, giant).tapped = true;
    let err = submit_err(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: giant,
                attacker: bears,
            }],
        },
    );
    assert!(err.reason().contains("untapped"), "{err}");
    // A creature that is not attacking cannot be blocked.
    let err = submit_err(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker,
                attacker: giant,
            }],
        },
    );
    assert!(err.reason().contains("not attacking"), "{err}");
    // The legal declaration still works.
    let events = submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker,
                attacker: bears,
            }],
        },
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::BlockersDeclared { .. }))
    );
}

#[test]
fn attackers_are_declared_only_during_the_declare_attackers_step() {
    let mut game = fixture(45)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let err = submit_err(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    assert!(matches!(err, EngineError::Illegal { .. }), "{err}");
}

#[test]
fn combat_damage_happens_once() {
    let mut game = fixture(46)
        .battlefield(P0, &[CardId::HillGiant])
        .main(P0, Step::PrecombatMain)
        .build();
    let giant = battlefield_card(&game, P0, CardId::HillGiant);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![giant],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::EndCombat || g.state.turn.step == Step::PostcombatMain,
        64
    ));
    assert_eq!(game.state.player(P1).life, 17, "exactly one 3-damage hit");
}

#[test]
fn giant_growth_pumps_until_end_of_turn() {
    // CR 613.4c, 514.2
    let mut game = fixture(47)
        .hand(P0, &[CardId::GiantGrowth])
        .battlefield(P0, &[CardId::Forest, CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let growth = hand_card(&game, P0, CardId::GiantGrowth);
    let mine = battlefield_card(&game, P0, CardId::GrizzlyBears);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: growth,
            targets: vec![Target::Permanent(mine)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.object(mine).power(), 5);
    assert_eq!(game.state.object(mine).toughness(), 5);

    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.number == 2 && g.state.turn.step == Step::Upkeep,
        256
    ));
    assert_eq!(
        game.state.object(mine).power(),
        2,
        "the pump ended in the cleanup step"
    );
    assert_eq!(game.state.object(mine).pump_power, 0);
}
