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
