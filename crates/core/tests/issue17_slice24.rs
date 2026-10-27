//! Slice 24: Surrak and Goreclaw static trample, nontoken entry counter/haste.
mod common;
use aperture_core::catalog;
#[test]
fn surrak_goreclaw_is_sourced_and_supported() {
    let def = catalog::by_name("Surrak and Goreclaw")
        .expect("Surrak and Goreclaw must be supported")
        .def();
    assert_eq!(def.cost, Some("{4}{G}{G}"));
    assert_eq!(def.type_line, "Legendary Creature — Human Bear");
    assert_eq!(def.power, Some(6));
    assert_eq!(def.toughness, Some(5));
    assert_eq!(
        def.oracle,
        "Trample\nOther creatures you control have trample.\nWhenever another nontoken creature you control enters, put a +1/+1 counter on it. It gains haste until end of turn."
    );
}

use aperture_core::*;
use common::*;
fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap()
}
fn land8() -> Vec<CardId> {
    vec![CardId::Forest; 8]
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
fn cast(g: &mut Game, id: ObjectId) {
    let a = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action,Action::CastSpell{card,..} if card==id))
        .expect("offered cast")
        .action;
    submit_ok(g, P0, a);
    drain(g);
}

#[test]
fn surrak_trigger_gives_counter_and_haste_only_to_entered_nontoken_creature() {
    let b = land8();
    let mut g = fixture(2401)
        .hand(P0, &[card("Surrak and Goreclaw"), card("Grizzly Bears")])
        .battlefield(P0, &b)
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let surrak = hand_card(&g, P0, card("Surrak and Goreclaw"));
    cast(&mut g, surrak);
    let surrak = battlefield_card(&g, P0, card("Surrak and Goreclaw"));
    assert_eq!(
        g.state
            .object(surrak)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        0,
        "own entry is not another"
    );
    let bear = hand_card(&g, P0, card("Grizzly Bears"));
    cast(&mut g, bear);
    let bear = battlefield_card(&g, P0, card("Grizzly Bears"));
    assert_eq!(
        g.state
            .object(bear)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        1
    );
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsHaste && e.target == bear)
    );
    assert!(
        g.legal_attackers(P0).contains(&bear),
        "haste allows the summoning-sick creature to attack"
    );
    assert!(g.state.pending.is_none());
    assert!(g.state.stack.is_empty());
    assert!(g.state.trigger_queue.is_empty());
    assert!(g.state.resolving.is_none());
}

#[test]
fn older_creature_does_not_get_haste_and_cleanup_expires_it() {
    let mut b = land8();
    b.push(CardId::GrizzlyBears);
    let mut g = fixture(2402)
        .hand(P0, &[card("Surrak and Goreclaw"), card("Grizzly Bears")])
        .battlefield(P0, &b)
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let old = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let surrak = hand_card(&g, P0, card("Surrak and Goreclaw"));
    cast(&mut g, surrak);
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsHaste && e.target == old)
    );
    let new = hand_card(&g, P0, card("Grizzly Bears"));
    cast(&mut g, new);
    let new = battlefield_card(&g, P0, card("Grizzly Bears"));
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsHaste && e.target == new)
    );
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
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsHaste && e.target == new),
        "cleanup expires haste"
    );
}

fn add_creature(g: &mut Game, p: PlayerId, c: CardId) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card: c,
            owner: p,
            controller: p,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: Default::default(),
        },
    );
    id
}

#[test]
fn static_other_creature_trample_changes_real_combat() {
    let mut b = land8();
    b.push(card("Surrak and Goreclaw"));
    b.push(CardId::GrizzlyBears);
    let mut g = fixture(2403)
        .battlefield(P0, &b)
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let blocker = add_creature(&mut g, P1, CardId::SoldierToken);
    let life = g.state.player(P1).life;
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareBlockers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P1,
        Action::DeclareBlockers {
            blocks: vec![aperture_core::BlockDecl {
                blocker,
                attacker: bear,
            }],
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.player(P1).life < life,
        64
    ));
    assert!(
        g.state.try_object(blocker).is_none(),
        "token blocker ceased to exist"
    );
    assert_eq!(
        g.state.player(P1).life,
        life - 1,
        "trample excess reaches the player"
    );
}

#[test]
fn bounced_entered_creature_loses_old_trigger_and_recast_triggers_once() {
    let mut b = vec![CardId::Forest; 12];
    b.push(CardId::Island);
    b.push(card("Surrak and Goreclaw"));
    let mut g = fixture(2404)
        .hand(P0, &[card("Grizzly Bears"), CardId::Unsummon])
        .battlefield(P0, &b)
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let grizzly = hand_card(&g, P0, card("Grizzly Bears"));
    let cast_action = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action,Action::CastSpell{card,..} if card==grizzly))
        .unwrap()
        .action;
    submit_ok(&mut g, P0, cast_action);
    // Resolve only the creature spell; its entry trigger remains on the stack.
    submit_ok(&mut g, P0, Action::Pass);
    submit_ok(&mut g, P1, Action::Pass);
    assert!(!g.state.trigger_queue.is_empty() || !g.state.stack.is_empty());
    let unsummon = hand_card(&g, P0, CardId::Unsummon);
    let a=g.legal_actions().into_iter().find(|a|matches!(&a.action,Action::CastSpell{card,targets} if *card==unsummon && targets.iter().any(|t|matches!(t,Target::Permanent(id) if *id==grizzly)))).expect("Unsummon offered").action;
    submit_ok(&mut g, P0, a);
    drain(&mut g);
    assert_eq!(g.state.object(grizzly).zone, Zone::Hand);
    let recast = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action,Action::CastSpell{card,..} if card==grizzly))
        .expect("recast offered")
        .action;
    submit_ok(&mut g, P0, recast);
    drain(&mut g);
    assert_eq!(
        g.state
            .object(grizzly)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        1,
        "only the new battlefield entry contributes once"
    );
}

#[test]
fn midtrigger_save_reload_finishes_identically() {
    let b = land8();
    let mut g = fixture(2405)
        .hand(P0, &[card("Surrak and Goreclaw"), card("Grizzly Bears")])
        .battlefield(P0, &b)
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let surrak = hand_card(&g, P0, card("Surrak and Goreclaw"));
    cast(&mut g, surrak);
    let grizzly = hand_card(&g, P0, card("Grizzly Bears"));
    let a = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action,Action::CastSpell{card,..} if card==grizzly))
        .unwrap()
        .action;
    submit_ok(&mut g, P0, a);
    submit_ok(&mut g, P0, Action::Pass);
    submit_ok(&mut g, P1, Action::Pass);
    assert!(
        !g.state.stack.is_empty() || !g.state.trigger_queue.is_empty(),
        "entry trigger is pending"
    );
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
    let loaded = SessionSave::from_json(&save.to_json()).expect("midtrigger save loads");
    let mut loaded = Game::from_state(loaded.game).unwrap();
    assert_eq!(loaded.digest(), g.digest());
    drain(&mut g);
    drain(&mut loaded);
    assert_eq!(loaded.state, g.state);
    assert_eq!(loaded.digest(), g.digest());
    let grizzly = g
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == CardId::GrizzlyBears)
        .unwrap()
        .id;
    assert_eq!(
        g.state
            .object(grizzly)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        1
    );
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsHaste && e.target == grizzly)
    );
}
