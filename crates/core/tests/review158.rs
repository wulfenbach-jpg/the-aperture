use aperture_core::*;

fn journey(command: bool, attacks: bool, seats: u8) -> (Game, Replay, Vec<Event>, Vec<Event>) {
    let seed = 15801;
    let deck = DeckList::parse_practice("review158", "40 Plains\n").unwrap();
    let decks = vec![deck; seats as usize];
    let specs: Vec<_> = (0..seats)
        .map(|p| (p, CardId::IsamaruHoundOfKonda))
        .collect();
    let (mut g, mut log) = if command {
        Game::new_command_zone_logged(seed, &decks, &specs)
    } else {
        Game::new_logged(seed, &decks)
    }
    .unwrap();
    let mut r = if command {
        Replay::new_command_zone(
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
    } else {
        Replay::new(seed, g.state.first_player, decks, AiSettings::default())
    }
    .unwrap();
    let mut last = Vec::new();
    for _ in 0..10000 {
        if g.state.is_over() {
            let (replayed, replay_events) = r.run().unwrap();
            assert_eq!(replayed.state, g.state);
            assert_eq!(replay_events, log);
            let save = SessionSave::capture(&g, r.clone(), log.clone(), false);
            assert_eq!(
                SessionSave::from_json(&save.to_json()).unwrap().game,
                g.state
            );
            return (g, r, log, last);
        }
        let p = g.decision_owner().unwrap();
        let legal = g.legal_actions();
        let action = match &g.state.pending {
            Some(Pending::Mulligan { .. }) => Action::Mulligan { keep: true },
            Some(Pending::DeclareAttackers { .. }) => {
                let a = if attacks {
                    g.legal_attackers(p)
                } else {
                    vec![]
                };
                let defender = if a.is_empty() {
                    None
                } else {
                    g.state.next_live_seat_after(p)
                };
                Action::DeclareAttackers {
                    attackers: a,
                    defender,
                }
            }
            Some(Pending::DeclareBlockers { .. }) => Action::DeclareBlockers { blocks: vec![] },
            Some(_) => legal[0].action.clone(),
            None => legal
                .iter()
                .find(|a| matches!(a.action, Action::PlayLand { .. }))
                .or_else(|| {
                    if attacks {
                        legal
                            .iter()
                            .find(|a| matches!(a.action, Action::CastSpell { .. }))
                    } else {
                        None
                    }
                })
                .map(|a| a.action.clone())
                .unwrap_or(Action::Pass),
        };
        assert!(legal.iter().any(|a| a.action == action));
        let step = ReplayStep {
            player: p,
            revision: g.revision(),
            decision: g.decision(),
            action: action.clone(),
        };
        // GREEN strengthening: restore the actual preterminal action boundary,
        // not just a terminal snapshot whose wrong event log could replay.
        let terminal_boundary = g.state.pending.is_none()
            && ((g.state.turn.step == Step::DeclareBlockers
                && !g.state.turn.attackers.is_empty()
                && g.state.players.iter().any(|p| !p.lost && p.life <= 2))
                || (g.state.turn.step == Step::Upkeep
                    && g.state.player(g.state.turn.active).library.is_empty()));
        let mut restored = terminal_boundary.then(|| {
            SessionSave::from_json(
                &SessionSave::capture(&g, r.clone(), log.clone(), false).to_json(),
            )
            .unwrap()
            .game()
        });
        last = g.submit_action(p, action.clone()).unwrap();
        if let Some(restored) = &mut restored {
            assert_eq!(restored.submit_action(p, action).unwrap(), last);
            assert_eq!(restored.state, g.state);
        }
        if g.state.is_over() {
            assert!(
                restored.is_some(),
                "terminal action was restored before submission"
            );
            let before = g.state.clone();
            assert!(matches!(
                g.submit_action(p, Action::Pass),
                Err(EngineError::GameOver)
            ));
            assert_eq!(g.state, before);
        }
        r.record(step);
        r.final_digest = g.digest();
        log.extend(last.clone());
        g.state.validate().unwrap();
    }
    panic!("journey capped");
}

#[test]
fn commander_lethal_combat_emits_exactly_one_terminal_event() {
    for seats in [2, 3, 4] {
        let (g, _, _, last) = journey(true, true, seats);
        assert_eq!(g.state.turn.step, Step::CombatDamage);
        assert_eq!(
            last.iter()
                .filter(|e| matches!(e, Event::GameOver { .. }))
                .count(),
            1,
            "one terminal action must emit exactly one GameOver"
        );
        assert!(!last.iter().any(|e| matches!(e, Event::Priority { .. })));
    }
}

#[test]
fn ordinary_terminal_empty_draw_emits_one_terminal_event() {
    let (_, _, _, last) = journey(false, false, 2);
    assert_eq!(
        last.iter()
            .filter(|e| matches!(e, Event::GameOver { .. }))
            .count(),
        1
    );
}

fn legal_draw_pending_save() -> SessionSave {
    legal_timing_pending_save(false)
}

fn legal_timing_pending_save(replacement: bool) -> SessionSave {
    let seed = 11111;
    let deck = DeckList::parse_practice(
        "legal timing",
        "12 Plains\n16 Island\n6 Time Ebb\n6 Consider\n",
    )
    .unwrap();
    let decks = vec![deck.clone(), deck];
    let (mut g, mut log) = Game::new_command_zone_logged(
        seed,
        &decks,
        &[
            (0, CardId::IsamaruHoundOfKonda),
            (1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .unwrap();
    let mut r = Replay::new_command_zone(
        seed,
        g.state.first_player,
        decks,
        vec![
            CommanderSetup {
                seat: 0,
                card: CardId::IsamaruHoundOfKonda,
            },
            CommanderSetup {
                seat: 1,
                card: CardId::IsamaruHoundOfKonda,
            },
        ],
        AiSettings::default(),
    )
    .unwrap();
    let mut stage = 0;
    for _ in 0..1500 {
        if (replacement
            && stage == 1
            && matches!(g.state.pending, Some(Pending::CommanderReplacement { .. })))
            || (!replacement
                && stage == 2
                && matches!(
                    g.state.pending,
                    Some(Pending::CommanderDraw {
                        resume: DrawResume::Step,
                        ..
                    })
                ))
        {
            g.state.validate().unwrap();
            assert_eq!(r.run().unwrap().0.state, g.state);
            let save = SessionSave::capture(&g, r.clone(), log.clone(), false);
            SessionSave::from_json(&save.to_json()).unwrap();
            return save;
        }
        let legal = g.legal_actions();
        let seat = g.decision_owner().unwrap();
        let action = match g.state.pending {
            Some(Pending::Mulligan { .. }) => Action::Mulligan { keep: true },
            Some(Pending::CommanderReplacement { .. }) if stage == 1 => {
                stage = 2;
                Action::AnswerCommanderReplacement {
                    to_command_zone: false,
                }
            }
            Some(_) => legal.first().unwrap().action.clone(),
            None => {
                if let Some(a) = legal
                    .iter()
                    .find(|a| matches!(a.action, Action::PlayLand { .. }))
                {
                    a.action.clone()
                } else if stage == 0 && g.state.turn.active == seat {
                    let commander = g.state.commanders.iter().find(|c| c.owner == seat).unwrap();
                    if let Some(a) = legal.iter().find(|a| matches!(&a.action, Action::CastSpell { card, targets }
                        if g.state.object(*card).card == CardId::TimeEbb && targets == &vec![Target::Permanent(commander.object)]
                            && (!replacement || g.state.battlefield(seat).iter().filter(|o| o.card == CardId::Island && !o.tapped).count() >= 4)))
                    {
                        stage = 1;
                        a.action.clone()
                    } else if let Some(a) = legal.iter().find(|a| matches!(a.action, Action::CastSpell { card, .. } if g.state.object(card).zone == Zone::Command)) {
                        a.action.clone()
                    } else {
                        Action::Pass
                    }
                } else {
                    legal
                        .iter()
                        .find(|a| {
                            matches!(
                                a.action,
                                Action::Pass
                                    | Action::DeclareAttackers { .. }
                                    | Action::DeclareBlockers { .. }
                            )
                        })
                        .unwrap()
                        .action
                        .clone()
                }
            }
        };
        let step = ReplayStep {
            player: seat,
            revision: g.revision(),
            decision: g.decision(),
            action: action.clone(),
        };
        log.extend(g.submit_action(seat, action).unwrap());
        r.record(step);
        r.final_digest = g.digest();
    }
    panic!("failed to reach draw pending");
}

#[test]
fn pending_priority_is_rejected_in_state_and_sandbox_save() {
    assert_pending_priority_rejected(&legal_draw_pending_save());
}

fn assert_pending_priority_rejected(save: &SessionSave) {
    assert_eq!(save.game.turn.priority, None);
    let p = save.game.pending.as_ref().unwrap().player();
    let mut forged = save.clone();
    forged.game.turn.priority = Some(p);
    forged.sandbox_modified = true;
    forged.digest = forged.game().digest();
    assert!(
        Game::from_state(forged.game.clone()).is_err(),
        "pending+priority is contradictory"
    );
    assert!(
        SessionSave::from_json(&forged.to_json()).is_err(),
        "sandbox does not exempt contradictory pending/priority"
    );
}

#[test]
fn ordinary_cast_is_refused_during_pending_without_mutation_even_if_priority_is_forged() {
    let save = legal_draw_pending_save();
    let p = save.game.pending.as_ref().unwrap().player();
    let card = save
        .game
        .player(p)
        .hand
        .iter()
        .copied()
        .find(|id| save.game.object(*id).card == CardId::Consider)
        .expect("Consider in hand");
    assert_pending_actions_refused(
        &save,
        Action::CastSpell {
            card,
            targets: vec![],
        },
    );
    // Direct canonical game.
    let mut canonical = save.game();
    let before = canonical.state.clone();
    assert!(
        canonical
            .submit_action(
                p,
                Action::CastSpell {
                    card,
                    targets: vec![]
                }
            )
            .is_err()
    );
    assert_eq!(canonical.state, before);

    // Unvalidated Game built from a forged priority state: the spell validator
    // itself must refuse any pending choice, not rely on GameState::validate.
    let mut forged = save.clone();
    forged.game.turn.priority = Some(p);
    let mut bypass = Game { state: forged.game };
    let before = bypass.state.clone();
    assert!(
        bypass
            .submit_action(
                p,
                Action::CastSpell {
                    card,
                    targets: vec![]
                }
            )
            .is_err()
    );
    assert_eq!(bypass.state, before);

    // Both answers must resume with the same payable Consider, not an optional
    // cast search which could silently skip the positive control.
    for accept in [false, true] {
        assert_pending_continuation(
            &save,
            Action::AnswerCommanderDraw {
                to_command_zone: accept,
            },
            Action::CastSpell {
                card,
                targets: vec![],
            },
        );
    }
}

// GREEN-only continuation matrix: no new test-first provenance claimed.
fn assert_pending_continuation(save: &SessionSave, answer: Action, cast: Action) {
    let mut direct = Game::from_state(save.game.clone()).unwrap();
    let mut restored = SessionSave::from_json(&save.to_json()).unwrap().game();
    let mut replay = save.replay.clone();
    let mut log = save.log.clone();
    for action in [answer, cast] {
        let seat = direct.decision_owner().unwrap();
        let step = ReplayStep {
            player: seat,
            revision: direct.revision(),
            decision: direct.decision(),
            action: action.clone(),
        };
        let events = direct.submit_action(seat, action.clone()).unwrap();
        assert_eq!(restored.submit_action(seat, action).unwrap(), events);
        assert_eq!(restored.state, direct.state);
        replay.record(step);
        replay.final_digest = direct.digest();
        log.extend(events);
        let (replayed, replay_events) = replay.run().unwrap();
        assert_eq!(replayed.state, direct.state);
        assert_eq!(replay_events, log);
        let roundtrip = SessionSave::capture(&direct, replay.clone(), log.clone(), false);
        assert_eq!(
            SessionSave::from_json(&roundtrip.to_json()).unwrap().game,
            direct.state
        );
    }
}

fn assert_pending_actions_refused(save: &SessionSave, cast: Action) {
    assert_pending_priority_rejected(save);
    let p = save.game.pending.as_ref().unwrap().player();
    for forged in [false, true] {
        let mut game = save.game();
        if forged {
            game.state.turn.priority = Some(p);
        }
        let before = game.state.clone();
        let digest = game.digest();
        let mut actions = vec![cast.clone(), Action::Pass];
        if let Some(land) = game
            .state
            .player(p)
            .hand
            .iter()
            .copied()
            .find(|id| game.state.object(*id).card.def().is_land())
        {
            actions.push(Action::PlayLand { card: land });
        }
        if let Some(land) = game
            .state
            .battlefield(p)
            .iter()
            .find(|o| !o.tapped && o.card.def().is_land())
        {
            actions.push(Action::ActivateManaAbility { land: land.id });
        }
        for action in actions {
            assert!(game.submit_action(p, action).is_err());
            assert_eq!(game.state, before);
            assert_eq!(game.digest(), digest);
        }
    }
}

#[test]
fn legal_replacement_pending_invariant_and_both_continuations() {
    let save = legal_timing_pending_save(true);
    assert!(matches!(
        save.game.pending,
        Some(Pending::CommanderReplacement { .. })
    ));
    let p = save.game.pending.as_ref().unwrap().player();
    let card = save
        .game
        .player(p)
        .hand
        .iter()
        .copied()
        .find(|id| save.game.object(*id).card == CardId::Consider)
        .unwrap();
    let cast = Action::CastSpell {
        card,
        targets: vec![],
    };
    assert_pending_actions_refused(&save, cast.clone());
    for accept in [false, true] {
        assert_pending_continuation(
            &save,
            Action::AnswerCommanderReplacement {
                to_command_zone: accept,
            },
            cast.clone(),
        );
    }
}

fn legal_return_pending_save() -> SessionSave {
    let seed = 11511;
    let deck = DeckList::parse_practice(
        "legal return",
        "12 Plains\n12 Mountain\n8 Shock\n8 Unsummon\n",
    )
    .unwrap();
    let decks = vec![deck.clone(), deck];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
    ];
    let (mut g, mut log) = Game::new_command_zone_logged(seed, &decks, &specs).unwrap();
    let mut replay = Replay::new_command_zone(
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
    let mut killing = false;
    for _ in 0..2000 {
        assert!(!g.state.is_over());
        if killing && matches!(g.state.pending, Some(Pending::CommanderReturn { .. })) {
            let save = SessionSave::capture(&g, replay, log, false);
            return SessionSave::from_json(&save.to_json()).unwrap();
        }
        let p = g.decision_owner().unwrap();
        let legal = g.legal_actions();
        let action = match g.state.pending {
            Some(Pending::Mulligan { .. }) => Action::Mulligan { keep: true },
            Some(_) => legal[0].action.clone(),
            None => {
                let commander = g
                    .state
                    .commanders
                    .iter()
                    .find(|c| c.owner == p)
                    .unwrap()
                    .object;
                let land = legal
                    .iter()
                    .find(|a| matches!(a.action, Action::PlayLand { .. }));
                let kill = legal.iter().find(|a| matches!(&a.action, Action::CastSpell { card, targets }
                    if g.state.object(*card).card == CardId::Shock && *targets == vec![Target::Permanent(commander)]));
                if !killing && land.is_some() && let Some(kill) = kill && p == g.state.turn.active
                    && g.state.turn.step.is_main() && g.state.stack.is_empty()
                    && g.state.battlefield(p).iter().filter(|o| o.card == CardId::Mountain && !o.tapped).count() >= 2 {
                    killing = true;
                    kill.action.clone()
                } else if let Some(land) = land { land.action.clone() }
                else if !killing && let Some(cast) = legal.iter().find(|a|
                    matches!(a.action, Action::CastSpell { card, .. } if g.state.object(card).zone == Zone::Command)) {
                    cast.action.clone()
                } else {
                    legal.iter().find(|a| matches!(a.action, Action::Pass | Action::DeclareAttackers { .. } | Action::DeclareBlockers { .. }))
                        .unwrap().action.clone()
                }
            }
        };
        assert!(legal.iter().any(|a| a.action == action));
        let step = ReplayStep {
            player: p,
            revision: g.revision(),
            decision: g.decision(),
            action: action.clone(),
        };
        log.extend(g.submit_action(p, action).unwrap());
        replay.record(step);
        replay.final_digest = g.digest();
    }
    panic!("no legal commander return reached");
}

#[test]
fn legal_return_pending_invariant_and_both_continuations() {
    let save = legal_return_pending_save();
    let p = save.game.pending.as_ref().unwrap().player();
    let card = save
        .game
        .player(p)
        .hand
        .iter()
        .copied()
        .find(|id| save.game.object(*id).card == CardId::Shock)
        .unwrap();
    let cast = Action::CastSpell {
        card,
        targets: vec![Target::Player(1 - p)],
    };
    assert_pending_actions_refused(&save, cast.clone());
    for accept in [false, true] {
        assert_pending_continuation(
            &save,
            Action::AnswerCommanderReturn {
                to_command_zone: accept,
            },
            cast.clone(),
        );
    }
}
