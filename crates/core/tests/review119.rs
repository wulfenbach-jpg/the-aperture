use aperture_core::*;

fn apply(g: &mut Game, r: &mut Replay, log: &mut Vec<Event>, a: Action) -> Vec<Event> {
    let p = g.decision_owner().unwrap();
    let step = ReplayStep {
        player: p,
        revision: g.revision(),
        decision: g.decision(),
        action: a.clone(),
    };
    assert!(
        g.legal_actions().iter().any(|x| x.action == a),
        "driver submits only offered actions: {a:?}"
    );
    let ev = g.submit_action(p, a).unwrap();
    r.record(step);
    r.final_digest = g.digest();
    log.extend(ev.clone());
    g.state.validate().unwrap();
    ev
}

fn journey(seats: u8, lethal: bool) {
    let seed = 11901;
    let deck = DeckList::parse_practice("combat review", "40 Plains\n").unwrap();
    let decks = vec![deck; seats as usize];
    let specs: Vec<_> = (0..seats)
        .map(|p| (p, CardId::IsamaruHoundOfKonda))
        .collect();
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
    let mut last = Vec::new();
    for _ in 0..2000 {
        if g.state.turn.step == Step::CombatDamage
            && last.iter().any(|e| matches!(e, Event::CombatDamage { .. }))
        {
            let priority_events = last
                .iter()
                .filter(|e| matches!(e, Event::Priority { .. }))
                .count();
            if lethal {
                let Some(Pending::CommanderReturn { order, .. }) = &g.state.pending else {
                    panic!(
                        "expected lethal commander return choices, got {:?}",
                        g.state.pending
                    );
                };
                assert_eq!(order.len(), 2);
                let order = order.clone();
                assert_eq!(g.state.turn.priority, None);
                for viewer in 0..seats as PlayerId {
                    assert_eq!(g.view_for(viewer, &log).priority, None);
                }
                assert_eq!(
                    priority_events, 0,
                    "no Priority before the unanswered SBA choices"
                );

                let save = SessionSave::capture(&g, r.clone(), log.clone(), false);
                let json = save.to_json();
                SessionSave::from_json(&json).expect("pending combat save loads");
                assert_eq!(r.run().unwrap().0.state, g.state);

                let ids: Vec<(CommanderId, ObjectId, u32, Zone)> = g
                    .state
                    .commanders
                    .iter()
                    .map(|c| {
                        (
                            c.id,
                            c.object,
                            c.casts_from_command_zone,
                            g.state.object(c.object).zone,
                        )
                    })
                    .collect();
                for mask in 0..4u8 {
                    let mut branch = save.game();
                    let mut restored = SessionSave::from_json(&json).unwrap().game();
                    let mut rr = r.clone();
                    let mut ll = log.clone();
                    for i in 0..2 {
                        let p = branch.decision_owner().unwrap();
                        assert_eq!(branch.state.turn.priority, None);
                        for viewer in 0..seats {
                            assert_eq!(branch.view_for(viewer, &ll).priority, None);
                        }
                        for game in [&mut branch, &mut restored] {
                            let before = game.state.clone();
                            let digest = game.digest();
                            assert!(game.submit_action(p, Action::Pass).is_err());
                            assert_eq!(game.state, before);
                            assert_eq!(game.digest(), digest);
                        }
                        let a = Action::AnswerCommanderReturn {
                            to_command_zone: mask & (1 << i) != 0,
                        };
                        let ev = apply(&mut branch, &mut rr, &mut ll, a.clone());
                        assert_eq!(restored.submit_action(p, a).unwrap(), ev);
                        assert_eq!(restored.state, branch.state);
                        assert_eq!(
                            ev.iter()
                                .filter(|e| matches!(e, Event::Priority { .. }))
                                .count(),
                            usize::from(i == 1),
                            "no priority between answers; exactly one after the final answer"
                        );
                        let answered = SessionSave::capture(&branch, rr.clone(), ll.clone(), false);
                        let reloaded = SessionSave::from_json(&answered.to_json())
                            .expect("each answered prefix must load without sandbox exemptions");
                        assert_eq!(reloaded.game, branch.state);
                        assert_eq!(rr.run().unwrap().0.state, branch.state);
                        restored = reloaded.game();
                        for viewer in 0..seats {
                            assert_eq!(
                                branch.view_for(viewer, &ll).priority,
                                if i == 0 {
                                    None
                                } else {
                                    Some(branch.state.turn.active)
                                }
                            );
                        }
                    }
                    assert_eq!(branch.state.turn.step, Step::CombatDamage);
                    assert_eq!(branch.state.turn.priority, Some(branch.state.turn.active));
                    for (id, object, tax, before_zone) in &ids {
                        let registry = branch
                            .state
                            .commanders
                            .iter()
                            .find(|c| c.id == *id)
                            .unwrap();
                        assert_eq!(registry.casts_from_command_zone, *tax);
                        let after = branch.state.object(registry.object).zone;
                        if let Some(index) = order.iter().position(|entry| entry.commander == *id) {
                            let accepted = mask & (1 << index) != 0;
                            assert_eq!(
                                after,
                                if accepted {
                                    Zone::Command
                                } else {
                                    *before_zone
                                }
                            );
                            if accepted {
                                assert_ne!(registry.object, *object);
                                assert!(branch.state.try_object(*object).is_none());
                            } else {
                                assert_eq!(registry.object, *object);
                            }
                        } else {
                            assert_eq!(registry.object, *object);
                            assert_eq!(after, *before_zone);
                        }
                    }
                    let final_save = SessionSave::capture(&branch, rr.clone(), ll, false);
                    SessionSave::from_json(&final_save.to_json())
                        .expect("answered branch save loads");
                    assert_eq!(rr.run().unwrap().0.state, branch.state);
                }
            } else {
                assert!(g.state.pending.is_none());
                assert_eq!(g.state.turn.priority, Some(g.state.turn.active));
                assert_eq!(
                    priority_events, 1,
                    "ordinary nonlethal combat still grants one priority"
                );
            }
            return;
        }
        assert!(!g.state.is_over());
        let p = g.decision_owner().unwrap();
        let ready = g
            .state
            .commanders
            .iter()
            .all(|c| g.state.object(c.object).zone == Zone::Battlefield);
        let legal = g.legal_actions();
        let a = match &g.state.pending {
            Some(Pending::Mulligan { .. }) => Action::Mulligan { keep: true },
            Some(Pending::DeclareAttackers { .. }) if ready => {
                let c = g.legal_attackers(p)[0];
                Action::DeclareAttackers { attackers: vec![c], defender: Some((p + 1) % seats) }
            }
            Some(Pending::DeclareBlockers { .. }) if lethal => {
                let blocker = g.legal_blockers(p)[0];
                Action::DeclareBlockers { blocks: vec![BlockDecl { blocker, attacker: g.state.turn.attackers[0] }] }
            }
            Some(Pending::DeclareBlockers { .. }) => Action::DeclareBlockers { blocks: vec![] },
            Some(_) => legal[0].action.clone(),
            None => legal
                .iter()
                .find(|a| matches!(a.action, Action::PlayLand { .. }))
                .or_else(|| legal.iter().find(|a| matches!(a.action, Action::CastSpell { card, .. } if g.state.object(card).zone == Zone::Command)))
                .map(|a| a.action.clone())
                .unwrap_or(Action::Pass),
        };
        last = apply(&mut g, &mut r, &mut log, a);
    }
    panic!("journey never reached combat damage");
}

#[test]
fn two_seat_commander_combat_sbas_precede_priority() {
    journey(2, true);
}
#[test]
fn three_seat_commander_combat_sbas_precede_priority() {
    journey(3, true);
}
#[test]
fn four_seat_commander_combat_sbas_precede_priority() {
    journey(4, true);
}
#[test]
fn nonlethal_combat_priority_control() {
    journey(2, false);
}
