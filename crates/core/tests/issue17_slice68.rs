//! Slice 68: Skirk Prospector's nonstack Goblin mana ability and Zo-Zu's
//! all-player land-entry damage.
mod common;
use aperture_core::*;
use common::*;

#[test]
fn skirk_sacrifice_is_immediate_mana_and_pays_a_real_spell() {
    let mut g = fixture(6801)
        .battlefield(P0, &[CardId::SkirkProspector, CardId::GoblinToken])
        .hand(P0, &[CardId::GoblinWarStrike])
        .library(P0, &[CardId::Mountain; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let skirk = battlefield_card(&g, P0, CardId::SkirkProspector);
    let token = battlefield_card(&g, P0, CardId::GoblinToken);
    let action = Action::ActivateSacrificeManaAbility {
        source: skirk,
        sacrifice: token,
    };
    assert!(
        g.legal_actions().iter().any(|x| x.action == action),
        "the offered Goblin sacrifice is listed"
    );
    let before_life = g.state.player(P1).life;
    let events = submit_ok(&mut g, P0, action);
    assert!(g.state.stack.is_empty(), "mana ability uses no stack");
    assert_eq!(g.state.player(P0).mana.red, 1, "red mana added immediately");
    assert!(g.state.pending.is_none(), "no response window");
    assert_eq!(g.state.turn.priority, Some(P0));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::ManaAdded {
            color: ManaColor::Red,
            amount: 1,
            ..
        }
    )));
    let spell = hand_card(&g, P0, CardId::GoblinWarStrike);
    let log = submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: spell,
            targets: vec![Target::Player(P1)],
        },
    );
    assert!(log.iter().any(|e| matches!(
        e,
        Event::Cast {
            card: CardId::GoblinWarStrike,
            ..
        }
    )));
    for _ in 0..10 {
        if g.state.stack.is_empty() && g.state.pending.is_none() {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(
        g.state.player(P1).life < before_life,
        "the new mana paid a real spell"
    );
}

#[test]
fn skirk_offers_each_legal_goblin_but_refuses_wrong_costs_atomically() {
    let mut g = fixture(6802)
        .battlefield(
            P0,
            &[
                CardId::SkirkProspector,
                CardId::GrizzlyBears,
                CardId::GoblinToken,
            ],
        )
        .battlefield(P1, &[CardId::GoblinToken])
        .main(P0, Step::PrecombatMain)
        .build();
    let skirk = battlefield_card(&g, P0, CardId::SkirkProspector);
    let goblin = battlefield_card(&g, P0, CardId::GoblinToken);
    let non_goblin = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let opponent = battlefield_card(&g, P1, CardId::GoblinToken);
    let offers: Vec<Action> = g.legal_actions().into_iter().map(|x| x.action).collect();
    assert!(offers.contains(&Action::ActivateSacrificeManaAbility {
        source: skirk,
        sacrifice: skirk
    }));
    assert!(offers.contains(&Action::ActivateSacrificeManaAbility {
        source: skirk,
        sacrifice: goblin
    }));
    let before = g.state.clone();
    assert!(
        g.submit_action(
            P0,
            Action::ActivateSacrificeManaAbility {
                source: skirk,
                sacrifice: non_goblin
            }
        )
        .is_err(),
        "non-Goblin sacrifice refused"
    );
    assert_eq!(g.state, before, "refusal is atomic");
    assert!(
        g.submit_action(
            P0,
            Action::ActivateSacrificeManaAbility {
                source: skirk,
                sacrifice: opponent
            }
        )
        .is_err(),
        "opponent's Goblin refused"
    );
    assert_eq!(g.state, before);
    // Tapped/sick Goblins remain legal: no tap cost and no summoning restriction.
    object_mut(&mut g.state, goblin).tapped = true;
    object_mut(&mut g.state, goblin).summoning_sick = true;
    let action = Action::ActivateSacrificeManaAbility {
        source: skirk,
        sacrifice: goblin,
    };
    assert!(g.legal_actions().iter().any(|x| x.action == action));
    submit_ok(&mut g, P0, action);
    assert_eq!(g.state.player(P0).mana.red, 1);
}

#[test]
fn skirk_death_triggers_stage_after_mana_and_pending_save_roundtrips() {
    let mut g = fixture(6803)
        .battlefield(
            P0,
            &[
                CardId::SkirkProspector,
                CardId::PashalikMons,
                CardId::GoblinToken,
            ],
        )
        .library(P0, &[CardId::Mountain; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let skirk = battlefield_card(&g, P0, CardId::SkirkProspector);
    let token = battlefield_card(&g, P0, CardId::GoblinToken);
    let action = Action::ActivateSacrificeManaAbility {
        source: skirk,
        sacrifice: token,
    };
    assert!(g.legal_actions().iter().any(|x| x.action == action));
    submit_ok(&mut g, P0, action);
    assert_eq!(g.state.player(P0).mana.red, 1);
    assert!(
        matches!(g.state.pending, Some(Pending::TriggerTarget { .. })),
        "Pashalik trigger waits for a target"
    );
    assert_eq!(
        g.state.turn.priority, None,
        "no priority while trigger target pending"
    );
    assert!(g.state.validate().is_ok());
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .unwrap();
    let save = SessionSave::capture(&g, replay, Vec::new(), true);
    assert!(
        SessionSave::from_json(&save.to_json()).is_ok(),
        "pending mana checkpoint loads"
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
        .expect("P1 target");
    submit_ok(&mut g, P0, choose);
}

#[test]
fn zozu_damages_the_entering_lands_controller_including_opponent_and_source_left() {
    let mut g = fixture(6804)
        .battlefield(P0, &[CardId::ZoZuThePunisher])
        .battlefield(P1, &[CardId::Mountain])
        .hand(P1, &[CardId::Forest])
        .library(P0, &[CardId::Mountain; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P1, Step::PrecombatMain)
        .build();
    let land = hand_card(&g, P1, CardId::Forest);
    let before = g.state.player(P1).life;
    let events = submit_ok(&mut g, P1, Action::PlayLand { card: land });
    assert!(
        events.iter().any(|e| matches!(e, Event::LandPlayed { .. })),
        "land actually entered"
    );
    for _ in 0..12 {
        if g.state.stack.is_empty() && g.state.pending.is_none() {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(
        g.state.player(P1).life,
        before - 2,
        "opponent's land controller is damaged"
    );
    assert_eq!(
        g.state.player(P0).life,
        20,
        "Zo-Zu's controller is not the damaged player"
    );
}

#[test]
fn zozu_does_not_trigger_on_a_creature_entering() {
    let mut g = fixture(6805)
        .battlefield(P0, &[CardId::ZoZuThePunisher])
        .hand(P1, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::Mountain; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P1, Step::PrecombatMain)
        .build();
    let bears = hand_card(&g, P1, CardId::GrizzlyBears);
    let before = g.state.player(P1).life;
    submit_ok(
        &mut g,
        P1,
        Action::CastSpell {
            card: bears,
            targets: Vec::new(),
        },
    );
    for _ in 0..12 {
        if g.state.stack.is_empty() && g.state.pending.is_none() {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(
        g.state.player(P1).life,
        before,
        "creature entry is not a land entry"
    );
}

fn drain_all(g: &mut Game, events: &mut Vec<Event>) {
    for _ in 0..100 {
        if g.state.stack.is_empty()
            && g.state.trigger_queue.is_empty()
            && g.state.pending.is_none()
            && g.state.resolving.is_none()
        {
            return;
        }
        let p = g.decision_owner().expect("owner during drain");
        let action = g
            .legal_actions()
            .into_iter()
            .map(|x| x.action)
            .find(|a| {
                matches!(
                    a,
                    Action::ChooseTriggerTarget {
                        target: Target::Player(P1)
                    }
                )
            })
            .unwrap_or(Action::Pass);
        assert!(g.legal_actions().iter().any(|x| x.action == action));
        events.extend(g.submit_action(p, action).unwrap());
        g.state.validate().unwrap();
    }
    panic!("drain cap");
}

#[test]
fn skirk_is_not_vanilla_under_ruxa_and_is_not_a_graveyard_target() {
    let mut g = fixture(6810)
        .battlefield(
            P0,
            &[
                CardId::SkirkProspector,
                CardId::GrizzlyBears,
                CardId::RuxaPatientProfessor,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let skirk = battlefield_card(&g, P0, CardId::SkirkProspector);
    assert!(
        !g.state.object_has_no_abilities(skirk),
        "Skirk has its printed activated mana ability, including under Ruxa"
    );
    submit_ok(&mut g, P0, Action::Pass);
    assert_eq!(
        g.state.object(skirk).power(),
        1,
        "Ruxa does not anthem Skirk"
    );
    if let Some(object) = g.state.objects.get_mut(&skirk) {
        object.zone = Zone::Graveyard;
    }
    g.state.players[P0 as usize].graveyard.push(skirk);
    assert!(
        !g.state.object_has_no_abilities(skirk),
        "the printed mana ability still excludes it from no-abilities graveyard targeting"
    );
}

#[test]
fn nonactive_sacrifice_pending_load_continues_identically() {
    let mut g = fixture(6811)
        .battlefield(
            P0,
            &[
                CardId::SkirkProspector,
                CardId::PashalikMons,
                CardId::GoblinToken,
            ],
        )
        .library(P0, &[CardId::Mountain; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P1, Step::PrecombatMain)
        .build();
    submit_ok(&mut g, P1, Action::Pass);
    assert_eq!(g.state.turn.priority, Some(P0));
    let source = battlefield_card(&g, P0, CardId::SkirkProspector);
    let sacrifice = battlefield_card(&g, P0, CardId::GoblinToken);
    let action = Action::ActivateSacrificeManaAbility { source, sacrifice };
    assert!(g.legal_actions().iter().any(|x| x.action == action));
    let mut events = submit_ok(&mut g, P0, action);
    let mana = events
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::ManaAdded {
                    player: P0,
                    amount: 1,
                    ..
                }
            )
        })
        .unwrap();
    assert!(
        events
            .iter()
            .take(mana)
            .all(|e| !matches!(e, Event::Priority { .. } | Event::TriggerStaged { .. })),
        "mana resolves before response and trigger staging"
    );
    assert_eq!(g.state.player(P0).mana.red, 1);
    assert_eq!(g.state.turn.priority, None);
    g.state.validate().unwrap();
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .unwrap();
    let save = SessionSave::capture(&g, replay, Vec::new(), true);
    let loaded = SessionSave::from_json(&save.to_json()).unwrap();
    let mut copy = Game::from_state(loaded.game).unwrap();
    for _ in 0..100 {
        if g.state.stack.is_empty()
            && g.state.trigger_queue.is_empty()
            && g.state.pending.is_none()
            && g.state.resolving.is_none()
        {
            break;
        }
        let p = g.decision_owner().unwrap();
        let a = g
            .legal_actions()
            .into_iter()
            .map(|x| x.action)
            .find(|a| {
                matches!(
                    a,
                    Action::ChooseTriggerTarget {
                        target: Target::Player(P1)
                    }
                )
            })
            .unwrap_or(Action::Pass);
        assert!(copy.legal_actions().iter().any(|x| x.action == a));
        let first_choice = matches!(a, Action::ChooseTriggerTarget { .. });
        let e = g.submit_action(p, a.clone()).unwrap();
        if first_choice {
            assert_eq!(
                g.state.turn.priority,
                Some(P0),
                "the nonactive mana activator regains priority after target staging"
            );
        }
        let f = copy.submit_action(p, a).unwrap();
        assert_eq!(e, f);
        events.extend(e);
        assert_eq!(g.state, copy.state);
        assert_eq!(g.digest(), copy.digest());
        assert_eq!(g.revision(), copy.revision());
        g.state.validate().unwrap();
    }
    assert!(g.state.stack.is_empty() && g.state.pending.is_none() && g.state.resolving.is_none());
    assert_eq!(g.state.player(P1).life, 19);
    assert_eq!(
        g.state.turn.priority,
        Some(P1),
        "after the triggered ability resolves the active player gets priority, CR117.3b"
    );
}

#[test]
fn zozu_bounced_source_still_uses_its_controller_for_damage() {
    let mut g = fixture(6812)
        .battlefield(P0, &[CardId::ZoZuThePunisher, CardId::AngrathsMarauders])
        .hand(P0, &[CardId::Unsummon])
        .hand(P1, &[CardId::Forest])
        .mana(
            P0,
            ManaPool {
                blue: 1,
                ..ManaPool::default()
            },
        )
        .library(P0, &[CardId::Mountain; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P1, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&g, P0, CardId::ZoZuThePunisher);
    let land = hand_card(&g, P1, CardId::Forest);
    let mut events = submit_ok(&mut g, P1, Action::PlayLand { card: land });
    submit_ok(&mut g, P1, Action::Pass);
    let spell = hand_card(&g, P0, CardId::Unsummon);
    let action = Action::CastSpell {
        card: spell,
        targets: vec![Target::Permanent(source)],
    };
    assert!(g.legal_actions().iter().any(|x| x.action == action));
    events.extend(submit_ok(&mut g, P0, action));
    drain_all(&mut g, &mut events);
    assert_eq!(g.state.object(source).zone, Zone::Hand);
    assert_eq!(
        g.state.player(P1).life,
        16,
        "departed P0 source still doubles through P0's Angrath's Marauders"
    );
    assert_eq!(g.state.player(P0).life, 20);
}

#[test]
fn skirk_can_sacrifice_itself_while_sick_and_tapped() {
    let mut g = fixture(6813)
        .battlefield_sick(P0, &[CardId::SkirkProspector])
        .main(P0, Step::PrecombatMain)
        .build();
    let skirk = battlefield_card(&g, P0, CardId::SkirkProspector);
    object_mut(&mut g.state, skirk).tapped = true;
    let action = Action::ActivateSacrificeManaAbility {
        source: skirk,
        sacrifice: skirk,
    };
    assert!(g.legal_actions().iter().any(|x| x.action == action));
    submit_ok(&mut g, P0, action);
    assert_eq!(g.state.player(P0).mana.red, 1);
    assert_eq!(g.state.object(skirk).zone, Zone::Graveyard);
}

#[test]
fn zozu_trigger_captures_a_valid_affected_player_and_rejects_malformed_saves() {
    let mut g = fixture(6814)
        .battlefield(P0, &[CardId::ZoZuThePunisher])
        .hand(P1, &[CardId::Forest])
        .library(P0, &[CardId::Mountain; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P1, Step::PrecombatMain)
        .build();
    let land = hand_card(&g, P1, CardId::Forest);
    submit_ok(&mut g, P1, Action::PlayLand { card: land });
    assert!(matches!(
        g.state.stack.last().and_then(|item| item.affected_player),
        Some(P1)
    ));
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .unwrap();
    let valid = SessionSave::capture(&g, replay.clone(), Vec::new(), true);
    assert!(SessionSave::from_json(&valid.to_json()).is_ok());
    let mut missing = valid.clone();
    missing.game.stack.last_mut().unwrap().affected_player = None;
    assert!(SessionSave::from_json(&missing.to_json()).is_err());
    let mut out_of_range = valid.clone();
    out_of_range.game.stack.last_mut().unwrap().affected_player = Some(9);
    assert!(SessionSave::from_json(&out_of_range.to_json()).is_err());
}

#[test]
fn zozu_damages_its_own_controller_for_their_land_play() {
    let mut g = fixture(6815)
        .battlefield(P0, &[CardId::ZoZuThePunisher])
        .hand(P0, &[CardId::Forest])
        .library(P0, &[CardId::Mountain; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let land = hand_card(&g, P0, CardId::Forest);
    submit_ok(&mut g, P0, Action::PlayLand { card: land });
    for _ in 0..12 {
        if g.state.stack.is_empty() && g.state.pending.is_none() {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(g.state.player(P0).life, 18);
    assert_eq!(g.state.player(P1).life, 20);
}
