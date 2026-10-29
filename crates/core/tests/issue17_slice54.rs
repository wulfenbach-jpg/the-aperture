//! Slice 54: Ursine Monstrosity combat trigger.
mod common;
use aperture_core::catalog;
#[test]
fn ursine_monstrosity_is_sourced_and_supported() {
    let def = catalog::by_name("Ursine Monstrosity")
        .expect("supported")
        .def();
    assert_eq!(def.cost, Some("{2}{G}"));
    assert_eq!(def.type_line, "Creature — Bear Mutant");
    assert_eq!(def.power, Some(3));
    assert_eq!(def.toughness, Some(3));
    assert_eq!(
        def.oracle,
        "Trample\nAt the beginning of combat on your turn, mill a card and choose an opponent at random. This creature attacks that player this combat if able. Until end of turn, this creature gains indestructible and gets +1/+1 for each card type among cards in your graveyard."
    );
}

use aperture_core::*;
use common::*;
fn c54(n: &str) -> CardId {
    catalog::by_name(n).unwrap()
}
fn grave(g: &mut Game, p: PlayerId, c_: CardId) {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card: c_,
            owner: p,
            controller: p,
            zone: Zone::Graveyard,
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
    g.state.players[p as usize].graveyard.push(id);
}

#[test]
fn begin_combat_mills_forces_attack_and_sets_indestructible_dynamic_body() {
    let mut g = fixture(5401)
        .hand(P0, &[c54("Ursine Monstrosity")])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::GrizzlyBears, CardId::Forest, CardId::Forest])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    grave(&mut g, P0, CardId::Forest);
    grave(&mut g, P0, CardId::Shock);
    let spell = hand_card(&g, P0, c54("Ursine Monstrosity"));
    let cast = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action,Action::CastSpell{card,..} if *card==spell))
        .expect("cast offered")
        .action;
    submit_ok(&mut g, P0, cast);
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g
            .state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::Indestructible),
        128
    ));
    let ursine = battlefield_card(&g, P0, c54("Ursine Monstrosity"));
    assert_eq!(
        g.state.player(P0).graveyard.len(),
        3,
        "mill added a card to two existing"
    );
    assert!(
        g.state
            .player(P0)
            .graveyard
            .iter()
            .any(|id| g.state.object(*id).card == CardId::GrizzlyBears)
    );
    assert_eq!(
        g.state.object(ursine).power(),
        6,
        "three distinct graveyard card types"
    );
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, ContinuousKind::MustAttackThisCombat) && e.controller == P1)
    );
    let first = run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        128,
    );
    assert!(first);
    let wrong = g.submit_action(
        P0,
        Action::DeclareAttackers {
            attackers: Vec::new(),
            defender: None,
        },
    );
    assert!(wrong.is_err(), "must attack the chosen opponent if able");
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![ursine],
            defender: Some(P1),
        },
    );
    // Indestructible replacement through the lethal-damage/SBA path.
    g.state.objects.get_mut(&ursine).unwrap().damage = 99;
    for _ in 0..4 {
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
        if g.state.object(ursine).zone != Zone::Battlefield {
            break;
        }
    }
    assert_eq!(
        g.state.object(ursine).zone,
        Zone::Battlefield,
        "indestructible survives lethal damage"
    );
}

#[test]
fn ursine_effects_expire_and_state_loads_roundtrip() {
    let mut g = fixture(5402)
        .hand(P0, &[c54("Ursine Monstrosity")])
        .battlefield(P0, &[CardId::Forest; 3])
        .library(P0, &[CardId::GrizzlyBears, CardId::Forest])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, c54("Ursine Monstrosity"));
    let cast = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action,Action::CastSpell{card,..} if *card==spell))
        .unwrap()
        .action;
    submit_ok(&mut g, P0, cast);
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g
            .state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::Indestructible),
        128
    ));
    let ursine = battlefield_card(&g, P0, c54("Ursine Monstrosity"));
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
    let loaded = SessionSave::from_json(&save.to_json()).expect("Ursine effect state loads");
    assert_eq!(loaded.game, g.state);
    let turn = g.state.turn.number;
    for _ in 0..1000 {
        if g.state.turn.number != turn {
            break;
        }
        if g.state.pending.is_some() {
            let a = auto_answer(&g);
            let p = g.decision_owner().unwrap();
            submit_ok(&mut g, p, a);
            continue;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(!g.state.effects.iter().any(|e| matches!(
        e.kind,
        ContinuousKind::Indestructible | ContinuousKind::MustAttackThisCombat
    )));
    assert_eq!(g.state.object(ursine).power(), 3);
}

#[test]
fn ursine_trigger_pump_is_fixed_and_affects_toughness() {
    let mut g = fixture(5403)
        .hand(P0, &[c54("Ursine Monstrosity")])
        .battlefield(P0, &[CardId::Forest; 3])
        .library(P0, &[CardId::GrizzlyBears, CardId::Forest])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    grave(&mut g, P0, CardId::Forest);
    grave(&mut g, P0, CardId::Shock);
    let spell = hand_card(&g, P0, c54("Ursine Monstrosity"));
    let cast = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action,Action::CastSpell{card,..} if *card==spell))
        .unwrap()
        .action;
    submit_ok(&mut g, P0, cast);
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g
            .state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::Indestructible),
        128
    ));
    let u = battlefield_card(&g, P0, c54("Ursine Monstrosity"));
    assert_eq!(
        (g.state.object(u).power(), g.state.object(u).toughness()),
        (6, 6)
    );
    // A later graveyard card type does not change the already-applied pump.
    grave(&mut g, P0, CardId::GrizzlyBears);
    let p = g.decision_owner().unwrap();
    submit_ok(&mut g, p, Action::Pass);
    assert_eq!(
        (g.state.object(u).power(), g.state.object(u).toughness()),
        (6, 6)
    );
}
