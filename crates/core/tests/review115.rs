use aperture_core::*;

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

#[test]
fn legal_pending_commander_return_refuses_all_ordinary_actions_then_resumes() {
    let seed = 11511;
    let deck = DeckList::parse_practice(
        "legal pending land",
        "12 Plains\n12 Mountain\n8 Shock\n8 Unsummon\n",
    )
    .unwrap();
    let decks = vec![deck.clone(), deck];
    let specs = [
        (0, CardId::IsamaruHoundOfKonda),
        (1, CardId::IsamaruHoundOfKonda),
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
    for _ in 0..2500 {
        assert!(!g.state.is_over());
        let seat = g.decision_owner().unwrap();
        if killing && matches!(g.state.pending, Some(Pending::CommanderReturn { .. })) {
            assert_eq!(g.state.turn.active, seat);
            assert_eq!(g.state.turn.lands_played, 0);
            assert_eq!(g.state.turn.priority, None);
            assert!(g.state.stack.is_empty());
            let land = g
                .state
                .player(seat)
                .hand
                .iter()
                .copied()
                .find(|id| g.state.object(*id).card.def().is_land())
                .unwrap();
            let land_action = Action::PlayLand { card: land };
            assert!(
                !g.legal_actions()
                    .iter()
                    .any(|action| action.action == land_action)
            );

            let pending_save = SessionSave::capture(&g, replay.clone(), log.clone(), false);
            let pending_json = pending_save.to_json();
            SessionSave::from_json(&pending_json).expect("pending non-sandbox save loads");
            assert_eq!(replay.run().unwrap().0.digest(), g.digest());

            let before = g.state.clone();
            assert!(
                g.submit_action(seat, land_action.clone()).is_err(),
                "land must be refused during the commander return choice"
            );
            assert_eq!(
                g.state, before,
                "refused land must preserve full state/digest/revision/decision/land allowance"
            );

            let mut restored = SessionSave::from_json(&pending_json).unwrap().game();
            assert!(
                restored.submit_action(seat, land_action.clone()).is_err(),
                "restored pending game must refuse the same land"
            );
            assert_eq!(restored.state, before);

            // Sibling ordinary actions are refused atomically for this pending choice.
            let mut sibling = Game::from_state(g.state.clone()).unwrap();
            let mut controls: Vec<Action> = vec![Action::Pass];
            let spell = sibling
                .state
                .player(seat)
                .hand
                .iter()
                .copied()
                .find(|id| sibling.state.object(*id).card == CardId::Shock)
                .expect("fixture has an instant for the priority refusal control");
            controls.push(Action::CastSpell {
                card: spell,
                targets: vec![Target::Player(1 - seat)],
            });
            let mana = sibling
                .state
                .battlefield(seat)
                .iter()
                .find(|o| !o.tapped && o.card.def().is_land())
                .expect("fixture has an untapped land for the mana refusal control")
                .id;
            controls.push(Action::ActivateManaAbility { land: mana });
            controls.push(Action::DeclareAttackers {
                attackers: Vec::new(),
                defender: None,
            });
            controls.push(Action::DeclareBlockers { blocks: Vec::new() });
            for action in controls {
                let digest = sibling.digest();
                assert!(
                    sibling.submit_action(seat, action.clone()).is_err(),
                    "{action:?} must be refused"
                );
                assert_eq!(sibling.digest(), digest);
                assert_eq!(sibling.state, before);
            }
            assert!(
                sibling
                    .submit_action(1 - seat, land_action.clone())
                    .is_err()
            );
            assert_eq!(sibling.state, before);
            assert!(
                sibling
                    .submit_action(
                        1 - seat,
                        Action::AnswerCommanderReturn {
                            to_command_zone: true,
                        }
                    )
                    .is_err()
            );
            assert_eq!(sibling.state, before);

            // Both owner answers resume the same main phase with one priority.
            for accept in [false, true] {
                let mut answered = SessionSave::from_json(&pending_json).unwrap().game();
                let step = answered.state.turn.step;
                let old = answered
                    .state
                    .commanders
                    .iter()
                    .find(|c| c.owner == seat)
                    .unwrap()
                    .object;
                let tax = answered
                    .state
                    .commanders
                    .iter()
                    .find(|c| c.owner == seat)
                    .unwrap()
                    .casts_from_command_zone;
                let events = answered
                    .submit_action(
                        seat,
                        Action::AnswerCommanderReturn {
                            to_command_zone: accept,
                        },
                    )
                    .unwrap();
                let mut direct = Game::from_state(g.state.clone()).unwrap();
                let mut branch_replay = replay.clone();
                let mut branch_log = log.clone();
                assert_eq!(
                    apply(
                        &mut direct,
                        &mut branch_replay,
                        &mut branch_log,
                        Action::AnswerCommanderReturn {
                            to_command_zone: accept
                        }
                    ),
                    events
                );
                assert_eq!(direct.state, answered.state);
                SessionSave::from_json(
                    &SessionSave::capture(
                        &direct,
                        branch_replay.clone(),
                        branch_log.clone(),
                        false,
                    )
                    .to_json(),
                )
                .expect("answered non-sandbox save and replay load");
                assert!(answered.state.pending.is_none());
                assert_eq!(answered.state.turn.step, step);
                assert_eq!(answered.state.turn.priority, Some(seat));
                assert_eq!(
                    events
                        .iter()
                        .filter(|e| matches!(e, Event::Priority { .. }))
                        .count(),
                    1
                );
                let new = answered
                    .state
                    .commanders
                    .iter()
                    .find(|c| c.owner == seat)
                    .unwrap()
                    .object;
                if accept {
                    assert_ne!(new, old, "accepted return re-keys the commander");
                    assert_eq!(answered.state.object(new).zone, Zone::Command);
                } else {
                    assert_eq!(new, old, "declining leaves the same graveyard object");
                    assert_eq!(answered.state.object(new).zone, Zone::Graveyard);
                }
                assert_eq!(
                    answered
                        .state
                        .commanders
                        .iter()
                        .find(|c| c.owner == seat)
                        .unwrap()
                        .casts_from_command_zone,
                    tax
                );
                assert!(
                    answered
                        .legal_actions()
                        .iter()
                        .any(|action| action.action == land_action)
                );
                let land_events = answered.submit_action(seat, land_action.clone()).unwrap();
                assert_eq!(
                    apply(
                        &mut direct,
                        &mut branch_replay,
                        &mut branch_log,
                        land_action.clone()
                    ),
                    land_events
                );
                assert_eq!(direct.state, answered.state);
                assert_eq!(answered.state.turn.lands_played, 1);
                answered.state.validate().unwrap();

                // Persist the answered/land continuation and verify replay identity.
                assert_eq!(branch_replay.run().unwrap().0.state, answered.state);
                let save = SessionSave::capture(&answered, branch_replay, branch_log, false);
                SessionSave::from_json(&save.to_json())
                    .expect("answered+land save/replay round trip");
            }
            return;
        }
        let legal = g.legal_actions();
        let action = match &g.state.pending {
            Some(Pending::Mulligan { .. }) => Action::Mulligan { keep: true },
            Some(_) => legal.first().unwrap().action.clone(),
            None => {
                let commander = g
                    .state
                    .commanders
                    .iter()
                    .find(|c| c.owner == seat)
                    .unwrap()
                    .object;
                let land = legal
                    .iter()
                    .find(|a| matches!(a.action, Action::PlayLand { .. }));
                let kill = legal.iter().find(|a| matches!(&a.action, Action::CastSpell { card, targets }
                    if g.state.object(*card).card == CardId::Shock && *targets == vec![Target::Permanent(commander)]));
                if !killing && let (Some(land), Some(kill)) = (land, kill)
                    && seat == g.state.turn.active
                    && g.state.turn.step.is_main()
                    && g.state.stack.is_empty()
                {
                    // Ordinary land control on the same real state before the Shock.
                    let mut control = Game::from_state(g.state.clone()).unwrap();
                    control.submit_action(seat, land.action.clone()).unwrap();
                    assert_eq!(control.state.turn.lands_played, 1);
                    killing = true;
                    kill.action.clone()
                } else if let Some(a) = land {
                    a.action.clone()
                } else if !killing
                    && let Some(a) = legal.iter().find(|a| matches!(a.action, Action::CastSpell { card, .. } if g.state.object(card).zone == Zone::Command))
                {
                    a.action.clone()
                } else {
                    legal
                        .iter()
                        .find(|a| matches!(a.action, Action::Pass | Action::DeclareAttackers { .. } | Action::DeclareBlockers { .. }))
                        .unwrap()
                        .action
                        .clone()
                }
            }
        };
        apply(&mut g, &mut replay, &mut log, action);
    }
    panic!("failed to reach pending commander return");
}
