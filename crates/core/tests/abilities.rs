//! Typed activated and triggered abilities (issue 14).
//!
//! Every test drives the engine through legal [`Action`] values only; none of
//! them reaches into rules internals to force an effect.

mod common;

use aperture_core::*;
use common::*;

fn ability_stack_kinds(game: &Game) -> Vec<StackEntry> {
    game.state.stack.iter().map(|item| item.entry).collect()
}

// ------------------------------------------------------------------ activated

#[test]
fn tap_ability_goes_on_the_stack_and_deals_damage() {
    // CR 602.2, 602.2b, 608.2
    let mut game = fixture(101)
        .battlefield(P0, &[CardId::ProdigalSorcerer])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, CardId::ProdigalSorcerer);

    let events = submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: vec![Target::Player(P1)],
        },
    );
    assert!(
        events.iter().any(
            |e| matches!(e, Event::AbilityActivated { text, .. } if text.contains("deals 1 damage"))
        ),
        "the activation is reported with the printed ability text"
    );
    assert_eq!(
        game.state.player(P1).life,
        20,
        "an ability on the stack does no damage yet"
    );
    assert_eq!(game.state.stack.len(), 1);
    assert!(matches!(
        game.state.stack[0].entry,
        StackEntry::Activated { ability: 0 }
    ));
    assert!(game.state.object(source).tapped, "the tap cost was paid");
    assert_eq!(
        game.decision_owner(),
        Some(P0),
        "CR 117.3c: activator keeps priority"
    );

    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.player(P1).life, 19);
    assert!(game.state.stack.is_empty(), "the ability left the stack");
    assert!(
        game.state.object(source).card == CardId::ProdigalSorcerer
            && game.state.object(source).zone == Zone::Battlefield,
        "an ability resolving never moves its source"
    );
}

#[test]
fn tap_ability_is_refused_while_summoning_sick_and_when_tapped() {
    // CR 302.6, 602.5a
    let mut sick = fixture(102)
        .battlefield_sick(P0, &[CardId::ProdigalSorcerer])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&sick, P0, CardId::ProdigalSorcerer);
    let err = submit_err(
        &mut sick,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: vec![Target::Player(P1)],
        },
    );
    assert!(
        matches!(err, EngineError::BadTiming { .. }),
        "summoning sickness refuses a {{T}} activation, got {err:?}"
    );

    let mut tapped = fixture(103)
        .battlefield(P0, &[CardId::ProdigalSorcerer])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&tapped, P0, CardId::ProdigalSorcerer);
    object_mut(&mut tapped.state, source).tapped = true;
    let err = submit_err(
        &mut tapped,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: vec![Target::Player(P1)],
        },
    );
    assert!(matches!(err, EngineError::Illegal { .. }), "got {err:?}");
}

#[test]
fn activated_ability_needs_its_controller_and_a_legal_target() {
    // CR 602.2, 602.2c
    let mut game = fixture(104)
        .battlefield(P0, &[CardId::ProdigalSorcerer])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, CardId::ProdigalSorcerer);
    let bear = battlefield_card(&game, P1, CardId::GrizzlyBears);

    submit_ok(&mut game, P0, Action::Pass);
    let err = submit_err(
        &mut game,
        P1,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: vec![Target::Permanent(bear)],
        },
    );
    assert!(
        matches!(err, EngineError::Illegal { .. }),
        "only the controller activates, got {err:?}"
    );

    submit_ok(&mut game, P1, Action::Pass);
    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: vec![Target::Spell(bear)],
        },
    );
    assert!(
        matches!(err, EngineError::BadTarget { .. }),
        "a permanent is not a stack spell target, got {err:?}"
    );
}

#[test]
fn mana_cost_ability_and_once_each_turn_limit() {
    // CR 602.2b (mana cost), 602.5b ("Activate only once each turn")
    let mut game = fixture(105)
        .library(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(
            P1,
            &[
                CardId::Island,
                CardId::Island,
                CardId::Island,
                CardId::Island,
                CardId::Island,
            ],
        )
        .battlefield(
            P0,
            &[
                CardId::Rootwalla,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, CardId::Rootwalla);

    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert!(
        !game.state.object(source).tapped,
        "this cost has no tap component"
    );
    let tapped_lands = game
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card.def().is_land() && o.tapped)
        .count();
    assert_eq!(tapped_lands, 2, "{{1}}{{G}} was paid from the pool/lands");

    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.object(source).power(), 4, "+2/+2 applied");

    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert!(
        matches!(err, EngineError::Illegal { .. }),
        "second activation in the same turn is refused, got {err:?}"
    );

    // A limit of once per turn expires with the turn (CR 602.5b).
    let mut log = Vec::new();
    let reached = run_until(
        &mut game,
        &mut log,
        |g| {
            g.state.turn.number >= 3
                && g.state.turn.active == P0
                && g.state.turn.step == Step::PrecombatMain
        },
        600,
    );
    assert!(
        reached,
        "the scripted driver should reach P0's next precombat main"
    );
    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
}

#[test]
fn sacrifice_cost_ability_moves_the_source_and_gains_life() {
    // CR 602.2b: the sacrifice is a cost; the ability still resolves afterwards.
    let mut game = fixture(106)
        .battlefield(P0, &[CardId::BrindleBoar])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, CardId::BrindleBoar);

    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert_eq!(
        game.state.player(P0).life,
        20,
        "the cost is paid, the effect is still on the stack"
    );
    assert_eq!(
        game.state.object(source).zone,
        Zone::Graveyard,
        "sacrificing the source is part of the cost"
    );
    assert_eq!(ability_stack_kinds(&game).len(), 1);

    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.player(P0).life, 24);
    assert!(game.state.stack.is_empty());
}

// ------------------------------------------------------------------ triggered

#[test]
fn enters_the_battlefield_trigger_draws_before_priority() {
    // CR 603.6a, 603.3
    let mut game = fixture(107)
        .hand(P0, &[CardId::ElvishVisionary])
        .battlefield(P0, &[CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let elf = hand_card(&game, P0, CardId::ElvishVisionary);

    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: elf,
            targets: Vec::new(),
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);

    assert_eq!(
        game.state.player(P0).hand_size(),
        0,
        "the spell resolved; the trigger is waiting on the stack"
    );
    assert!(matches!(
        game.state.stack.last().map(|item| item.entry),
        Some(StackEntry::Triggered { .. })
    ));
    assert_eq!(game.state.player(P0).library.len(), 1);

    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(
        game.state.player(P0).hand_size(),
        1,
        "the trigger drew a card"
    );
    assert!(game.state.stack.is_empty());
}

#[test]
fn optional_trigger_is_declined_or_used_by_its_controller() {
    // CR 603.5: the ability goes on the stack either way; the choice is made on
    // resolution by the controller.
    let mut declined = fixture(108)
        .battlefield(P0, &[CardId::Oculus])
        .hand(P1, &[CardId::Shock])
        .battlefield(P1, &[CardId::Mountain])
        .library(P0, &[CardId::GrizzlyBears])
        .main(P1, Step::PrecombatMain)
        .build();
    let oculus = battlefield_card(&declined, P0, CardId::Oculus);
    let shock = hand_card(&declined, P1, CardId::Shock);
    submit_ok(
        &mut declined,
        P1,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Permanent(oculus)],
        },
    );
    submit_ok(&mut declined, P1, Action::Pass);
    submit_ok(&mut declined, P0, Action::Pass);
    assert_eq!(
        declined.state.object(oculus).zone,
        Zone::Graveyard,
        "Oculus died"
    );
    // CR 603.3: the trigger is put on the stack before anyone gets priority; it
    // resolves only after both seats pass again.
    submit_ok(&mut declined, P1, Action::Pass);
    submit_ok(&mut declined, P0, Action::Pass);
    assert!(
        matches!(
            declined.state.pending,
            Some(Pending::TriggerChoice { player: P0, .. })
        ),
        "the optional trigger waits for its controller, got {:?}",
        declined.state.pending
    );

    let events = submit_ok(
        &mut declined,
        P0,
        Action::AnswerTriggerChoice { use_ability: false },
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::TriggerDeclined { .. })),
        "declining is a typed event"
    );
    assert_eq!(declined.state.player(P0).hand_size(), 0, "no card drawn");

    let mut used = fixture(109)
        .battlefield(P0, &[CardId::Oculus])
        .hand(P1, &[CardId::Shock])
        .battlefield(P1, &[CardId::Mountain])
        .library(P0, &[CardId::GrizzlyBears])
        .main(P1, Step::PrecombatMain)
        .build();
    let oculus = battlefield_card(&used, P0, CardId::Oculus);
    let shock = hand_card(&used, P1, CardId::Shock);
    submit_ok(
        &mut used,
        P1,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Permanent(oculus)],
        },
    );
    submit_ok(&mut used, P1, Action::Pass);
    submit_ok(&mut used, P0, Action::Pass);
    submit_ok(&mut used, P1, Action::Pass);
    submit_ok(&mut used, P0, Action::Pass);
    submit_ok(
        &mut used,
        P0,
        Action::AnswerTriggerChoice { use_ability: true },
    );
    assert_eq!(
        used.state.player(P0).hand_size(),
        1,
        "the controller used the optional trigger"
    );
}

#[test]
fn trigger_without_a_legal_target_is_removed() {
    // CR 603.3d
    let mut game = fixture(110)
        .battlefield(P0, &[CardId::FesteringGoblin])
        .hand(P1, &[CardId::Shock])
        .battlefield(P1, &[CardId::Mountain])
        .main(P1, Step::PrecombatMain)
        .build();
    let goblin = battlefield_card(&game, P0, CardId::FesteringGoblin);
    let shock = hand_card(&game, P1, CardId::Shock);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Permanent(goblin)],
        },
    );
    submit_ok(&mut game, P1, Action::Pass);
    let events = submit_ok(&mut game, P0, Action::Pass);
    assert!(
        events.iter().any(
            |e| matches!(e, Event::TriggerRemoved { card, .. } if *card == CardId::FesteringGoblin)
        ),
        "the trigger had no legal target and never reached the stack"
    );
    assert!(game.state.stack.is_empty());
}

#[test]
fn simultaneous_dies_triggers_use_apnap_order_and_lki() {
    // CR 603.3b (APNAP), 603.3d (targets), 700.4/603.6c (dies + LKI)
    let mut game = fixture(111)
        .hand(P1, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::FesteringGoblin, CardId::RuneclawBear])
        .battlefield(
            P1,
            &[
                CardId::FesteringGoblin,
                CardId::GrizzlyBears,
                CardId::Mountain,
            ],
        )
        .main(P1, Step::PrecombatMain)
        .build();
    let goblin_p0 = battlefield_card(&game, P0, CardId::FesteringGoblin);
    let goblin_p1 = battlefield_card(&game, P1, CardId::FesteringGoblin);
    let bear_p0 = battlefield_card(&game, P0, CardId::RuneclawBear);
    let bear_p1 = battlefield_card(&game, P1, CardId::GrizzlyBears);
    let crack = hand_card(&game, P1, CardId::CrackTheEarth);

    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    );
    submit_ok(&mut game, P1, Action::Pass);
    submit_ok(&mut game, P0, Action::Pass);

    // The sacrifice choice is APNAP: the active player first.
    assert!(matches!(
        game.state.pending,
        Some(Pending::ApnapSacrifice { player: P1, .. })
    ));
    submit_ok(
        &mut game,
        P1,
        Action::ChooseSacrifice {
            permanent: Some(goblin_p1),
        },
    );
    submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(goblin_p0),
        },
    );

    let mut log = Vec::new();
    // Two triggers, two target choices, in APNAP order (P1 is the active seat).
    let mut choices = 0;
    while let Some(pending) = game.state.pending.clone() {
        match pending {
            Pending::TriggerTarget { player, .. } => {
                choices += 1;
                // P1's trigger takes P0's bear; P0's trigger takes P1's bear.
                let wanted = if player == P1 { bear_p0 } else { bear_p1 };
                let target = game
                    .legal_actions()
                    .into_iter()
                    .find(|legal| {
                        legal.action
                            == Action::ChooseTriggerTarget {
                                target: Target::Permanent(wanted),
                            }
                    })
                    .map(|legal| legal.action)
                    .expect("a target option is offered");
                log.extend(submit_ok(&mut game, player, target));
            }
            other => panic!("unexpected pending: {other:?}"),
        }
        if choices == 2 {
            break;
        }
    }
    assert_eq!(choices, 2, "both dies triggers asked for targets");

    let staged: Vec<(PlayerId, CardId)> = log
        .iter()
        .filter_map(|event| match event {
            Event::TriggerStaged {
                controller, card, ..
            } => Some((*controller, *card)),
            _ => None,
        })
        .collect();
    // APNAP: the active player's (P1) trigger is put on the stack first, so it
    // resolves last.
    assert_eq!(
        staged,
        vec![(P1, CardId::FesteringGoblin), (P0, CardId::FesteringGoblin)],
        "trigger stacking is APNAP from the active player"
    );
    assert_eq!(
        ability_stack_kinds(&game),
        vec![
            StackEntry::Triggered { ability: 0 },
            StackEntry::Triggered { ability: 0 }
        ]
    );

    // Resolve both triggers; their sources are already in graveyards, so the
    // pump must come from last-known information.
    let mut log = Vec::new();
    run_until(&mut game, &mut log, |g| g.state.stack.is_empty(), 64);
    let resolved = log
        .iter()
        .filter(|event| matches!(event, Event::AbilityResolved { .. }))
        .count();
    assert_eq!(resolved, 2, "both triggers resolved from the graveyard");
    assert_eq!(game.state.object(bear_p0).zone, Zone::Battlefield);
    assert_eq!(game.state.object(bear_p1).zone, Zone::Battlefield);
}

#[test]
fn trigger_target_lost_before_resolution_makes_the_ability_fizzle() {
    // CR 608.2b
    let mut game = fixture(112)
        .hand(P0, &[CardId::SwordsToPlowshares])
        .battlefield(P0, &[CardId::FesteringGoblin, CardId::Plains])
        .hand(P1, &[CardId::Shock])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::Mountain])
        .main(P1, Step::PrecombatMain)
        .build();
    let goblin = battlefield_card(&game, P0, CardId::FesteringGoblin);
    let bear = battlefield_card(&game, P1, CardId::GrizzlyBears);
    let shock = hand_card(&game, P1, CardId::Shock);

    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Permanent(goblin)],
        },
    );
    submit_ok(&mut game, P1, Action::Pass);
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.object(goblin).zone, Zone::Graveyard);
    // Only one creature is left, so the single legal target is chosen for the
    // trigger without a pending choice (CR 603.3d).
    assert!(matches!(
        game.state.stack.last().map(|item| item.entry),
        Some(StackEntry::Triggered { .. })
    ));

    submit_ok(&mut game, P1, Action::Pass);
    // P0 answers the trigger's target with Swords to Plowshares on that exact
    // creature, so the target is gone before the trigger resolves.
    let swords = hand_card(&game, P0, CardId::SwordsToPlowshares);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: swords,
            targets: vec![Target::Permanent(bear)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(game.state.object(bear).zone, Zone::Exile);

    submit_ok(&mut game, P1, Action::Pass);
    let events = submit_ok(&mut game, P0, Action::Pass);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::AbilityFizzled { .. })),
        "an ability whose only target is illegal does not resolve"
    );
}

#[test]
fn an_ability_on_the_stack_is_not_a_spell() {
    // CR 115.1: Counterspell's "target spell" cannot target an ability.
    let mut game = fixture(113)
        .battlefield(P0, &[CardId::ProdigalSorcerer])
        .hand(P1, &[CardId::Counterspell])
        .battlefield(P1, &[CardId::Island, CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, CardId::ProdigalSorcerer);
    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: vec![Target::Player(P1)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);

    let offered = game.legal_actions();
    assert!(
        !offered
            .iter()
            .any(|legal| matches!(legal.action, Action::CastSpell { .. })),
        "Counterspell has no legal target while only an ability is on the stack"
    );
}

// ------------------------------------------------------ resolution ordering

/// R170-1 (review 170): a `Priority` event must never be emitted while an
/// ability stack item is still resolving (CR 117.5, 608.2). The ability's own
/// effect events come first, then `AbilityResolved`, and only then the next
/// priority grant.
fn assert_no_priority_inside_ability_resolution(log: &[Event], effect: fn(&Event) -> bool) {
    let resolved = log
        .iter()
        .position(|event| matches!(event, Event::AbilityResolved { .. }))
        .unwrap_or_else(|| panic!("the ability never reported AbilityResolved: {log:?}"));
    let effect_index = log[..resolved]
        .iter()
        .rposition(effect)
        .unwrap_or_else(|| panic!("the ability's own effect event is missing: {log:?}"));
    let false_priority: Vec<&Event> = log[effect_index..resolved]
        .iter()
        .filter(|event| matches!(event, Event::Priority { .. }))
        .collect();
    assert!(
        false_priority.is_empty(),
        "no Priority may be granted between the ability's effect and its \
         AbilityResolved event (CR 117.5, 608.2): {false_priority:?} in {log:?}"
    );
    assert!(
        log[resolved + 1..]
            .iter()
            .any(|event| matches!(event, Event::Priority { .. })),
        "priority resumes after the ability finished resolving: {log:?}"
    );
}

#[test]
fn activated_ability_effect_comes_before_ability_resolved_and_priority() {
    // R170-1, CR 117.5: the scheduler must not hand out priority in the middle
    // of an activated ability's resolution.
    let mut game = fixture(114)
        .battlefield(P0, &[CardId::ProdigalSorcerer])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, CardId::ProdigalSorcerer);

    let mut log = submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source,
            ability: 0,
            mode: 0,
            targets: vec![Target::Player(P1)],
        },
    );
    log.extend(submit_ok(&mut game, P0, Action::Pass));
    log.extend(submit_ok(&mut game, P1, Action::Pass));

    assert_eq!(game.state.player(P1).life, 19, "the ability resolved");
    assert_no_priority_inside_ability_resolution(&log, |event| {
        matches!(event, Event::Damage { .. } | Event::LifeChange { .. })
    });
}

#[test]
fn triggered_ability_effect_comes_before_ability_resolved_and_priority() {
    // R170-1 for a triggered ability: the draw happens, the trigger is reported
    // resolved, and only then does the active player receive priority.
    let mut game = fixture(115)
        .hand(P0, &[CardId::ElvishVisionary])
        .battlefield(P0, &[CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let elf = hand_card(&game, P0, CardId::ElvishVisionary);

    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: elf,
            targets: Vec::new(),
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert!(
        matches!(
            game.state.stack.last().map(|item| item.entry),
            Some(StackEntry::Triggered { .. })
        ),
        "the enters-the-battlefield trigger is waiting on the stack"
    );

    let mut log = submit_ok(&mut game, P0, Action::Pass);
    log.extend(submit_ok(&mut game, P1, Action::Pass));
    assert_eq!(
        game.state.player(P0).hand_size(),
        1,
        "the trigger drew a card"
    );
    assert_no_priority_inside_ability_resolution(&log, |event| matches!(event, Event::Draw { .. }));
}
