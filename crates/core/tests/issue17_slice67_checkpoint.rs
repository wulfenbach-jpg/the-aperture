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
    let save = SessionSave::capture(&g, replay, log, true);
    let loaded = SessionSave::from_json(&save.to_json());
    eprintln!(
        "REVIEW367 actual SessionSave load={:?}",
        loaded.as_ref().err()
    );
    assert!(
        loaded.is_ok(),
        "a legal post-activation state must load: {:?}",
        loaded.err()
    );
    assert!(g.state.validate().is_ok());
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
        .battlefield(P1, &[CardId::ArcaneSignet])
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
    assert!(g.state.validate().is_ok());
    let action = Action::ActivateCostedAbility {
        source: trash,
        ability: 0,
        mode: 0,
        cost_cards: vec![trash],
        targets: vec![Target::Permanent(artifact)],
    };
    assert!(g.legal_actions().iter().any(|x| x.action == action));
    let events = submit_ok(&mut g, P0, action);
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
}
