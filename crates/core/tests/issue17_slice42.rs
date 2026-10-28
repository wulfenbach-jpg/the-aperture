//! Slice 42: Lightning Greaves / Swiftfoot Boots shared Equipment.
mod common;
use aperture_core::catalog;
#[test]
fn equipment_sources_are_registered_and_castable() {
    for name in ["Lightning Greaves", "Swiftfoot Boots"] {
        let def = catalog::by_name(name).expect("registered equipment").def();
        assert_eq!(def.cost, Some("{2}"));
        assert_eq!(def.type_line, "Artifact — Equipment");
        assert!(def.is_artifact());
        assert!(!def.is_creature());
    }
}

use aperture_core::*;
use common::*;
fn card42(n: &str) -> CardId {
    catalog::by_name(n).unwrap()
}
fn drain(g: &mut Game) {
    for _ in 0..128 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    panic!("settle");
}
fn cast42(g: &mut Game, id: ObjectId) {
    let a = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action,Action::CastSpell{card,..} if *card==id))
        .expect("offered cast")
        .action;
    submit_ok(g, P0, a);
    drain(g);
}
fn equip(g: &mut Game, equipment: ObjectId, target: ObjectId) {
    for _ in 0..8 {
        if g.state.turn.priority == Some(P0) {
            break;
        }
        let p = g.decision_owner().expect("priority");
        submit_ok(g, p, Action::Pass);
    }
    let a=g.legal_actions().into_iter().find(|a|matches!(&a.action,Action::ActivateAbility{source,targets,..} if *source==equipment && targets==&vec![Target::Permanent(target)])).expect("equip offered").action;
    submit_ok(g, P0, a);
    drain(g);
}
fn add_to_hand(g: &mut Game, p: PlayerId, c: CardId) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card: c,
            owner: p,
            controller: p,
            zone: Zone::Hand,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: Default::default(),
            paired_with: None,
        },
    );
    g.state.players[p as usize].hand.push(id);
    id
}

#[test]
fn greaves_equip_grants_haste_and_shroud_to_all_players_targeting() {
    let mut g = fixture(4201)
        .hand(
            P0,
            &[
                card42("Lightning Greaves"),
                card42("Swiftfoot Boots"),
                CardId::Shock,
                CardId::GiantGrowth,
            ],
        )
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::GrizzlyBears,
                CardId::GrizzlyBears,
                CardId::Mountain,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let greaves = hand_card(&g, P0, card42("Lightning Greaves"));
    cast42(&mut g, greaves);
    let boots = hand_card(&g, P0, card42("Swiftfoot Boots"));
    cast42(&mut g, boots);
    let bears: Vec<_> = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::GrizzlyBears)
        .map(|o| o.id)
        .collect();
    g.state.objects.get_mut(&bears[0]).unwrap().summoning_sick = true;
    equip(&mut g, greaves, bears[0]);
    assert!(
        g.legal_attackers(P0).contains(&bears[0]),
        "haste enables attack"
    );
    assert!(g.state.effects.iter().any(|e| matches!(
        e.kind,
        ContinuousKind::Equipped {
            haste: true,
            shroud: true,
            ..
        }
    ) && e.target == bears[0]));
    let shock = hand_card(&g, P0, CardId::Shock);
    let before = g.state.clone();
    assert!(
        g.submit_action(
            P0,
            Action::CastSpell {
                card: shock,
                targets: vec![Target::Permanent(bears[0])]
            }
        )
        .is_err(),
        "shroud refuses owner's spell"
    );
    assert_eq!(g.state, before);
    let growth = hand_card(&g, P0, CardId::GiantGrowth);
    assert!(
        g.submit_action(
            P0,
            Action::CastSpell {
                card: growth,
                targets: vec![Target::Permanent(bears[0])]
            }
        )
        .is_err(),
        "shroud refuses own targeting"
    );
    assert!(!g.legal_actions().iter().any(|a|matches!(&a.action,Action::ActivateAbility{source,targets,..} if *source==boots && targets==&vec![Target::Permanent(bears[0])])),"Boots cannot target a shrouded creature");
    equip(&mut g, greaves, bears[1]);
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, ContinuousKind::Equipped { .. }) && e.target == bears[1])
    );
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, ContinuousKind::Equipped { .. }) && e.target == bears[0]),
        "moving Equipment ends old attachment"
    );
}

#[test]
fn boots_hexproof_allows_owner_but_refuses_opponent_and_grants_haste() {
    let mut g = fixture(4202)
        .hand(P0, &[card42("Swiftfoot Boots")])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::GrizzlyBears,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Mountain; 2])
        .main(P0, Step::PrecombatMain)
        .build();
    let boots = hand_card(&g, P0, card42("Swiftfoot Boots"));
    cast42(&mut g, boots);
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    g.state.objects.get_mut(&bear).unwrap().summoning_sick = true;
    equip(&mut g, boots, bear);
    assert!(
        g.legal_attackers(P0).contains(&bear),
        "haste for summoning-sick creature"
    );
    // Owner may target with own Boots/equip target and own spell; opponent may not.
    let growth = add_to_hand(&mut g, P0, CardId::GiantGrowth);
    assert!(
        g.validate(
            P0,
            &Action::CastSpell {
                card: growth,
                targets: vec![Target::Permanent(bear)]
            }
        )
        .is_ok(),
        "hexproof allows controller"
    );
    let shock = add_to_hand(&mut g, P1, CardId::Shock);
    assert!(
        g.submit_action(
            P1,
            Action::CastSpell {
                card: shock,
                targets: vec![Target::Permanent(bear)]
            }
        )
        .is_err(),
        "hexproof refuses opponent"
    );
    // Player-targeting is unaffected by creature hexproof.
    let shock_targets = g.legal_targets(P1, TargetSpec::AnyTarget, shock);
    assert!(shock_targets.contains(&Target::Player(P0)));
    assert!(!shock_targets.contains(&Target::Permanent(bear)));
}

#[test]
fn old_attachment_survives_until_equip_resolves_and_bounce_does_not_transfer() {
    let mut g = fixture(4203)
        .hand(P0, &[card42("Swiftfoot Boots"), CardId::Unsummon])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Island,
                CardId::Island,
                CardId::GrizzlyBears,
                CardId::GrizzlyBears,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let boots = hand_card(&g, P0, card42("Swiftfoot Boots"));
    cast42(&mut g, boots);
    let bears: Vec<_> = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::GrizzlyBears)
        .map(|o| o.id)
        .collect();
    equip(&mut g, boots, bears[0]);
    let action=g.legal_actions().into_iter().find(|a|matches!(&a.action,Action::ActivateAbility{source,targets,..} if *source==boots && targets==&vec![Target::Permanent(bears[1])])).unwrap().action;
    submit_ok(&mut g, P0, action);
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, ContinuousKind::Equipped { .. }) && e.target == bears[0]),
        "old attachment stays while equip is on stack"
    );
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, ContinuousKind::Equipped { .. }) && e.target == bears[1])
    );
    drain(&mut g);
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, ContinuousKind::Equipped { .. }) && e.target == bears[1])
    );
    let unsummon = hand_card(&g, P0, CardId::Unsummon);
    assert!(
        g.submit_action(
            P0,
            Action::CastSpell {
                card: unsummon,
                targets: vec![Target::Permanent(bears[1])]
            }
        )
        .is_ok()
    );
    drain(&mut g);
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, ContinuousKind::Equipped { .. }) && e.target == bears[1]),
        "bounce ends attachment"
    );
}

#[test]
fn attached_and_mid_equip_save_reload_have_identical_continuation() {
    let mut g = fixture(4204)
        .hand(P0, &[card42("Swiftfoot Boots")])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::GrizzlyBears,
                CardId::GrizzlyBears,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let boots = hand_card(&g, P0, card42("Swiftfoot Boots"));
    cast42(&mut g, boots);
    let bears: Vec<_> = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::GrizzlyBears)
        .map(|o| o.id)
        .collect();
    equip(&mut g, boots, bears[0]);
    let action=g.legal_actions().into_iter().find(|a|matches!(&a.action,Action::ActivateAbility{source,targets,..} if *source==boots && targets==&vec![Target::Permanent(bears[1])])).unwrap().action;
    submit_ok(&mut g, P0, action);
    assert!(!g.state.stack.is_empty(), "equip ability is on stack");
    let decks = vec![
        DeckList::parse_practice("A", "40 Forest\n").unwrap(),
        DeckList::parse_practice("B", "40 Forest\n").unwrap(),
    ];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        decks,
        AiSettings::default(),
    )
    .unwrap();
    let save = SessionSave::capture(&g, replay, Vec::new(), true);
    let loaded = SessionSave::from_json(&save.to_json()).expect("mid-equip save loads");
    let mut loaded = Game::from_state(loaded.game).unwrap();
    assert_eq!(loaded.digest(), g.digest());
    drain(&mut g);
    drain(&mut loaded);
    assert_eq!(loaded.state, g.state);
    assert_eq!(loaded.digest(), g.digest());
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, ContinuousKind::Equipped { .. }) && e.target == bears[1])
    );
}
