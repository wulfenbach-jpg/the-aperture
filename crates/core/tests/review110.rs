use aperture_core::*;

#[test]
fn ordinary_pending_scry_save_roundtrip() {
    use aperture_core::{AiSettings, Replay, SessionSave};
    let deck = DeckList::parse_practice("ordinary", "20 Island\n20 Opt\n").unwrap();
    let decks = vec![deck.clone(), deck];
    let (mut g, mut log) = Game::new_logged(11001, &decks).unwrap();
    let mut replay =
        Replay::new(11001, g.state.first_player, decks, AiSettings::default()).unwrap();
    for _ in 0..200 {
        if matches!(g.state.pending, Some(Pending::OrderCard { .. })) {
            assert_eq!(replay.run().unwrap().0.digest(), g.digest());
            let save = SessionSave::capture(&g, replay, log, false);
            SessionSave::from_json(&save.to_json())
                .expect("real ordinary pending scry save must load");
            for case in [
                "unanswered-kept",
                "wrong-mode",
                "excess-viewed",
                "no-resolver",
                "duplicate-viewed",
                "foreign-to-top",
            ] {
                let mut value = serde_json::to_value(&save).unwrap();
                value["sandbox_modified"] = serde_json::json!(true);
                match case {
                    "unanswered-kept" => {
                        let pending = &mut value["game"]["pending"]["OrderCard"];
                        let current = pending["viewed"][0].clone();
                        pending["to_top"] = serde_json::json!([current]);
                    }
                    "wrong-mode" => {
                        value["game"]["pending"]["OrderCard"]["mode"] =
                            serde_json::json!("Surveil");
                    }
                    "excess-viewed" => {
                        let id = g.state.players[P0 as usize].library[0];
                        value["game"]["pending"]["OrderCard"]["viewed"]
                            .as_array_mut()
                            .unwrap()
                            .push(serde_json::json!(id));
                    }
                    "no-resolver" => {
                        value["game"]["resolving"] = serde_json::Value::Null;
                    }
                    "duplicate-viewed" => {
                        let viewed = value["game"]["pending"]["OrderCard"]["viewed"]
                            .as_array_mut()
                            .unwrap();
                        let first = viewed[0].clone();
                        viewed.push(first);
                    }
                    "foreign-to-top" => {
                        value["game"]["pending"]["OrderCard"]["to_top"] =
                            serde_json::json!([ObjectId(999_999)]);
                    }
                    _ => unreachable!(),
                }
                let state: GameState = serde_json::from_value(value["game"].clone()).unwrap();
                value["digest"] = serde_json::json!(aperture_core::digest::digest(&state));
                SessionSave::from_json(&value.to_string())
                    .expect_err(&format!("{case} must be rejected with a recomputed digest"));
            }
            return;
        }
        let action = if matches!(g.state.pending, Some(Pending::Mulligan { .. })) {
            Action::Mulligan { keep: true }
        } else {
            let legal = g.legal_actions();
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
        let p = g.decision_owner().unwrap();
        let sub = Submission {
            player: p,
            revision: g.revision(),
            decision: g.decision(),
            action,
        };
        let events = g.submit(&sub).unwrap();
        replay.record(ReplayStep {
            player: sub.player,
            revision: sub.revision,
            decision: sub.decision,
            action: sub.action.clone(),
        });
        log.extend(events);
    }
    panic!("failed to reach scry");
}

#[test]
fn commander_surveilled_to_graveyard_does_not_panic_and_resolution_continues() {
    let deck = DeckList::parse_practice("fixture", "40 Forest\n").unwrap();
    let mut g = Game::new_command_zone(
        11002,
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
    let island = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        island,
        aperture_core::state::Object {
            id: island,
            card: CardId::Island,
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
    let consider = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        consider,
        aperture_core::state::Object {
            id: consider,
            card: CardId::Consider,
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
    g.state.players[0].hand.push(consider);
    g.state.turn.active = P0;
    g.state.turn.step = Step::PrecombatMain;
    g.state.turn.priority = Some(P0);
    g.state.validate().unwrap();
    g.submit_action(
        P0,
        Action::CastSpell {
            card: consider,
            targets: vec![],
        },
    )
    .unwrap();
    pass_both(&mut g);
    assert!(matches!(g.state.pending, Some(Pending::OrderCard { .. })));
    g.submit_action(P0, Action::OrderCard { to_top: false })
        .expect("surveil commander without stale-id panic");
    // Consider's draw and the subsequent commander-return choice still resolve.
    for _ in 0..8 {
        if let Some(Pending::CommanderReturn { .. }) = g.state.pending {
            break;
        }
        if g.state.pending.is_none() && g.state.stack.is_empty() {
            break;
        }
        let p = g.decision_owner().unwrap();
        let action = match g.state.pending {
            Some(Pending::OrderCard { .. }) => Action::OrderCard { to_top: false },
            _ => Action::Pass,
        };
        g.submit_action(p, action).unwrap();
    }
    g.state.validate().unwrap();
}

fn pass_both(g: &mut Game) {
    for _ in 0..8 {
        if matches!(g.state.pending, Some(Pending::OrderCard { .. })) {
            return;
        }
        if g.state.pending.is_none() && g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        g.submit_action(p, Action::Pass).unwrap();
    }
    panic!("resolution did not reach the surveil choice");
}

fn add_obj(g: &mut Game, p: PlayerId, card: CardId, zone: Zone) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card,
            owner: p,
            controller: p,
            zone,
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
    match zone {
        Zone::Hand => g.state.players[p as usize].hand.push(id),
        Zone::Battlefield
        | Zone::Library
        | Zone::Graveyard
        | Zone::Exile
        | Zone::Command
        | Zone::Stack
        | Zone::Removed => {}
    }
    id
}

#[test]
fn real_time_ebb_decline_then_consider_surveils_the_commander() {
    let deck = DeckList::parse_practice("fixture", "40 Forest\n").unwrap();
    let decks = [deck.clone(), deck.clone()];
    let mut g = Game::new_command_zone(
        11003,
        &decks,
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .unwrap();
    g.state.mulligan = None;
    g.state.pending = None;
    g.state.turn.active = P0;
    g.state.turn.step = Step::PrecombatMain;
    g.state.turn.priority = Some(P0);
    add_obj(&mut g, P0, CardId::Plains, Zone::Battlefield);
    let commander = g.state.players[0].command_zone[0];
    g.submit_action(
        P0,
        Action::CastSpell {
            card: commander,
            targets: vec![],
        },
    )
    .unwrap();
    pass_pending(&mut g);
    let battlefield = g.state.commanders[0].object;
    for _ in 0..3 {
        add_obj(&mut g, P0, CardId::Island, Zone::Battlefield);
    }
    let ebb = add_obj(&mut g, P0, CardId::TimeEbb, Zone::Hand);
    g.submit_action(
        P0,
        Action::CastSpell {
            card: ebb,
            targets: vec![Target::Permanent(battlefield)],
        },
    )
    .unwrap();
    pass_pending(&mut g);
    g.submit_action(
        P0,
        Action::AnswerCommanderReplacement {
            to_command_zone: false,
        },
    )
    .unwrap();
    assert_eq!(g.state.commanders[0].object, g.state.players[0].library[0]);
    // Real main phase, real Consider; surveil the commander into the graveyard.
    g.state.turn.step = Step::PrecombatMain;
    g.state.turn.priority = Some(P0);
    g.state.turn.passed = 0;
    add_obj(&mut g, P0, CardId::Island, Zone::Battlefield);
    let consider = add_obj(&mut g, P0, CardId::Consider, Zone::Hand);
    let tax_before = g.state.commanders[0].casts_from_command_zone;
    g.submit_action(
        P0,
        Action::CastSpell {
            card: consider,
            targets: vec![],
        },
    )
    .unwrap();
    pass_pending(&mut g);
    assert!(matches!(g.state.pending, Some(Pending::OrderCard { .. })));
    g.submit_action(P0, Action::OrderCard { to_top: false })
        .unwrap();
    let grave = g.state.commanders[0].object;
    assert_eq!(g.state.object(grave).zone, Zone::Graveyard);
    assert!(!g.state.players[0].hand.iter().all(|id| *id == commander));
    assert!(matches!(
        g.state.pending,
        Some(Pending::CommanderReturn { .. })
    ));
    g.submit_action(
        P0,
        Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    )
    .unwrap();
    let command = g.state.commanders[0].object;
    assert_eq!(g.state.object(command).zone, Zone::Command);
    assert_eq!(g.state.commanders[0].casts_from_command_zone, tax_before);
    assert_eq!(g.state.players[0].command_zone, vec![command]);
    g.state.validate().unwrap();
}

fn pass_pending(g: &mut Game) {
    for _ in 0..10 {
        if matches!(
            g.state.pending,
            Some(Pending::OrderCard { .. } | Pending::CommanderReplacement { .. })
        ) {
            return;
        }
        if g.state.pending.is_none() && g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        g.submit_action(p, Action::Pass).unwrap();
    }
}
