//! Regression tests for the findings of the independent rules review
//! (see HANDOFF.md "Independent review"): combat after a creature leaves the
//! battlefield, private scry/surveil masking, the replay seed in the log, the
//! coaching information boundary, the first-player draw skip, the mulligan
//! deadlock, and mana abilities during a pending choice.
//! Rules cited: CR 103.5, 103.8a, 506.4, 509.1h, 605.3a, 701.22a, 704.5.

mod common;

use aperture_core::*;
use common::*;

/// Drive to the declare-attackers step and declare `attackers`.
fn declare_attack(game: &mut Game, log: &mut Vec<Event>, attackers: &[ObjectId]) {
    assert!(run_until(
        game,
        log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        game,
        game.decision_owner().unwrap(),
        Action::DeclareAttackers {
            attackers: attackers.to_vec(),
            defender: Some(P1),
        },
    );
}

#[test]
fn a_destroyed_attacker_assigns_no_combat_damage() {
    // CR 506.4: a creature that leaves the battlefield is removed from combat.
    let mut game = fixture(70)
        .battlefield(P0, &[CardId::HillGiant])
        .hand(P1, &[CardId::LightningStrike])
        .battlefield(P1, &[CardId::Mountain, CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let giant = battlefield_card(&game, P0, CardId::HillGiant);
    let mut log = Vec::new();
    declare_attack(&mut game, &mut log, &[giant]);
    // P0 holds priority in the declare attackers step (CR 508.2); pass it, then
    // P1 kills the attacker before combat damage.
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.decision_owner(), Some(P1));
    let strike = hand_card(&game, P1, CardId::LightningStrike);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Permanent(giant)],
        },
    );
    submit_ok(&mut game, P1, Action::Pass);
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(
        game.state.object(giant).zone,
        Zone::Graveyard,
        "the attacker died"
    );
    assert!(game.state.turn.attackers.is_empty(), "it left combat");

    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(
            g.state.turn.step,
            Step::PostcombatMain | Step::EndStep | Step::Cleanup
        ),
        128
    ));
    assert_eq!(
        game.state.player(P1).life,
        20,
        "no damage from a dead attacker"
    );
    assert!(
        !log.iter()
            .any(|event| matches!(event, Event::CombatDamage { blocker: None, .. })),
        "no unblocked combat damage was reported"
    );
}

#[test]
fn a_blocker_that_died_absorbs_nothing_and_the_attacker_stays_blocked() {
    // CR 509.1h: the attacker remains blocked even when its blocker leaves.
    let mut game = fixture(71)
        .battlefield(P0, &[CardId::GrizzlyBears, CardId::Mountain])
        .hand(P0, &[CardId::Shock])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let attacker = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let blocker = battlefield_card(&game, P1, CardId::GrizzlyBears);
    let mut log = Vec::new();
    declare_attack(&mut game, &mut log, &[attacker]);
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl { blocker, attacker }],
        },
    );
    // P0 (active player, CR 509.2) kills the blocker before damage.
    assert_eq!(game.decision_owner(), Some(P0));
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Permanent(blocker)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.object(blocker).zone, Zone::Graveyard);
    assert!(
        game.state.turn.blockers.is_empty(),
        "the dead blocker is out of combat"
    );

    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(
            g.state.turn.step,
            Step::PostcombatMain | Step::EndStep | Step::Cleanup
        ),
        128
    ));
    assert_eq!(
        game.state.player(P1).life,
        20,
        "a blocked attacker hits nothing"
    );
    let attacker_object = game.state.object(attacker);
    assert_eq!(
        attacker_object.zone,
        Zone::Battlefield,
        "the attacker survived"
    );
    assert_eq!(attacker_object.damage, 0, "and took no damage");
}

#[test]
fn a_dead_attacker_cannot_be_blocked() {
    let mut game = fixture(72)
        .battlefield(P0, &[CardId::HillGiant])
        .hand(P1, &[CardId::LightningStrike])
        .battlefield(
            P1,
            &[CardId::Mountain, CardId::Mountain, CardId::RuneclawBear],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let giant = battlefield_card(&game, P0, CardId::HillGiant);
    let blocker = battlefield_card(&game, P1, CardId::RuneclawBear);
    let mut log = Vec::new();
    declare_attack(&mut game, &mut log, &[giant]);
    submit_ok(&mut game, P0, Action::Pass);
    let strike = hand_card(&game, P1, CardId::LightningStrike);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Permanent(giant)],
        },
    );
    submit_ok(&mut game, P1, Action::Pass);
    submit_ok(&mut game, P0, Action::Pass);
    assert!(game.state.turn.attackers.is_empty());
    // P1 still has an untapped creature, but the step never asks for blockers.
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.turn.step, Step::DeclareBlockers);
    assert!(
        game.state.pending.is_none(),
        "no blocker declaration is pending when nothing is attacking"
    );
    assert!(
        game.legal_actions()
            .iter()
            .all(|a| !matches!(a.action, Action::DeclareBlockers { .. })),
        "a block against a dead attacker is never offered"
    );
    let err = submit_err(
        &mut game,
        P0,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker,
                attacker: giant,
            }],
        },
    );
    assert!(matches!(err, EngineError::Illegal { .. }), "{err}");
    let before = game.digest();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.turn.step, Step::PostcombatMain | Step::EndStep),
        128
    ));
    assert_eq!(game.state.player(P1).life, 20);
    assert_ne!(game.digest(), before, "the game progressed normally");
}

#[test]
fn the_opponent_does_not_learn_a_private_scry_or_surveil_choice() {
    // CR 701.22a / 701.25a: looking at the top cards is not public information.
    let mut game = fixture(73)
        .hand(P0, &[CardId::Opt])
        .battlefield(P0, &[CardId::Island])
        .library(P0, &[CardId::Forest, CardId::Shock])
        .main(P0, Step::PrecombatMain)
        .build();
    let opt = hand_card(&game, P0, CardId::Opt);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: vec![],
        },
    );
    log.extend(submit_ok(&mut game, P0, Action::Pass));
    log.extend(submit_ok(&mut game, P1, Action::Pass));
    log.extend(submit_ok(
        &mut game,
        P0,
        Action::OrderCard { to_top: false },
    ));

    let mine = game.view_for(P0, &log);
    assert!(
        mine.log.iter().any(|line| line.contains("Forest")),
        "the owner sees their own scry choice: {:?}",
        mine.log
    );
    let theirs = game.view_for(P1, &log);
    assert!(
        !theirs.to_json().contains("Forest"),
        "the other seat must not learn the looked-at card: {:?}",
        theirs.log
    );

    // A surveilled card that goes to the graveyard is public (CR 404.2).
    let mut game = fixture(74)
        .hand(P0, &[CardId::Consider])
        .battlefield(P0, &[CardId::Island])
        .library(P0, &[CardId::Mountain, CardId::Shock])
        .main(P0, Step::PrecombatMain)
        .build();
    let consider = hand_card(&game, P0, CardId::Consider);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: consider,
            targets: vec![],
        },
    );
    log.extend(submit_ok(&mut game, P0, Action::Pass));
    log.extend(submit_ok(&mut game, P1, Action::Pass));
    log.extend(submit_ok(
        &mut game,
        P0,
        Action::OrderCard { to_top: false },
    ));
    let theirs = game.view_for(P1, &log);
    assert!(
        theirs.log.iter().any(|line| line.contains("Mountain")),
        "a surveilled card in a graveyard is public: {:?}",
        theirs.log
    );
}

#[test]
fn the_game_start_log_does_not_reveal_the_replay_seed() {
    // With public deck lists, the seed alone reconstructs both libraries.
    let (game, log) = Game::new_logged(987_654_321, &[deck_a(), deck_b()]).expect("game");
    for seat in [P0, P1] {
        let view = game.view_for(seat, &log);
        assert!(
            !view.to_json().contains("987654321"),
            "the seed must not appear in the projection"
        );
        assert!(
            view.log.iter().any(|line| line.contains("plays first")),
            "the first player is still announced"
        );
    }
}

#[test]
fn coaching_never_describes_another_seats_hand() {
    // Give P1 (who will hold priority) a card it cannot possibly cast.
    let mut game = fixture(75)
        .hand(P1, &[CardId::Divination])
        .battlefield(P1, &[])
        .main(P0, Step::PrecombatMain)
        .build();
    set_priority(&mut game.state, Some(P1));
    let legal = game.legal_actions();
    assert_eq!(game.decision_owner(), Some(P1));

    let from_p0 = coach::explain(&game, P0, &legal);
    assert!(
        !from_p0
            .iter()
            .any(|e| e.text.contains("Divination") || e.topic.contains("Divination")),
        "P0 must not be told about P1's hand: {from_p0:?}"
    );

    let from_p1 = coach::explain(&game, P1, &legal);
    assert!(
        from_p1
            .iter()
            .any(|e| e.text.contains("Divination") || e.topic.contains("Divination")),
        "the seat that owns the decision still gets its own analysis"
    );
}

#[test]
fn the_player_who_plays_first_skips_their_first_draw_step() {
    // CR 103.8a
    let (mut game, mut log) = Game::new_logged(76, &[deck_a(), deck_b()]).expect("game");
    run_until(&mut game, &mut log, |g| g.state.pending.is_none(), 32);
    let first = game.state.first_player;
    let other = game
        .state
        .next_live_seat_after(first)
        .expect("a second live seat");
    assert_eq!(game.state.player(first).hand_size(), 7);
    assert_eq!(game.state.player(first).library.len(), 33);

    // Count only draws that happen after turn 1 is announced; the opening deal
    // also emits Draw events, before the game starts.
    let turn_one = log
        .iter()
        .position(|event| matches!(event, Event::TurnBegin { turn: 1, .. }))
        .expect("turn 1 is announced");
    let seen_draws = log[turn_one..]
        .iter()
        .filter(|event| matches!(event, Event::Draw { player, .. } if *player == first))
        .count();
    assert_eq!(seen_draws, 0, "the first player drew on turn 1");

    // The other player does draw in their own first draw step.
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.active == other && g.state.turn.step == Step::PrecombatMain,
        256
    ));
    assert!(
        log.iter()
            .any(|event| matches!(event, Event::Draw { player, .. } if *player == other)),
        "the second player draws normally"
    );
}

#[test]
fn a_mulligan_that_could_never_be_bottomed_is_refused() {
    // CR 103.5: the London mulligan bottoms one card per mulligan taken.
    let (mut game, mut log) = Game::new_logged(77, &[deck_a(), deck_b()]).expect("game");
    for _ in 0..7 {
        assert!(matches!(
            game.state.pending,
            Some(Pending::Mulligan { player: P0 })
        ));
        submit_ok(&mut game, P0, Action::Mulligan { keep: false });
    }
    assert_eq!(game.state.player(P0).mulligans, 7);
    assert_eq!(game.state.player(P0).hand_size(), 7);

    let offered = game.legal_actions();
    assert_eq!(
        offered.len(),
        1,
        "only keeping is possible now: {offered:?}"
    );
    let before = game.digest();
    let decision = game.decision();
    let err = submit_err(&mut game, P0, Action::Mulligan { keep: false });
    assert!(matches!(err, EngineError::Illegal { .. }), "{err}");
    assert_eq!(game.digest(), before, "a refused mulligan changes nothing");
    assert_eq!(game.decision(), decision);

    // Keeping completes: seven cards must go to the bottom, and then the game
    // continues instead of deadlocking.
    submit_ok(&mut game, P0, Action::Mulligan { keep: true });
    let cards = match &game.state.pending {
        Some(Pending::BottomCards { count, .. }) => {
            assert_eq!(*count, 7);
            game.state.player(P0).hand.clone()
        }
        other => panic!("expected a bottoming decision, got {other:?}"),
    };
    submit_ok(&mut game, P0, Action::BottomCards { cards });
    assert_eq!(game.state.player(P0).hand_size(), 0);
    assert!(matches!(
        game.state.pending,
        Some(Pending::Mulligan { player: P1 })
    ));
    submit_ok(&mut game, P1, Action::Mulligan { keep: true });
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.priority.is_some() || g.state.is_over(),
        128
    ));
    assert!(!game.state.is_over());
}

#[test]
fn a_mana_ability_is_refused_while_a_choice_is_pending() {
    // CR 605.3a: a mana ability needs priority (or a payment in progress).
    let mut game = fixture(78)
        .hand(P0, &[CardId::Opt])
        .battlefield(P0, &[CardId::Island, CardId::Island])
        .library(P0, &[CardId::Forest, CardId::Shock])
        .main(P0, Step::PrecombatMain)
        .build();
    let opt = hand_card(&game, P0, CardId::Opt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert!(matches!(
        game.state.pending,
        Some(Pending::OrderCard { .. })
    ));
    let untapped = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| !o.tapped && o.card == CardId::Island)
        .map(|o| o.id)
        .expect("one Island is still untapped");

    let before = game.digest();
    let revision = game.revision();
    let decision = game.decision();
    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateManaAbility { land: untapped },
    );
    assert!(matches!(err, EngineError::Illegal { .. }), "{err}");
    assert_eq!(game.digest(), before);
    assert_eq!(game.revision(), revision);
    assert_eq!(game.decision(), decision);
    assert!(!game.state.object(untapped).tapped);
    assert!(game.state.player(P0).mana.is_empty());
    assert!(
        game.legal_actions()
            .iter()
            .all(|a| !matches!(a.action, Action::ActivateManaAbility { .. })),
        "no mana ability is offered during a pending choice"
    );
}
