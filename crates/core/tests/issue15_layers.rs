//! Issue 15, part two: layered continuous effects (CR 613) in a real game.
//!
//! `issue15_effects.rs` covers replacement (CR 614) and prevention (CR 615)
//! plus the CR 616.1 ordering rule; this file covers the layer order for
//! characteristic-defining power/toughness, until-end-of-turn expiry, and the
//! representative end-to-end journey with its replay.

mod common;

use aperture_core::*;
use common::*;

fn green(n: u8) -> ManaPool {
    ManaPool {
        green: n,
        ..Default::default()
    }
}

/// Two consecutive passes resolve the item on top of the stack (CR 117.4) and
/// return the events emitted while it resolved.
fn resolve_cast(game: &mut Game, caster: PlayerId, other: PlayerId) -> Vec<Event> {
    let mut events = submit_ok(game, caster, Action::Pass);
    events.extend(submit_ok(game, other, Action::Pass));
    events
}

/// Wight of Precinct Six is a 1/1 whose printed ability is an additive layer
/// 7c static ("gets +1/+1 for each creature card in your opponents'
/// graveyards"), so with no creature cards in any graveyard it is still a 1/1
/// and does not die.
#[test]
fn printed_wight_is_a_one_one_with_empty_graveyards() {
    let mut game = fixture(1504)
        .battlefield(P0, &[CardId::WightOfPrecinctSix])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let wight = battlefield_card(&game, P0, CardId::WightOfPrecinctSix);
    assert_eq!(game.state.object(wight).power(), 1);
    assert_eq!(game.state.object(wight).toughness(), 1);
    let events = submit_ok(&mut game, P0, Action::Pass);
    assert!(
        !events.iter().any(
            |event| matches!(event, Event::Destroyed { card, .. } if *card == CardId::WightOfPrecinctSix)
        ),
        "a printed 1/1 does not die to state-based actions: {events:?}"
    );
    assert_eq!(game.state.object(wight).zone, Zone::Battlefield);
}

/// Only creature cards in *opponents'* graveyards count: two of them make the
/// 1/1 a 3/3, and a creature card in its controller's own graveyard adds
/// nothing (CR 613.4c).
#[test]
fn printed_wight_counts_only_opponents_creature_cards() {
    let mut game = fixture(1505)
        .battlefield(P0, &[CardId::WightOfPrecinctSix, CardId::GrizzlyBears])
        .add(P0, Zone::Graveyard, CardId::RuneclawBear)
        .add(P0, Zone::Graveyard, CardId::CentaurCourser)
        .add(P1, Zone::Graveyard, CardId::RuneclawBear)
        .add(P1, Zone::Graveyard, CardId::CentaurCourser)
        .hand(P0, &[CardId::Forest, CardId::GiantGrowth])
        .mana(P0, green(1))
        .main(P0, Step::PrecombatMain)
        .build();
    let wight = battlefield_card(&game, P0, CardId::WightOfPrecinctSix);
    // Playing a land recomputes the layers from the current game state.
    let forest = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: forest });
    assert_eq!(
        game.state.object(wight).power(),
        3,
        "1/1 plus two opponent creature cards is a 3/3; own graveyard ignored"
    );
    assert_eq!(game.state.object(wight).toughness(), 3);

    // A 7c until-end-of-turn modifier is added to the same layer's static.
    let growth = hand_card(&game, P0, CardId::GiantGrowth);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: growth,
            targets: vec![Target::Permanent(wight)],
        },
    );
    let events = resolve_cast(&mut game, P0, P1);
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::ContinuousEffectStarted { layer, .. } if layer == "7c"
        )),
        "the modifier is logged in its layer: {events:?}"
    );
    assert_eq!(game.state.object(wight).power(), 6, "3/3 plus +3/+3");
    assert_eq!(game.state.object(wight).toughness(), 6);
}

/// Until-end-of-turn modifiers and prevention shields both end in cleanup
/// (CR 514.2), and the layer recomputation reflects it.
#[test]
fn until_end_of_turn_effects_end_in_cleanup() {
    let mut game = fixture(1506)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .hand(P0, &[CardId::GiantGrowth, CardId::MendingHands])
        .mana(
            P0,
            ManaPool {
                white: 1,
                green: 1,
                ..Default::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let growth = hand_card(&game, P0, CardId::GiantGrowth);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: growth,
            targets: vec![Target::Permanent(bears)],
        },
    );
    resolve_cast(&mut game, P0, P1);
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
    assert_eq!(game.state.object(bears).power(), 5);
    assert_eq!(game.state.effects.len(), 1);
    assert_eq!(game.state.shields.len(), 1);

    let mut log: Vec<Event> = Vec::new();
    for _ in 0..64 {
        if game.state.is_over()
            || log.iter().any(|event| {
                matches!(
                    event,
                    Event::StepBegin {
                        step: Step::Cleanup
                    }
                )
            })
        {
            break;
        }
        let Some(owner) = game.decision_owner() else {
            break;
        };
        let action = auto_answer(&game);
        log.extend(submit_ok(&mut game, owner, action));
    }
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::StepBegin {
                step: Step::Cleanup
            }
        )),
        "the game reached cleanup"
    );
    assert!(
        game.state.effects.is_empty(),
        "the +3/+3 until end of turn ended (CR 514.2)"
    );
    assert!(
        game.state.shields.is_empty(),
        "the prevention shield ended (CR 514.2)"
    );
    assert_eq!(game.state.object(bears).power(), 2);
    assert_eq!(game.state.object(bears).toughness(), 2);
}

// -------------------------------------------------------------- real journey

const JOURNEY_A: &str = "\
10 Forest
5 Plains
2 Mountain
4 Grizzly Bears
4 Wight of Precinct Six
3 Angrath's Marauders
4 Giant Growth
4 Mending Hands
4 Runeclaw Bear
";

const JOURNEY_B: &str = "\
17 Mountain
4 Hill Giant
4 Centaur Courser
4 Shock
4 Lightning Strike
3 Craw Wurm
4 Runeclaw Bear
";

fn journey_deck(name: &str, text: &str) -> DeckList {
    DeckList::parse_practice(name, text).expect("journey deck parses")
}

/// Deterministic casting policy: answer forced choices with the shared driver,
/// attack with everything offered, then cast, play or pass.
fn journey_action(game: &Game) -> Action {
    let legal = game.legal_actions();
    if let Some(Pending::DeclareAttackers { .. }) = &game.state.pending {
        if let Some(attack) = legal.iter().find(|legal| {
            matches!(&legal.action, Action::DeclareAttackers { attackers, .. } if !attackers.is_empty())
        }) {
            return attack.action.clone();
        }
        return auto_answer(game);
    }
    if game.state.pending.is_some() {
        return auto_answer(game);
    }
    // Prefer a spell aimed at the other seat: the shield then sits on the
    // defending player and the very next attack is a real prevention instance.
    let actor = game.decision_owner();
    for legal_action in &legal {
        if let Action::CastSpell { targets, .. } = &legal_action.action
            && !targets.is_empty()
            // Only in the combat steps after attackers are known: the shield
            // then meets the attack it was cast for in the same turn, instead
            // of expiring in an earlier cleanup.
            && matches!(
                game.state.turn.step,
                Step::DeclareBlockers | Step::CombatDamage
            )
            && !targets
                .iter()
                .any(|target| matches!(target, Target::Player(p) if Some(*p) == actor))
        {
            return legal_action.action.clone();
        }
    }
    for legal_action in &legal {
        // A prevention shield is only worth casting once attackers are known.
        let held_shield = matches!(
            &legal_action.action,
            Action::CastSpell { card, .. }
                if game.state.object(*card).card == CardId::MendingHands
                    && !matches!(
                        game.state.turn.step,
                        Step::DeclareBlockers | Step::CombatDamage
                    )
        );
        if matches!(legal_action.action, Action::CastSpell { .. }) && !held_shield {
            return legal_action.action.clone();
        }
    }
    for legal_action in &legal {
        if matches!(
            legal_action.action,
            Action::ActivateAbility { .. } | Action::PlayLand { .. }
        ) {
            return legal_action.action.clone();
        }
    }
    Action::Pass
}

#[test]
fn a_real_game_logs_the_new_effects_and_replays_to_the_same_digest() {
    // Chosen deterministically: this seed's policy run casts a shield, meets it
    // with a real attacker and casts a modifier.
    let seed = 15_005u64;
    let decks = vec![
        journey_deck("Issue 15 shields", JOURNEY_A),
        journey_deck("Issue 15 pressure", JOURNEY_B),
    ];
    let (mut game, mut log) = Game::new_logged(seed, &decks).expect("game");
    let mut replay = Replay::new(
        seed,
        game.state.first_player,
        decks.clone(),
        AiSettings::default(),
    )
    .expect("replay");
    let mut steps = Vec::new();
    for _ in 0..900 {
        if game.state.is_over() {
            break;
        }
        let Some(owner) = game.decision_owner() else {
            break;
        };
        let action = journey_action(&game);
        let step = ReplayStep {
            player: owner,
            revision: game.revision(),
            decision: game.decision(),
            action: action.clone(),
        };
        match game.submit_action(owner, action) {
            Ok(events) => {
                log.extend(events);
                replay.record(step.clone());
                steps.push(step);
            }
            Err(error) => panic!("journey step rejected: {error}"),
        }
    }
    assert!(!steps.is_empty(), "the journey actually played");

    let created = log
        .iter()
        .filter(|event| matches!(event, Event::DamageShieldCreated { .. }))
        .count();
    let _prevented = log
        .iter()
        .filter(|event| matches!(event, Event::DamagePrevented { .. }))
        .count();
    let modifiers = log
        .iter()
        .filter(|event| matches!(event, Event::ContinuousEffectStarted { .. }))
        .count();
    assert!(
        created > 0,
        "the real game creates a prevention shield: {:?}",
        log.iter()
            .filter(|event| matches!(event, Event::DamageShieldCreated { .. }))
            .collect::<Vec<_>>()
    );
    assert!(
        _prevented > 0,
        "the shield meets real damage in the journey: {:?}",
        log.iter()
            .filter(|event| matches!(
                event,
                Event::DamageShieldCreated { .. } | Event::DamagePrevented { .. }
            ))
            .collect::<Vec<_>>()
    );
    assert!(modifiers > 0, "a layered continuous effect actually starts");

    // The rules trace renders the new events for players, citing the rule.
    let rendered: Vec<String> = log
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::DamageShieldCreated { .. }
                    | Event::DamagePrevented { .. }
                    | Event::ContinuousEffectStarted { .. }
            )
        })
        .map(|event| aperture_core::view::render_event(&game, P0, event))
        .collect();
    assert!(
        rendered.iter().all(|line| line.contains("CR 61")),
        "each new event cites its rule: {rendered:?}"
    );

    let (replayed, replayed_log) = replay.run().expect("the recorded journey replays");
    assert_eq!(
        replayed.digest(),
        game.digest(),
        "the replay reproduces the exact final state"
    );
    assert_eq!(replayed_log, log, "the replay reproduces the event trace");
}
