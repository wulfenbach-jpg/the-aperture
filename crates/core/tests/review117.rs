mod common;

use aperture_core::*;

fn assert_round_trip(g: &Game, replay: &Replay, log: &[Event]) {
    let saved = SessionSave::capture(g, replay.clone(), log.to_vec(), false);
    assert_eq!(
        SessionSave::from_json(&saved.to_json()).unwrap().game,
        g.state
    );
    assert_eq!(replay.run().unwrap().0.state, g.state);
}

fn apply(g: &mut Game, r: &mut Replay, log: &mut Vec<Event>, a: Action) -> Vec<Event> {
    let p = g.decision_owner().unwrap();
    let step = ReplayStep {
        player: p,
        revision: g.revision(),
        decision: g.decision(),
        action: a.clone(),
    };
    let ev = g.submit_action(p, a).unwrap();
    r.record(step);
    r.final_digest = g.digest();
    log.extend(ev.clone());
    ev
}

fn legal_cleanup_pending() -> (SessionSave, PlayerId, u32) {
    let seed = 11701;
    let deck = DeckList::parse_practice(
        "legal cleanup",
        "10 Plains\n10 Forest\n8 Island\n6 Unsummon\n6 Giant Growth\n",
    )
    .unwrap();
    let decks = vec![deck.clone(), deck];
    let specs = [
        (0, CardId::IsamaruHoundOfKonda),
        (1, CardId::IsamaruHoundOfKonda),
    ];
    let (mut g, mut log) = Game::new_command_zone_logged(seed, &decks, &specs).unwrap();
    let mut r = Replay::new_command_zone(
        seed,
        g.state.first_player,
        decks,
        specs
            .iter()
            .map(|(seat, card)| CommanderSetup {
                seat: *seat,
                card: *card,
            })
            .collect(),
        AiSettings::default(),
    )
    .unwrap();
    let mut stage = 0;
    let mut owner = 0;
    for _ in 0..4000 {
        assert!(!g.state.is_over());
        let p = g.decision_owner().unwrap();
        let legal = g.legal_actions();
        if let Some(Pending::CommanderReturn { .. }) = &g.state.pending
            && stage == 3
        {
            assert_eq!(g.state.turn.step, Step::Cleanup);
            g.state.validate().unwrap();
            assert_eq!(r.run().unwrap().0.state, g.state);
            let save = SessionSave::capture(&g, r.clone(), log.clone(), false);
            SessionSave::from_json(&save.to_json()).expect("pending cleanup-return save loads");
            return (save, owner, g.state.turn.number);
        }
        let a = match &g.state.pending {
            Some(Pending::Mulligan { .. }) => Action::Mulligan { keep: true },
            Some(Pending::CommanderReplacement { .. }) if stage == 1 => {
                stage = 2;
                Action::AnswerCommanderReplacement {
                    to_command_zone: false,
                }
            }
            Some(Pending::Discard { count, .. }) if stage == 2 && p == owner => {
                let commander = g
                    .state
                    .commanders
                    .iter()
                    .find(|c| c.owner == owner)
                    .unwrap()
                    .object;
                if g.state.player(p).hand.contains(&commander)
                    && g.state
                        .player(p)
                        .hand
                        .iter()
                        .any(|id| g.state.object(*id).card == CardId::GiantGrowth)
                    && g.state
                        .battlefield(p)
                        .iter()
                        .any(|o| o.card == CardId::Forest && !o.tapped)
                    && g.state
                        .objects
                        .values()
                        .any(|o| o.zone == Zone::Battlefield && o.is_creature())
                {
                    let mut cards = vec![commander];
                    for id in &g.state.player(p).hand {
                        if cards.len() == *count as usize {
                            break;
                        }
                        if *id != commander && g.state.object(*id).card != CardId::GiantGrowth {
                            cards.push(*id);
                        }
                    }
                    if cards.len() != *count as usize {
                        panic!("could not choose a legal cleanup discard set");
                    }
                    stage = 3;
                    Action::Discard { cards }
                } else {
                    legal.first().unwrap().action.clone()
                }
            }
            Some(_) => legal.first().unwrap().action.clone(),
            None => {
                if stage <= 1 || (stage == 2 && p != owner) {
                    if stage == 0
                        && g.state
                            .commanders
                            .iter()
                            .all(|c| g.state.object(c.object).zone == Zone::Battlefield)
                        && g.state
                            .player(p)
                            .hand
                            .iter()
                            .any(|id| g.state.object(*id).card == CardId::GiantGrowth)
                        && g.state
                            .battlefield(p)
                            .iter()
                            .any(|o| o.card == CardId::Forest)
                    {
                        let c = g
                            .state
                            .commanders
                            .iter()
                            .find(|c| c.owner == p)
                            .unwrap()
                            .object;
                        if let Some(a) = legal.iter().find(|a| matches!(&a.action, Action::CastSpell { card, targets }
                            if g.state.object(*card).card == CardId::Unsummon && targets == &vec![Target::Permanent(c)]))
                        {
                            owner = p;
                            stage = 1;
                            a.action.clone()
                        } else {
                            legal.iter().find(|a| matches!(a.action, Action::PlayLand { .. })).map(|a| a.action.clone()).unwrap_or(Action::Pass)
                        }
                    } else if let Some(a) = legal
                        .iter()
                        .find(|a| matches!(a.action, Action::PlayLand { .. }))
                    {
                        a.action.clone()
                    } else if stage == 0 {
                        legal.iter().find(|a| matches!(a.action, Action::CastSpell { card, .. } if g.state.object(card).zone == Zone::Command))
                            .map(|a| a.action.clone()).unwrap_or(Action::Pass)
                    } else {
                        Action::Pass
                    }
                } else {
                    Action::Pass
                }
            }
        };
        apply(&mut g, &mut r, &mut log, a);
    }
    panic!("no legal cleanup journey reached");
}

fn cleanup_branch(accept: bool) {
    let (save, owner, turn) = legal_cleanup_pending();
    let mut g = save.game();
    let mut restored = SessionSave::from_json(&save.to_json()).unwrap().game();
    let mut replay = save.replay.clone();
    let mut log = save.log.clone();
    let answer = Action::AnswerCommanderReturn {
        to_command_zone: accept,
    };
    let direct = apply(&mut g, &mut replay, &mut log, answer.clone());
    let restored_events = restored.submit_action(owner, answer).unwrap();
    assert_eq!(restored_events, direct);
    assert_eq!(restored.state, g.state);
    assert_eq!(g.state.turn.step, Step::Cleanup);
    assert_eq!(g.state.turn.priority, Some(owner));
    assert_round_trip(&g, &replay, &log);
    // Float mana during the cleanup priority window through a real mana ability.
    let mana = g
        .legal_actions()
        .into_iter()
        .find(|a| {
            matches!(a.action, Action::ActivateManaAbility { land }
            if g.state.object(land).card != CardId::Forest)
        })
        .map(|a| a.action)
        .expect("the legal cleanup journey has unused non-green mana");
    {
        let ev = apply(&mut g, &mut replay, &mut log, mana.clone());
        let restored_ev = restored.submit_action(owner, mana).unwrap();
        assert_eq!(restored_ev, ev);
        assert_eq!(restored.state, g.state);
        assert!(g.state.player(g.state.turn.active).mana.total() > 0);
    }
    let growth = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::CastSpell { card, .. } if g.state.object(card).card == CardId::GiantGrowth))
        .expect("cleanup priority offers a legal Giant Growth")
        .action;
    let target = if let Action::CastSpell { targets, .. } = &growth {
        if let Some(Target::Permanent(id)) = targets.first() {
            *id
        } else {
            panic!()
        }
    } else {
        panic!()
    };
    let ev = apply(&mut g, &mut replay, &mut log, growth.clone());
    assert_eq!(restored.submit_action(owner, growth).unwrap(), ev);
    assert_eq!(restored.state, g.state);
    assert!(g.state.player(owner).mana.total() > 0);
    assert_round_trip(&g, &replay, &log);
    let mut saw_pump = false;
    let mut saw_next = false;
    let mut cleanup_repeats = 0;
    let mut partial_checked = false;
    for _ in 0..20 {
        if g.state.object(target).pump_power == 3 {
            if !saw_pump {
                assert_round_trip(&g, &replay, &log);
            }
            saw_pump = true;
        }
        let (p, action) = if g.state.pending.is_some() {
            let p = g.decision_owner().unwrap();
            (p, g.legal_actions().first().unwrap().action.clone())
        } else {
            let p = g.decision_owner().unwrap();
            if !partial_checked {
                partial_checked = true;
                // A partial pass round must not repeat cleanup or expire effects.
                let partial_events = apply(&mut g, &mut replay, &mut log, Action::Pass);
                let partial_restored = restored.submit_action(p, Action::Pass).unwrap();
                assert_eq!(partial_restored, partial_events);
                assert_eq!(restored.state, g.state);
                assert!(!partial_events.iter().any(|e| matches!(
                    e,
                    Event::StepBegin {
                        step: Step::Cleanup
                    }
                )));
                continue;
            }
            (p, Action::Pass)
        };
        let events = apply(&mut g, &mut replay, &mut log, action.clone());
        assert_eq!(restored.submit_action(p, action).unwrap(), events);
        assert_eq!(restored.state, g.state);
        cleanup_repeats += events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::StepBegin {
                        step: Step::Cleanup
                    }
                )
            })
            .count();
        if cleanup_repeats > 0 {
            assert_eq!(
                g.state.player(owner).mana.total(),
                0,
                "the repeated cleanup boundary empties floating mana"
            );
        }
        if g.state.turn.number != turn {
            saw_next = true;
            break;
        }
    }
    assert_eq!(
        cleanup_repeats, 1,
        "exactly one repeated StepBegin(Cleanup)"
    );
    assert_eq!(g.state.player(g.state.turn.active).mana.total(), 0);
    assert!(
        saw_pump,
        "the cleanup-window Giant Growth actually resolved"
    );
    assert!(
        saw_next,
        "the repeated cleanup must eventually complete the turn"
    );
    assert_eq!(
        g.state.object(target).pump_power,
        0,
        "until-end-of-turn effects must expire in the repeated cleanup before the next turn"
    );
    assert_eq!(g.state.object(target).pump_toughness, 0);
    assert_round_trip(&g, &replay, &log);
    assert_eq!(replay.run().unwrap().0.state, g.state);
    let saved = SessionSave::capture(&g, replay, log, false);
    SessionSave::from_json(&saved.to_json()).expect("post-cleanup save/replay identity");
}

#[test]
fn accepted_cleanup_return_repeats_cleanup_and_expires_growth() {
    cleanup_branch(true);
}

#[test]
fn declined_cleanup_return_repeats_cleanup_and_expires_growth() {
    cleanup_branch(false);
}

#[test]
fn ordinary_end_step_growth_expires_in_cleanup_without_cleanup_priority() {
    // Synthetic ordinary-step control, not a legal-journey persistence proof.
    let mut g = Game::new_logged(
        11702,
        &[
            DeckList::parse_practice("control", "40 Forest\n").unwrap(),
            DeckList::parse_practice("control", "40 Forest\n").unwrap(),
        ],
    )
    .unwrap()
    .0;
    g.state.mulligan = None;
    g.state.pending = None;
    let bear = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        bear,
        aperture_core::state::Object {
            id: bear,
            card: CardId::GrizzlyBears,
            owner: P0,
            controller: P0,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: aperture_core::counters::Counters::default(),
        },
    );
    let forest = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        forest,
        aperture_core::state::Object {
            id: forest,
            card: CardId::Forest,
            owner: P0,
            controller: P0,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: aperture_core::counters::Counters::default(),
        },
    );
    let growth = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        growth,
        aperture_core::state::Object {
            id: growth,
            card: CardId::GiantGrowth,
            owner: P0,
            controller: P0,
            zone: Zone::Hand,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: aperture_core::counters::Counters::default(),
        },
    );
    g.state.players[P0 as usize].hand.push(growth);
    g.state.turn.number = 5;
    g.state.turn.active = P0;
    g.state.turn.step = Step::EndStep;
    g.state.turn.priority = Some(P0);
    g.state.turn.passed = 0;
    g.submit_action(
        P0,
        Action::CastSpell {
            card: growth,
            targets: vec![Target::Permanent(bear)],
        },
    )
    .unwrap();
    g.submit_action(P0, Action::Pass).unwrap();
    g.submit_action(P1, Action::Pass).unwrap();
    assert_eq!(g.state.object(bear).pump_power, 3);
    for _ in 0..10 {
        if g.state.turn.number > 5 {
            break;
        }
        let p = g.decision_owner().unwrap();
        g.submit_action(p, Action::Pass).unwrap();
    }
    assert_eq!(g.state.object(bear).pump_power, 0);
    assert!(g.state.turn.number > 5);
}

#[test]
fn synthetic_repeated_cleanup_waits_for_discard_then_removes_damage_and_pump() {
    // Isolate a repeated-cleanup discard boundary; this is explicitly a
    // synthetic fixture, not an accepted-action journey or replay proof.
    let mut g = common::fixture(11803)
        .library(P0, &[CardId::Forest])
        .library(P1, &[CardId::Forest])
        .hand(P0, &[CardId::Forest; 8])
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::Cleanup)
        .build();
    let bear = common::battlefield_card(&g, P0, CardId::GrizzlyBears);
    g.state.objects.get_mut(&bear).unwrap().damage = 1;
    g.state.objects.get_mut(&bear).unwrap().pump_power = 3;
    g.state.objects.get_mut(&bear).unwrap().pump_toughness = 3;
    // Issue 15: a synthetic pump must also exist as a typed layer-7c record,
    // otherwise the layer recomputation would (correctly) clear the bare field.
    g.state.effects.push(ContinuousEffect {
        id: g.state.next_effect,
        timestamp: 1,
        source: None,
        card: CardId::GiantGrowth,
        controller: P0,
        layer: Layer::Modifying,
        target: bear,
        kind: ContinuousKind::ModifyPowerToughness {
            power: 3,
            toughness: 3,
        },
        duration: EffectDuration::UntilEndOfTurn,
    });
    g.state.next_effect += 1;
    let turn = g.state.turn.number;
    g.submit_action(P0, Action::Pass).unwrap();
    let events = g.submit_action(P1, Action::Pass).unwrap();
    assert_eq!(g.state.turn.number, turn);
    assert_eq!(g.state.turn.step, Step::Cleanup);
    assert_eq!(g.state.turn.priority, None);
    assert!(matches!(
        g.state.pending,
        Some(Pending::Discard {
            player: P0,
            count: 1
        })
    ));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(
                e,
                Event::StepBegin {
                    step: Step::Cleanup
                }
            ))
            .count(),
        1
    );
    assert_eq!(g.state.object(bear).damage, 1);
    assert_eq!(g.state.object(bear).pump_power, 3);
    let discard = g.state.player(P0).hand[0];
    g.submit_action(
        P0,
        Action::Discard {
            cards: vec![discard],
        },
    )
    .unwrap();
    assert_eq!(g.state.turn.number, turn + 1);
    assert_eq!(g.state.object(bear).damage, 0);
    assert_eq!(g.state.object(bear).pump_power, 0);
    assert_eq!(g.state.object(bear).pump_toughness, 0);
    assert_eq!(g.state.player(P0).hand.len(), 7);
    g.state.validate().unwrap();
}
