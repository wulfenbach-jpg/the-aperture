//! Legal-action persistence controls for R110-1/R110-2. These are GREEN
//! controls following the assertion-first fixes, not additional claimed REDs.
use aperture_core::*;

fn apply(game: &mut Game, replay: &mut Replay, log: &mut Vec<Event>, action: Action) {
    let player = game.decision_owner().unwrap();
    let step = ReplayStep {
        player,
        revision: game.revision(),
        decision: game.decision(),
        action: action.clone(),
    };
    log.extend(game.submit_action(player, action).unwrap());
    replay.record(step);
    replay.final_digest = game.digest();
}

fn roundtrip(game: &Game, replay: &Replay, log: &[Event]) -> SessionSave {
    game.state.validate().unwrap();
    assert_eq!(replay.run().unwrap().0.digest(), game.digest());
    let save = SessionSave::capture(game, replay.clone(), log.to_vec(), false);
    let loaded = SessionSave::from_json(&save.to_json()).unwrap();
    assert!(!loaded.sandbox_modified);
    assert_eq!(loaded.game, game.state);
    loaded
}

fn apply_restored(game: &mut Game, replay: &mut Replay, log: &mut Vec<Event>, action: Action) {
    let mut restored = roundtrip(game, replay, log).game();
    let player = game.decision_owner().unwrap();
    let expected = restored.submit_action(player, action.clone()).unwrap();
    let before = log.len();
    apply(game, replay, log, action);
    assert_eq!(&log[before..], expected);
    assert_eq!(restored.state, game.state);
    roundtrip(game, replay, log);
}

fn start(seed: u64, text: &str, command: bool) -> (Game, Replay, Vec<Event>) {
    let deck = DeckList::parse_practice("ordering persistence", text).unwrap();
    let decks = vec![deck.clone(), deck];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
    ];
    let (game, log) = if command {
        Game::new_command_zone_logged(seed, &decks, &specs).unwrap()
    } else {
        Game::new_logged(seed, &decks).unwrap()
    };
    let replay = if command {
        Replay::new_command_zone(
            seed,
            game.state.first_player,
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
        .unwrap()
    } else {
        Replay::new(seed, game.state.first_player, decks, AiSettings::default()).unwrap()
    };
    (game, replay, log)
}

#[test]
fn ordinary_and_command_opt_consider_both_order_answers_save_resume_and_replay() {
    for command in [false, true] {
        for card in ["Opt", "Consider"] {
            for keep in [false, true] {
                let (mut game, mut replay, mut log) =
                    start(11101, &format!("20 Island\n20 {card}\n"), command);
                let mut reached = false;
                for _ in 0..250 {
                    if matches!(game.state.pending, Some(Pending::OrderCard { .. })) {
                        apply_restored(
                            &mut game,
                            &mut replay,
                            &mut log,
                            Action::OrderCard { to_top: keep },
                        );
                        reached = true;
                        break;
                    }
                    let legal = game.legal_actions();
                    let action = if matches!(game.state.pending, Some(Pending::Mulligan { .. })) {
                        Action::Mulligan { keep: true }
                    } else {
                        legal
                            .iter()
                            .find(|a| matches!(a.action, Action::PlayLand { .. }))
                            .or_else(|| {
                                legal
                                    .iter()
                                    .find(|a| matches!(a.action, Action::CastSpell { .. }))
                            })
                            .map(|a| a.action.clone())
                            .unwrap_or(Action::Pass)
                    };
                    apply(&mut game, &mut replay, &mut log, action);
                }
                assert!(
                    reached,
                    "did not reach {card}: command={command}, keep={keep}"
                );
            }
        }
    }
}

#[test]
fn legal_time_ebb_decline_consider_all_owner_answers_preserve_draw_identity_tax_and_replay() {
    for keep in [false, true] {
        for accept in [false, true] {
            let (mut game, mut replay, mut log) = start(
                11111,
                "12 Plains\n16 Island\n6 Time Ebb\n6 Consider\n",
                true,
            );
            let mut stage = 0;
            let mut owner = None;
            let mut original = None;
            let mut identity = None;
            let mut cast_count = 0;
            let mut expected_draw = None;
            let mut hand_before = 0;
            let mut done = false;
            for _ in 0..1000 {
                assert!(!game.state.is_over(), "journey ended in stage {stage}");
                let seat = game.decision_owner().unwrap();
                let legal = game.legal_actions();
                let action = match &game.state.pending {
                    Some(Pending::Mulligan { .. }) => Action::Mulligan { keep: true },
                    Some(Pending::CommanderReplacement { .. }) if stage == 1 => {
                        stage = 2;
                        Action::AnswerCommanderReplacement {
                            to_command_zone: false,
                        }
                    }
                    Some(Pending::OrderCard { .. }) if stage == 3 => {
                        let player = owner.unwrap();
                        let commander = game
                            .state
                            .commanders
                            .iter()
                            .find(|c| c.owner == player)
                            .unwrap();
                        assert_ne!(Some(commander.object), original);
                        assert!(game.state.try_object(original.unwrap()).is_none());
                        assert_eq!(Some(commander.id), identity);
                        assert_eq!(commander.casts_from_command_zone, cast_count);
                        expected_draw = game.state.player(player).library.first().copied();
                        hand_before = game.state.player(player).hand.len();
                        apply_restored(
                            &mut game,
                            &mut replay,
                            &mut log,
                            Action::OrderCard { to_top: keep },
                        );
                        stage = 4;
                        continue;
                    }
                    Some(Pending::CommanderReturn { .. }) if stage == 4 && !keep => {
                        assert!(game.state.stack.is_empty());
                        assert!(game.state.resolving.is_none());
                        let player = owner.unwrap();
                        assert_eq!(game.state.player(player).hand.len(), hand_before + 1);
                        assert!(
                            game.state
                                .player(player)
                                .hand
                                .contains(&expected_draw.unwrap())
                        );
                        apply_restored(
                            &mut game,
                            &mut replay,
                            &mut log,
                            Action::AnswerCommanderReturn {
                                to_command_zone: accept,
                            },
                        );
                        stage = 5;
                        continue;
                    }
                    Some(Pending::CommanderDraw { .. }) if stage == 4 && keep => {
                        apply_restored(
                            &mut game,
                            &mut replay,
                            &mut log,
                            Action::AnswerCommanderDraw {
                                to_command_zone: accept,
                            },
                        );
                        stage = 5;
                        continue;
                    }
                    Some(_) => legal.first().unwrap().action.clone(),
                    None => {
                        if stage == 5 {
                            let player = owner.unwrap();
                            let commander = game
                                .state
                                .commanders
                                .iter()
                                .find(|c| c.owner == player)
                                .unwrap();
                            assert_eq!(Some(commander.id), identity);
                            assert_eq!(commander.casts_from_command_zone, cast_count);
                            assert_eq!(
                                game.state.object(commander.object).zone,
                                if accept {
                                    Zone::Command
                                } else if keep {
                                    Zone::Hand
                                } else {
                                    Zone::Graveyard
                                }
                            );
                            assert_eq!(game.state.turn.priority, Some(game.state.turn.active));
                            assert!(game.state.stack.is_empty());
                            roundtrip(&game, &replay, &log);
                            done = true;
                            break;
                        }
                        if stage == 2 && Some(seat) == owner {
                            let action = legal
                                .iter()
                                .find(|a| {
                                    matches!(a.action, Action::CastSpell { card, .. }
                                if game.state.object(card).card == CardId::Consider)
                                })
                                .expect("Consider legal immediately after Time Ebb decline");
                            stage = 3;
                            action.action.clone()
                        } else if let Some(action) = legal
                            .iter()
                            .find(|a| matches!(a.action, Action::PlayLand { .. }))
                        {
                            action.action.clone()
                        } else if stage == 0 && game.state.turn.active == seat {
                            let commander = game
                                .state
                                .commanders
                                .iter()
                                .find(|c| c.owner == seat)
                                .unwrap();
                            let has_consider = game
                                .state
                                .player(seat)
                                .hand
                                .iter()
                                .any(|id| game.state.object(*id).card == CardId::Consider);
                            let mana = game
                                .state
                                .battlefield(seat)
                                .iter()
                                .filter(|o| {
                                    !o.tapped && matches!(o.card, CardId::Island | CardId::Plains)
                                })
                                .count();
                            if has_consider && mana >= 4 && let Some(action) = legal.iter().find(|a| matches!(&a.action, Action::CastSpell { card, targets }
                                if game.state.object(*card).card == CardId::TimeEbb && targets == &vec![Target::Permanent(commander.object)])) {
                                owner = Some(seat);
                                original = Some(commander.object);
                                identity = Some(commander.id);
                                cast_count = commander.casts_from_command_zone;
                                stage = 1;
                                action.action.clone()
                            } else if let Some(action) = legal.iter().find(|a| matches!(a.action, Action::CastSpell { card, .. } if game.state.object(card).zone == Zone::Command)) {
                                action.action.clone()
                            } else {
                                legal.iter().find(|a| matches!(a.action, Action::Pass | Action::DeclareAttackers { .. } | Action::DeclareBlockers { .. })).unwrap().action.clone()
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
                apply(&mut game, &mut replay, &mut log, action);
            }
            assert!(
                done,
                "journey failed: keep={keep}, accept={accept}, stage={stage}"
            );
        }
    }
}
