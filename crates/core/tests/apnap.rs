//! Real APNAP producer: Crack the Earth (normal public sorcery), deferred
//! simultaneous sacrifices, persistence and departure boundaries.

#[test]
fn crack_the_earth_is_a_supported_normal_cast_card() {
    let list = DeckList::parse_practice("Crack", "1 Crack the Earth\n40 Mountain\n")
        .expect("Crack the Earth must be in the supported pool");
    let id = aperture_core::catalog::by_name("Crack the Earth").expect("catalog entry");
    assert_eq!(list.count_of(id), 1);
    assert!(
        aperture_core::catalog::pool_size() >= 16,
        "the accepted practice corpus must stay at least 16 cards"
    );
}

mod common;

use aperture_core::view::PendingKind;
use aperture_core::*;
use common::*;

fn cast_to_pending(game: &mut Game, caster: PlayerId, events: &mut Vec<Event>) {
    let crack = hand_card(game, caster, CardId::CrackTheEarth);
    events.extend(submit_ok(
        game,
        caster,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    ));
    let mut guard = 0;
    while !matches!(game.state.pending, Some(Pending::ApnapSacrifice { .. })) {
        let owner = game.decision_owner().expect("priority while resolving");
        events.extend(submit_ok(game, owner, Action::Pass));
        guard += 1;
        assert!(guard < 16, "the spell never reached its APNAP queue");
    }
}

fn pending_parts(game: &Game) -> (PlayerId, Vec<PlayerId>, usize) {
    match &game.state.pending {
        Some(Pending::ApnapSacrifice {
            player,
            order,
            choices,
            ..
        }) => (*player, order.clone(), choices.len()),
        other => panic!("expected an APNAP sacrifice queue, got {other:?}"),
    }
}

#[test]
fn real_crack_the_earth_cast_defers_all_sacrifices_until_the_last_answer() {
    let mut game = fixture_n(200, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let crack = hand_card(&game, P0, CardId::CrackTheEarth);
    let forest = battlefield_card(&game, P0, CardId::Forest);
    let bear = battlefield_card(&game, 1, CardId::GrizzlyBears);
    let mountain = battlefield_card(&game, P0, CardId::Mountain);
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);

    let (player, order, chosen) = pending_parts(&game);
    assert_eq!(player, P0);
    assert_eq!(order, vec![P0, 1, 2]);
    assert_eq!(chosen, 0);
    assert_eq!(game.state.stack.len(), 1);
    assert_eq!(game.state.resolving, Some(crack));
    assert_eq!(game.state.turn.priority, None);
    assert!(!game.legal_actions().is_empty(), "owner has choices");

    events.extend(submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(forest),
        },
    ));
    let (player, order, chosen) = pending_parts(&game);
    assert_eq!(player, 1);
    assert_eq!(order, vec![1, 2]);
    assert_eq!(chosen, 1);
    assert_eq!(game.state.object(forest).zone, Zone::Battlefield);
    assert_eq!(game.state.object(bear).zone, Zone::Battlefield);
    assert_eq!(game.state.stack.len(), 1, "the spell is still resolving");
    assert_eq!(game.state.turn.priority, None);

    events.extend(submit_ok(
        &mut game,
        1,
        Action::ChooseSacrifice {
            permanent: Some(bear),
        },
    ));
    assert_eq!(pending_parts(&game).0, 2);
    assert_eq!(game.state.object(bear).zone, Zone::Battlefield);
    let legal = game.legal_actions();
    assert_eq!(legal.len(), 1, "P2 has no permanents: only forced None");
    assert!(matches!(
        legal[0].action,
        Action::ChooseSacrifice { permanent: None }
    ));

    events.extend(submit_ok(
        &mut game,
        2,
        Action::ChooseSacrifice { permanent: None },
    ));
    assert_eq!(game.state.pending, None);
    assert_eq!(game.state.object(forest).zone, Zone::Graveyard);
    assert_eq!(game.state.object(bear).zone, Zone::Graveyard);
    assert_eq!(game.state.object(mountain).zone, Zone::Battlefield);
    assert_eq!(game.state.stack.len(), 0);
    assert_eq!(game.state.turn.priority, Some(P0));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::SacrificeChosen { .. }))
            .count(),
        3
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                Event::Destroyed { object, .. } if *object == forest || *object == bear
            ))
            .count(),
        2,
        "only the two chosen permanents were sacrificed"
    );
}

#[test]
fn apnap_order_anchors_at_nonzero_active_and_wraps() {
    let mut game = fixture_n(201, 4)
        .hand(2, &[CardId::CrackTheEarth])
        .battlefield(2, &[CardId::Mountain])
        .main(2, Step::PrecombatMain)
        .build();
    let mut events = Vec::new();
    cast_to_pending(&mut game, 2, &mut events);
    let (player, order, _) = pending_parts(&game);
    assert_eq!(player, 2);
    assert_eq!(order, vec![2, 3, 0, 1]);
}

#[test]
fn apnap_skips_an_already_lost_seat() {
    let mut game = fixture_n(202, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain])
        .life(1, 0)
        .main(P0, Step::PrecombatMain)
        .build();
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);
    assert!(game.state.player(1).lost);
    let (_, order, _) = pending_parts(&game);
    assert_eq!(order, vec![P0, 2]);
}

#[test]
fn apnap_rejections_leave_the_queue_unchanged() {
    let mut game = fixture_n(203, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, 1, CardId::GrizzlyBears);
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);
    let digest = game.digest();
    let revision = game.revision();
    let pending = game.state.pending.clone();
    assert!(matches!(
        submit_err(
            &mut game,
            1,
            Action::ChooseSacrifice {
                permanent: Some(bear)
            }
        ),
        EngineError::NotYourDecision { .. }
    ));
    assert!(matches!(
        submit_err(&mut game, P0, Action::ChooseSacrifice { permanent: None }),
        EngineError::Illegal { .. }
    ));
    assert!(matches!(
        submit_err(
            &mut game,
            P0,
            Action::ChooseSacrifice {
                permanent: Some(ObjectId(99_999))
            }
        ),
        EngineError::UnknownObject
    ));
    assert!(matches!(
        submit_err(
            &mut game,
            P0,
            Action::ChooseSacrifice {
                permanent: Some(bear)
            }
        ),
        EngineError::WrongZone { .. } | EngineError::Illegal { .. }
    ));
    let stale = Submission {
        player: P0,
        revision: revision.wrapping_sub(1),
        decision: game.decision(),
        action: Action::ChooseSacrifice { permanent: None },
    };
    assert!(matches!(
        game.submit(&stale),
        Err(EngineError::StaleRevision { .. })
    ));
    assert_eq!(game.digest(), digest);
    assert_eq!(game.state.pending, pending);
}

#[test]
fn counterspell_prevents_the_apnap_queue() {
    let mut game = fixture_n(204, 2)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain])
        .hand(1, &[CardId::Counterspell])
        .battlefield(1, &[CardId::Island, CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let crack = hand_card(&game, P0, CardId::CrackTheEarth);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    let counter = hand_card(&game, 1, CardId::Counterspell);
    submit_ok(
        &mut game,
        1,
        Action::CastSpell {
            card: counter,
            targets: vec![Target::Spell(crack)],
        },
    );
    submit_ok(&mut game, 1, Action::Pass);
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.stack.len(), 0);
    assert_eq!(game.state.pending, None);
    assert_eq!(game.state.object(crack).zone, Zone::Graveyard);
    assert_eq!(game.state.battlefield(P0).len(), 1, "no sacrifice happened");
}

#[test]
fn departing_participant_is_purged_and_remaining_answers_resume() {
    let mut game = fixture_n(205, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .battlefield(2, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let p0_forest = battlefield_card(&game, P0, CardId::Forest);
    let p2_forest = battlefield_card(&game, 2, CardId::Forest);
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);

    game.state.players[1].life = 0;
    events.extend(submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(p0_forest),
        },
    ));
    assert!(game.state.player(1).lost);
    let (player, order, chosen) = pending_parts(&game);
    assert_eq!(player, 2);
    assert_eq!(order, vec![2]);
    assert_eq!(chosen, 1, "P0's answer is preserved");
    assert_eq!(game.state.object(p0_forest).zone, Zone::Battlefield);

    events.extend(submit_ok(
        &mut game,
        2,
        Action::ChooseSacrifice {
            permanent: Some(p2_forest),
        },
    ));
    assert_eq!(game.state.object(p0_forest).zone, Zone::Graveyard);
    assert_eq!(game.state.object(p2_forest).zone, Zone::Graveyard);
    assert_eq!(game.state.pending, None);
    assert_eq!(game.state.stack.len(), 0);
}

#[test]
fn last_live_seat_outcome_discards_the_pending_queue() {
    let mut game = fixture_n(206, 2)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let forest = battlefield_card(&game, P0, CardId::Forest);
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);
    game.state.players[1].life = 0;
    submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(forest),
        },
    );
    assert!(matches!(
        game.state.outcome,
        Some(Outcome::Win { winner: P0, .. })
    ));
    assert_eq!(game.state.pending, None);
}

#[test]
fn malformed_apnap_queue_is_rejected_by_state_validation() {
    let mut game = fixture_n(207, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);
    let mut malformed = game.state.clone();
    if let Some(Pending::ApnapSacrifice { order, .. }) = &mut malformed.pending {
        order.push(order[0]);
    }
    assert!(malformed.validate().is_err());

    let mut foreign = game.state.clone();
    if let Some(Pending::ApnapSacrifice { choices, .. }) = &mut foreign.pending {
        choices.insert(1, Some(ObjectId(99_999)));
    }
    assert!(foreign.validate().is_err());
}

#[test]
fn apnap_pending_view_keeps_choices_public_and_hidden_state_masked() {
    fn pending_game(seed: u64, hand: CardId) -> Game {
        let mut game = fixture_n(seed, 3)
            .hand(P0, &[CardId::CrackTheEarth])
            .battlefield(P0, &[CardId::Mountain, CardId::Forest])
            .battlefield(1, &[CardId::GrizzlyBears])
            .hand(1, &[hand])
            .library(1, &[hand, hand])
            .main(P0, Step::PrecombatMain)
            .build();
        let mut events = Vec::new();
        cast_to_pending(&mut game, P0, &mut events);
        let forest = battlefield_card(&game, P0, CardId::Forest);
        submit_ok(
            &mut game,
            P0,
            Action::ChooseSacrifice {
                permanent: Some(forest),
            },
        );
        game
    }
    let a = pending_game(208, CardId::Forest);
    let b = pending_game(208, CardId::Island);
    assert_ne!(a.digest(), b.digest());
    for viewer in [P0, 2] {
        let view_a = a.view_for(viewer, &[]);
        let view_b = b.view_for(viewer, &[]);
        assert_eq!(view_a.to_json(), view_b.to_json());
        assert_eq!(view_a.digest, view_b.digest);
    }
    let owner = a.state.decision_owner().expect("next chooser");
    let owner_view = a.view_for(owner, &[]);
    assert_eq!(owner_view.pending_kind, Some(PendingKind::ApnapSacrifice));
    assert!(!owner_view.legal.is_empty());
    let p0 = a.view_for(P0, &[]);
    assert_eq!(p0.pending_kind, None);
    assert!(p0.legal.is_empty());
    assert!(p0.pending_viewed.is_empty());
}

fn recorded_submit(
    game: &mut Game,
    replay: &mut Replay,
    player: PlayerId,
    action: Action,
    log: &mut Vec<Event>,
) {
    let revision = game.revision();
    let decision = game.decision();
    let events = game
        .submit_action(player, action.clone())
        .expect("recorded APNAP action is legal");
    replay.record(ReplayStep {
        player,
        revision,
        decision,
        action,
    });
    log.extend(events);
}

#[test]
fn intermediate_apnap_queue_save_and_replay_round_trip() {
    let deck = DeckList::new(
        "Crack practice",
        vec![(CardId::CrackTheEarth, 4), (CardId::Mountain, 36)],
    );
    let decks = vec![deck; 2];
    let mut found = None;
    for seed in 0..1_000u64 {
        let (game, _) = Game::new_logged(seed, &decks).expect("game");
        let target = game.state.first_player;
        let hand = &game.state.player(target).hand;
        let has_crack = hand
            .iter()
            .any(|id| game.state.object(*id).card == CardId::CrackTheEarth);
        let has_mountain = hand
            .iter()
            .any(|id| game.state.object(*id).card == CardId::Mountain);
        if has_crack && has_mountain {
            found = Some((seed, target));
            break;
        }
    }
    let (seed, target) = found.expect("a seed with Crack the Earth and Mountain");
    let (mut game, _) = Game::new_logged(seed, &decks).expect("game");
    let mut replay =
        Replay::new(seed, game.state.first_player, decks, AiSettings::default()).expect("replay");
    let mut log = Vec::new();

    let order = game
        .state
        .mulligan
        .as_ref()
        .expect("mulligan state")
        .order
        .clone();
    for player in order {
        recorded_submit(
            &mut game,
            &mut replay,
            player,
            Action::Mulligan { keep: true },
            &mut log,
        );
    }

    let mut guard = 0;
    while !(game.state.turn.active == target
        && game.state.turn.step == Step::PrecombatMain
        && game.state.turn.priority == Some(target)
        && game.state.stack.is_empty())
    {
        let owner = game.decision_owner().expect("a priority holder");
        recorded_submit(&mut game, &mut replay, owner, Action::Pass, &mut log);
        guard += 1;
        assert!(guard < 64, "never reached the first main phase");
    }

    let mountain = game
        .state
        .player(target)
        .hand
        .iter()
        .copied()
        .find(|id| game.state.object(*id).card == CardId::Mountain)
        .expect("Mountain in hand");
    recorded_submit(
        &mut game,
        &mut replay,
        target,
        Action::PlayLand { card: mountain },
        &mut log,
    );
    recorded_submit(
        &mut game,
        &mut replay,
        target,
        Action::ActivateManaAbility { land: mountain },
        &mut log,
    );
    let crack = game
        .state
        .player(target)
        .hand
        .iter()
        .copied()
        .find(|id| game.state.object(*id).card == CardId::CrackTheEarth)
        .expect("Crack in hand");
    recorded_submit(
        &mut game,
        &mut replay,
        target,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
        &mut log,
    );
    while !matches!(game.state.pending, Some(Pending::ApnapSacrifice { .. })) {
        let owner = game.decision_owner().expect("priority while resolving");
        recorded_submit(&mut game, &mut replay, owner, Action::Pass, &mut log);
    }

    let (chooser, _, _) = pending_parts(&game);
    assert_eq!(chooser, target);
    let sacrifice = game
        .legal_actions()
        .into_iter()
        .find_map(|legal| match legal.action {
            Action::ChooseSacrifice {
                permanent: Some(id),
            } => Some(id),
            _ => None,
        })
        .expect("the caster controls a permanent");
    recorded_submit(
        &mut game,
        &mut replay,
        target,
        Action::ChooseSacrifice {
            permanent: Some(sacrifice),
        },
        &mut log,
    );
    assert!(matches!(
        game.state.pending,
        Some(Pending::ApnapSacrifice { .. })
    ));

    let intermediate = SessionSave::capture(&game, replay.clone(), log.clone(), false);
    let parsed = SessionSave::from_json(&intermediate.to_json())
        .expect("intermediate APNAP save round-trips");
    assert_eq!(parsed.game().digest(), game.digest());
    let (replayed, _) = replay.run().expect("intermediate APNAP replay reproduces");
    assert_eq!(replayed.state, game.state);

    let remaining = game.decision_owner().expect("remaining chooser");
    let choice = game
        .legal_actions()
        .into_iter()
        .find(|legal| matches!(legal.action, Action::ChooseSacrifice { .. }))
        .expect("remaining choice offered")
        .action;
    recorded_submit(&mut game, &mut replay, remaining, choice, &mut log);
    assert_eq!(game.state.pending, None);
    assert_eq!(game.state.stack.len(), 0);
    replay.final_digest = game.digest();
    let final_save = SessionSave::capture(&game, replay.clone(), log, false);
    SessionSave::from_json(&final_save.to_json()).expect("final APNAP save round-trips");
    let (replayed, _) = replay.run().expect("final APNAP replay reproduces");
    assert_eq!(replayed.digest(), game.digest());
}

#[test]
fn departing_active_current_chooser_purges_its_answer_and_resumes() {
    let mut game = fixture_n(209, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .battlefield(2, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let p0_forest = battlefield_card(&game, P0, CardId::Forest);
    let bear = battlefield_card(&game, 1, CardId::GrizzlyBears);
    let p2_forest = battlefield_card(&game, 2, CardId::Forest);
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);

    game.state.players[P0 as usize].life = 0;
    events.extend(submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(p0_forest),
        },
    ));
    assert!(game.state.player(P0).lost);
    assert_eq!(game.state.turn.active, P0, "the turn continues");
    let (player, order, chosen) = pending_parts(&game);
    assert_eq!(player, 1);
    assert_eq!(order, vec![1, 2]);
    assert_eq!(chosen, 0, "the departing answer was purged");
    assert_eq!(game.state.object(p0_forest).zone, Zone::Removed);

    events.extend(submit_ok(
        &mut game,
        1,
        Action::ChooseSacrifice {
            permanent: Some(bear),
        },
    ));
    events.extend(submit_ok(
        &mut game,
        2,
        Action::ChooseSacrifice {
            permanent: Some(p2_forest),
        },
    ));
    assert_eq!(game.state.object(bear).zone, Zone::Graveyard);
    assert_eq!(game.state.object(p2_forest).zone, Zone::Graveyard);
    assert_eq!(game.state.pending, None);
    assert_eq!(game.state.turn.priority, Some(1));
}

#[test]
fn malformed_apnap_pending_save_is_rejected() {
    let mut game = fixture_n(210, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);
    let replay = Replay::new(
        210,
        game.state.first_player,
        vec![deck_a(), deck_b(), deck_a()],
        AiSettings::default(),
    )
    .expect("replay shell");
    let save = SessionSave::capture(&game, replay, Vec::new(), false);
    let mut value = serde_json::to_value(&save).expect("save json");
    value["game"]["pending"]["ApnapSacrifice"]["order"]
        .as_array_mut()
        .expect("order array")
        .push(serde_json::json!(0));
    let json = serde_json::to_string(&value).expect("json");
    assert!(
        SessionSave::from_json(&json).is_err(),
        "a malformed saved APNAP queue must be rejected"
    );
}

#[test]
fn apnap_pending_label_and_kind_are_owner_only() {
    let mut game = fixture_n(300, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain])
        .battlefield(1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);
    let owner = game.decision_owner().expect("chooser");
    for viewer in 0..3u8 {
        let view = game.view_for(viewer, &[]);
        if viewer == owner {
            assert_eq!(view.pending_kind, Some(PendingKind::ApnapSacrifice));
            assert!(view.pending.is_some());
        } else {
            assert_eq!(
                view.pending_kind, None,
                "P{viewer} must not see the APNAP kind"
            );
            assert_eq!(view.pending, None, "P{viewer} must not see the APNAP label");
        }
    }
}

#[test]
fn apnap_validation_rejects_rewound_source_and_nonprefix_answers() {
    let mut game = fixture_n(302, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain])
        .battlefield(1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);
    let mut rewound = game.state.clone();
    rewound.stack.last_mut().expect("source").cursor = 0;
    assert!(
        rewound.validate().is_err(),
        "an APNAP queue bound to cursor 0 must be rejected"
    );

    let mut out_of_prefix = game.state.clone();
    if let Some(Pending::ApnapSacrifice {
        player,
        order,
        choices,
        ..
    }) = &mut out_of_prefix.pending
    {
        // Pretend P2 answered while P0 remains first: choices must be a prefix.
        choices.insert(2, None);
        *player = P0;
        *order = vec![P0, 1];
    }
    assert!(
        out_of_prefix.validate().is_err(),
        "an answered seat that is not a prefix must be rejected"
    );
}

#[test]
fn coach_does_not_print_an_unentitled_apnap_choice_label() {
    let mut game = fixture_n(304, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);
    let forest = battlefield_card(&game, P0, CardId::Forest);
    submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(forest),
        },
    );
    let p0 = game.view_for(P0, &[]);
    let explanations = aperture_core::coach::explain(&game, P0, &p0.legal);
    assert!(
        !explanations
            .iter()
            .any(|e| e.text.contains("choose a permanent to sacrifice")),
        "P0 must not receive P1's private APNAP choice label: {explanations:?}"
    );
}

#[test]
fn malformed_apnap_save_with_missing_source_returns_typed_error_not_panic() {
    let mut game = fixture_n(310, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut events = Vec::new();
    cast_to_pending(&mut game, P0, &mut events);
    let source = game.state.resolving.expect("resolving Crack source");
    let replay = Replay::new(
        310,
        game.state.first_player,
        vec![deck_a(), deck_b(), deck_a()],
        AiSettings::default(),
    )
    .expect("replay shell");
    let save = SessionSave::capture(&game, replay, Vec::new(), false);
    assert!(game.state.validate().is_ok(), "valid certified control");
    let mut malformed = save.clone();
    malformed.game.objects.remove(&source);
    let json = serde_json::to_string(&malformed).expect("json");
    let err = std::panic::catch_unwind(|| SessionSave::from_json(&json));
    assert!(
        err.is_ok(),
        "missing APNAP source must not unwind the validator"
    );
    assert!(
        err.unwrap().is_err(),
        "missing APNAP source must return a typed error"
    );
}
