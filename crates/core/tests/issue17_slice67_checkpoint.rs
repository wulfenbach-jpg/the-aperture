//! Owned R367-1/2 regressions, adapted from reviewer probes.
mod common;
use aperture_core::*;
use common::*;

fn base(cards: &[CardId]) -> Game {
    fixture(36701)
        .battlefield(P0, cards)
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(
            P0,
            ManaPool {
                red: 10,
                colorless: 10,
                ..Default::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build()
}
fn activate(
    g: &mut Game,
    source: ObjectId,
    ability: u8,
    sacrifice: ObjectId,
    targets: Vec<Target>,
) -> Vec<Event> {
    let action = Action::ActivateCostedAbility {
        source,
        ability,
        mode: 0,
        cost_cards: vec![sacrifice],
        targets,
    };
    assert!(
        g.legal_actions().iter().any(|x| x.action == action),
        "probe action must be offered"
    );
    submit_ok(g, P0, action)
}
fn drain(g: &mut Game, log: &mut Vec<Event>) {
    for _ in 0..512 {
        if g.state.pending.is_none()
            && g.state.stack.is_empty()
            && g.state.trigger_queue.is_empty()
            && g.state.resolving.is_none()
        {
            return;
        }
        let p = g.decision_owner().expect("decision");
        let action = if matches!(g.state.pending, Some(Pending::TriggerTarget { .. })) {
            g.legal_actions()
                .into_iter()
                .map(|x| x.action)
                .find(|x| {
                    matches!(
                        x,
                        Action::ChooseTriggerTarget {
                            target: Target::Player(P1)
                        }
                    )
                })
                .expect("P1 target")
        } else {
            auto_answer(g)
        };
        log.extend(submit_ok(g, p, action));
    }
    panic!("did not drain");
}
#[test]
fn a_real_offered_sacrifice_with_pending_target_produces_valid_saveable_state() {
    let mut g = base(&[CardId::PashalikMons, CardId::GoblinToken]);
    assert!(
        g.state.validate().is_ok(),
        "initial fixture structurally valid"
    );
    let mons = battlefield_card(&g, P0, CardId::PashalikMons);
    let token = battlefield_card(&g, P0, CardId::GoblinToken);
    let log = activate(&mut g, mons, 1, token, vec![]);
    assert!(matches!(
        g.state.pending,
        Some(Pending::TriggerTarget { .. })
    ));
    eprintln!(
        "REVIEW367 pending={:?} priority={:?} validation={:?} events={:?}",
        g.state.pending,
        g.state.turn.priority,
        g.state.validate(),
        log
    );
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .unwrap();
    let save = SessionSave::capture(&g, replay, log.clone(), true);
    let loaded = SessionSave::from_json(&save.to_json());
    eprintln!(
        "REVIEW367 actual SessionSave load={:?}",
        loaded.as_ref().err()
    );
    let loaded = loaded.expect("a legal post-activation state must load");
    assert_eq!(
        g.state.turn.priority, None,
        "no priority while the trigger target is pending"
    );
    assert_eq!(loaded.game().state.pending, g.state.pending);
    let mut loaded = loaded.game();
    let mut original_log = log;
    drain_pair(&mut g, &mut loaded, &mut original_log);
    assert_eq!(
        g.state, loaded.state,
        "loaded continuation matches original state"
    );
    assert_eq!(g.state.revision, loaded.state.revision);
    assert_eq!(g.digest(), loaded.digest());
    assert_eq!(g.state.player(P1).life, loaded.state.player(P1).life);
}

/// Drain original and loaded copies with the same offered answers.
fn drain_pair(a: &mut Game, b: &mut Game, log: &mut Vec<Event>) {
    for _ in 0..512 {
        let a_done = a.state.pending.is_none()
            && a.state.stack.is_empty()
            && a.state.trigger_queue.is_empty()
            && a.state.resolving.is_none();
        if a_done {
            assert_eq!(a.state, b.state, "both continuations drained identically");
            return;
        }
        let owner = a.decision_owner().expect("decision owner");
        let action = if matches!(a.state.pending, Some(Pending::TriggerTarget { .. })) {
            a.legal_actions()
                .into_iter()
                .map(|x| x.action)
                .find(|x| {
                    matches!(
                        x,
                        Action::ChooseTriggerTarget {
                            target: Target::Player(P1)
                        }
                    )
                })
                .expect("P1 target offered")
        } else {
            auto_answer(a)
        };
        assert!(
            b.legal_actions().iter().any(|x| x.action == action),
            "the loaded game offers the same action"
        );
        log.extend(submit_ok(a, owner, action.clone()));
        submit_ok(b, owner, action);
    }
    panic!("pair did not drain");
}
#[test]
fn b_sacrifice_source_itself_pings_exactly_once_and_makes_two_tokens() {
    let mut g = base(&[CardId::PashalikMons]);
    let mons = battlefield_card(&g, P0, CardId::PashalikMons);
    let mut log = activate(&mut g, mons, 1, mons, vec![]);
    drain(&mut g, &mut log);
    assert_eq!(g.state.player(P1).life, 19);
    assert_eq!(
        g.state
            .battlefield(P0)
            .iter()
            .filter(|x| x.card == CardId::GoblinToken)
            .count(),
        2
    );
    assert_eq!(
        log.iter()
            .filter(|x| matches!(
                x,
                Event::TriggerStaged {
                    card: CardId::PashalikMons,
                    ..
                }
            ))
            .count(),
        1
    );
}
#[test]
fn c_batched_pashalik_deaths_no_double_count() {
    let mut g = base(&[
        CardId::PashalikMons,
        CardId::GoblinToken,
        CardId::GoblinToken,
    ]);
    for o in g
        .state
        .objects
        .values_mut()
        .filter(|x| x.zone == Zone::Battlefield)
    {
        o.damage = 9;
    }
    let mut log = submit_ok(&mut g, P0, Action::Pass);
    drain(&mut g, &mut log);
    assert_eq!(g.state.player(P1).life, 17);
    assert_eq!(
        log.iter()
            .filter(|x| matches!(
                x,
                Event::TriggerStaged {
                    card: CardId::PashalikMons,
                    ..
                }
            ))
            .count(),
        3
    );
}
#[test]
fn d_trashmaster_self_cost_destroys_artifact_and_keeps_enchantment() {
    let mut g = fixture(36704)
        .battlefield(P0, &[CardId::GoblinTrashmaster])
        .battlefield(P1, &[CardId::ArcaneSignet, CardId::WordsOfWilding])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let trash = battlefield_card(&g, P0, CardId::GoblinTrashmaster);
    let artifact = battlefield_card(&g, P1, CardId::ArcaneSignet);
    let enchantment = battlefield_card(&g, P1, CardId::WordsOfWilding);
    let mut log = activate(&mut g, trash, 0, trash, vec![Target::Permanent(artifact)]);
    drain(&mut g, &mut log);
    assert_eq!(g.state.object(trash).zone, Zone::Graveyard);
    assert_eq!(g.state.object(artifact).zone, Zone::Graveyard);
    assert_eq!(g.state.object(enchantment).zone, Zone::Battlefield);
}

#[test]
fn sacrifice_anthem_runs_sbas_before_choosing_cost_trigger_targets() {
    let mut g = fixture(36705)
        .battlefield(
            P0,
            &[
                CardId::PashalikMons,
                CardId::GoblinTrashmaster,
                CardId::GoblinToken,
            ],
        )
        .battlefield(P1, &[CardId::ArcaneSignet, CardId::WordsOfWilding])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    // Recompute the standing anthem before marking nonlethal damage.
    let mut log = submit_ok(&mut g, P0, Action::Pass);
    run_until(&mut g, &mut log, |x| x.decision_owner() == Some(P0), 64);
    let token = battlefield_card(&g, P0, CardId::GoblinToken);
    assert_eq!(g.state.object(token).toughness(), 2);
    object_mut(&mut g.state, token).damage = 1;
    let trash = battlefield_card(&g, P0, CardId::GoblinTrashmaster);
    let artifact = battlefield_card(&g, P1, CardId::ArcaneSignet);
    let enchantment = battlefield_card(&g, P1, CardId::WordsOfWilding);
    assert!(g.state.validate().is_ok());
    let action = Action::ActivateCostedAbility {
        source: trash,
        ability: 0,
        mode: 0,
        cost_cards: vec![trash],
        targets: vec![Target::Permanent(artifact)],
    };
    assert!(g.legal_actions().iter().any(|x| x.action == action));
    let mut events = submit_ok(&mut g, P0, action);
    eprintln!(
        "REVIEW367 SBA token={:?}, pending={:?}, offers={:?}, events={:?}",
        g.state.try_object(token),
        g.state.pending,
        g.legal_actions(),
        events
    );
    assert!(
        g.state
            .try_object(token)
            .is_none_or(|x| x.zone != Zone::Battlefield),
        "a 1/1 with 1 damage must die BEFORE any cost-trigger target is chosen (CR117.5)"
    );
    assert_eq!(
        g.state.turn.priority, None,
        "no priority while target choice is open"
    );
    assert!(
        g.state.validate().is_ok(),
        "post-cost pending state validates"
    );
    assert!(
        !g.legal_actions()
            .iter()
            .any(|x| matches!(x.action, Action::ChooseTriggerTarget { target: Target::Permanent(id) } if id == token)),
        "doomed token is absent from the whole offered target set"
    );
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .unwrap();
    let save = SessionSave::capture(&g, replay, events.clone(), true);
    assert!(
        SessionSave::from_json(&save.to_json()).is_ok(),
        "post-cost pending state survives a real sandbox SessionSave load"
    );
    assert_eq!(
        g.state.stack.first().map(|item| item.entry),
        Some(StackEntry::Activated { ability: 0 }),
        "the paid ability is below the waiting cost triggers"
    );
    let choose = g
        .legal_actions()
        .into_iter()
        .map(|x| x.action)
        .find(|x| {
            matches!(
                x,
                Action::ChooseTriggerTarget {
                    target: Target::Player(P1)
                }
            )
        })
        .expect("P1 target offered");
    events.extend(submit_ok(&mut g, P0, choose));
    drain(&mut g, &mut events);
    assert_eq!(
        events
            .iter()
            .filter(|x| matches!(
                x,
                Event::TriggerStaged {
                    card: CardId::PashalikMons,
                    ..
                }
            ))
            .count(),
        2,
        "Trashmaster and the anthem-loss token each trigger Pashalik once"
    );
    assert_eq!(g.state.object(artifact).zone, Zone::Graveyard);
    assert_eq!(
        g.state.object(enchantment).zone,
        Zone::Battlefield,
        "the enchantment control is untouched"
    );
}

#[test]
fn nonactive_activator_gets_priority_back_without_a_spurious_active_round() {
    let mut g = fixture(36706)
        .battlefield(P0, &[CardId::GoblinTrashmaster])
        .battlefield(P1, &[CardId::ArcaneSignet])
        .library(P0, &[CardId::Mountain; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(
            P0,
            ManaPool {
                red: 2,
                colorless: 2,
                ..Default::default()
            },
        )
        .main(P1, Step::PrecombatMain)
        .build();
    submit_ok(&mut g, P1, Action::Pass);
    assert_eq!(g.state.turn.priority, Some(P0));
    let trash = battlefield_card(&g, P0, CardId::GoblinTrashmaster);
    let artifact = battlefield_card(&g, P1, CardId::ArcaneSignet);
    let action = Action::ActivateCostedAbility {
        source: trash,
        ability: 0,
        mode: 0,
        cost_cards: vec![trash],
        targets: vec![Target::Permanent(artifact)],
    };
    assert!(
        g.legal_actions().iter().any(|x| x.action == action),
        "activation offered to non-active seat"
    );
    let events = submit_ok(&mut g, P0, action);
    assert_eq!(g.state.turn.active, P1, "P1 remains the active player");
    assert_eq!(
        g.state.turn.priority,
        Some(P0),
        "CR 117.3c returns priority to the activator P0"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e,Event::Priority{player} if *player==P1)),
        "no spurious active-player priority"
    );
}
