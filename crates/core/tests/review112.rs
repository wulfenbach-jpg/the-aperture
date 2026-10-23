use aperture_core::*;

fn synthetic_commander_on_top() -> Game {
    let deck = DeckList::parse_practice("fixture", "40 Forest\n").unwrap();
    let mut g = Game::new_command_zone(
        11201,
        &[deck.clone(), deck],
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .unwrap();
    g.state.mulligan = None;
    g.state.pending = None;
    let commander = g.state.players[0].command_zone.pop().unwrap();
    g.state.objects.get_mut(&commander).unwrap().zone = Zone::Library;
    g.state.players[0].library.insert(0, commander);
    g.state.turn.number = 3;
    g.state.turn.step = Step::Upkeep;
    g.state.turn.active = P0;
    g.state.turn.priority = Some(P0);
    g.state.turn.passed = 0;
    g.state.validate().unwrap();
    g
}

#[test]
fn synthetic_draw_step_suspension_has_no_priority_until_answered() {
    let mut g = synthetic_commander_on_top();
    g.submit_action(P0, Action::Pass).unwrap();
    let events = g.submit_action(P1, Action::Pass).unwrap();
    assert!(matches!(
        g.state.pending,
        Some(Pending::CommanderDraw {
            resume: DrawResume::Step,
            ..
        })
    ));
    assert_eq!(g.state.turn.priority, None);
    assert!(!events.iter().any(|e| matches!(e, Event::Priority { .. })));
    let view = g.view_for(P0, &events);
    assert_eq!(view.priority, None);
    let digest = g.digest();
    assert!(g.submit_action(P0, Action::Pass).is_err());
    assert_eq!(g.digest(), digest);
}

fn legal_commander_draw_pending(accept: bool) -> (Game, u32) {
    use aperture_core::{AiSettings, CommanderSetup, Replay, ReplayStep, SessionSave};
    let deck = DeckList::parse_practice(
        "legal timing",
        "12 Plains\n16 Island\n6 Time Ebb\n6 Consider\n",
    )
    .unwrap();
    let decks = vec![deck.clone(), deck];
    let (mut g, mut log) = Game::new_command_zone_logged(
        11111,
        &decks,
        &[
            (0, CardId::IsamaruHoundOfKonda),
            (1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .unwrap();
    let mut replay = Replay::new_command_zone(
        11111,
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
    let mut entering_events = Vec::new();
    for _ in 0..1500 {
        assert!(!g.state.is_over());
        if stage == 2
            && matches!(
                g.state.pending,
                Some(Pending::CommanderDraw {
                    resume: DrawResume::Step,
                    ..
                })
            )
        {
            g.state.validate().unwrap();
            assert_eq!(replay.run().unwrap().0.digest(), g.digest());
            let save = SessionSave::capture(&g, replay.clone(), log.clone(), false);
            let mut restored = SessionSave::from_json(&save.to_json())
                .expect("non-sandbox pending draw save loads")
                .game();
            assert_eq!(g.state.turn.priority, None);
            assert!(
                !entering_events
                    .iter()
                    .any(|e| matches!(e, Event::Priority { .. }))
            );
            for viewer in [P0, P1] {
                assert_eq!(g.view_for(viewer, &entering_events).priority, None);
            }
            assert_eq!(g.state.turn.step, Step::Draw);
            let digest = g.digest();
            assert!(g.submit_action(P0, Action::Pass).is_err());
            assert_eq!(g.digest(), digest);
            let old = g.state.commanders[0].object;
            let identity = g.state.commanders[0].id;
            let tax = g.state.commanders[0].casts_from_command_zone;
            let action = Action::AnswerCommanderDraw {
                to_command_zone: accept,
            };
            let step = ReplayStep {
                player: P0,
                revision: g.revision(),
                decision: g.decision(),
                action: action.clone(),
            };
            let events = g.submit_action(P0, action.clone()).unwrap();
            assert_eq!(restored.submit_action(P0, action).unwrap(), events);
            assert_eq!(restored.state, g.state);
            replay.record(step);
            replay.final_digest = g.digest();
            log.extend(events.clone());
            let priority: Vec<PlayerId> = events
                .iter()
                .filter_map(|event| match event {
                    Event::Priority { player } => Some(*player),
                    _ => None,
                })
                .collect();
            assert_eq!(priority, vec![g.state.turn.active]);
            assert_eq!(g.state.turn.step, Step::Draw);
            assert_eq!(g.state.turn.priority, Some(g.state.turn.active));
            let new = g.state.commanders[0].object;
            assert_ne!(new, old);
            assert!(g.state.try_object(old).is_none());
            assert_eq!(g.state.commanders[0].id, identity);
            assert_eq!(
                g.state.object(new).zone,
                if accept { Zone::Command } else { Zone::Hand }
            );
            assert_eq!(g.state.commanders[0].casts_from_command_zone, tax);
            assert_eq!(
                events
                    .iter()
                    .filter(|e| matches!(e, Event::Draw { .. }))
                    .count(),
                usize::from(!accept)
            );
            g.state.validate().unwrap();
            assert_eq!(replay.run().unwrap().0.digest(), g.digest());
            let answered = SessionSave::capture(&g, replay, log, false);
            assert_eq!(
                SessionSave::from_json(&answered.to_json()).unwrap().game,
                g.state
            );
            return (g, tax);
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
                    if let Some(a) = legal.iter().find(|a| {
                        matches!(&a.action, Action::CastSpell { card, targets }
                            if g.state.object(*card).card == CardId::TimeEbb
                                && targets == &vec![Target::Permanent(commander.object)])
                    }) {
                        stage = 1;
                        a.action.clone()
                    } else if let Some(a) = legal.iter().find(|a| {
                        matches!(a.action, Action::CastSpell { card, .. } if g.state.object(card).zone == Zone::Command)
                    }) {
                        a.action.clone()
                    } else {
                        legal
                            .iter()
                            .find(|a| matches!(a.action, Action::Pass | Action::DeclareAttackers { .. } | Action::DeclareBlockers { .. }))
                            .unwrap()
                            .action
                            .clone()
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
        entering_events = g.submit_action(seat, action).unwrap();
        log.extend(entering_events.clone());
        replay.record(step);
        replay.final_digest = g.digest();
    }
    panic!("failed to reach commander draw step");
}

#[test]
fn legal_time_ebb_decline_to_draw_decline_resumes_with_exactly_one_priority() {
    let _ = legal_commander_draw_pending(false);
}

#[test]
fn legal_time_ebb_decline_to_draw_accept_resumes_with_exactly_one_priority() {
    let _ = legal_commander_draw_pending(true);
}

#[test]
fn ordinary_draw_step_still_grants_priority_normally() {
    // Synthetic ordinary-state control; the Time Ebb tests above use only legal actions.
    let deck = DeckList::parse_practice("ordinary", "40 Forest\n").unwrap();
    let (mut g, mut _log) = Game::new_logged(11202, &[deck.clone(), deck]).unwrap();
    g.state.mulligan = None;
    g.state.pending = None;
    g.state.turn.number = 2;
    g.state.turn.step = Step::Upkeep;
    g.state.turn.active = P0;
    g.state.turn.priority = Some(P0);
    g.state.turn.passed = 0;
    g.submit_action(P0, Action::Pass).unwrap();
    let events = g.submit_action(P1, Action::Pass).unwrap();
    assert_eq!(g.state.turn.step, Step::Draw);
    assert_eq!(g.state.turn.priority, Some(P0));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Priority { player } if *player == P0))
    );
}
