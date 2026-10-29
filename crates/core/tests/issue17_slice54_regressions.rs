//! Native recovery controls for the slice54 working package.
mod common;
use aperture_core::*;
use common::*;

fn combat(seed: u64, copies: usize) -> Game {
    let creatures = vec![CardId::UrsineMonstrosity; copies];
    let mut g = fixture_n(seed, 4)
        .battlefield(P0, &creatures)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 24])
        .library(P1, &[CardId::Forest; 24])
        .library(2, &[CardId::Forest; 24])
        .library(3, &[CardId::Forest; 24])
        .main(P0, Step::PrecombatMain)
        .build();
    for _ in 0..128 {
        if matches!(g.state.pending, Some(Pending::DeclareAttackers { .. })) {
            return g;
        }
        assert!(g.state.pending.is_none());
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    panic!("did not reach attack declaration");
}
fn requirements(g: &Game) -> Vec<(ObjectId, PlayerId)> {
    g.state
        .effects
        .iter()
        .filter_map(|e| {
            (e.kind == ContinuousKind::MustAttackThisCombat).then_some((e.target, e.controller))
        })
        .collect()
}

#[test]
fn wrong_opponent_cannot_evade_one_able_ursine_requirement() {
    let mut g = combat(54101, 1);
    let req = requirements(&g);
    assert_eq!(req.len(), 1);
    let chosen = req[0].1;
    let wrong = (1..4).find(|p| *p != chosen).unwrap();
    let u = req[0].0;
    let before = g.state.clone();
    let refused = g.submit_action(
        P0,
        Action::DeclareAttackers {
            attackers: vec![u],
            defender: Some(wrong),
        },
    );
    assert!(
        refused.is_err(),
        "an able Ursine must attack chosen P{chosen}, not P{wrong}"
    );
    assert_eq!(g.state, before, "refusal must be atomic");
    let offered = g.legal_actions();
    assert!(!offered.is_empty());
    for a in offered {
        assert!(g.validate(P0, &a.action).is_ok());
        assert!(
            matches!(a.action, Action::DeclareAttackers { ref attackers, defender: Some(p) } if p == chosen && attackers.contains(&u))
        );
    }
}

#[test]
fn conflicting_requirements_offer_only_maximum_satisfiable_groups() {
    let mut found_unequal = false;
    for seed in 0..32 {
        let g = combat(seed, 3);
        let req = requirements(&g);
        assert_eq!(req.len(), 3);
        let counts: Vec<_> = (1..4)
            .map(|p| req.iter().filter(|(_, d)| *d == p).count())
            .collect();
        if !counts.contains(&2) || !counts.contains(&1) {
            continue;
        }
        found_unequal = true;
        let max = *counts.iter().max().unwrap();
        let offered = g.legal_actions();
        assert!(!offered.is_empty(), "attack pending must not livelock");
        for a in offered {
            assert!(g.validate(P0, &a.action).is_ok());
            let Action::DeclareAttackers {
                attackers,
                defender: Some(p),
            } = a.action
            else {
                panic!("able required attacker forbids no attack")
            };
            let obeyed = req
                .iter()
                .filter(|(id, d)| *d == p && attackers.contains(id))
                .count();
            assert_eq!(
                obeyed, max,
                "CR508.1d requires maximum requirements, seed={seed}, defender={p}"
            );
        }
    }
    assert!(
        found_unequal,
        "actual seeded triggers must exercise conflicting 2-vs-1 groups"
    );
}

#[test]
fn pump_is_fixed_after_a_genuinely_new_graveyard_type() {
    let mut g = combat(54103, 1);
    let u = requirements(&g)[0].0;
    assert_eq!(
        (g.state.object(u).power(), g.state.object(u).toughness()),
        (4, 4)
    );
    // Controlled fixture addition of a NEW card type (Sorcery), unlike the
    // earlier control which added a Creature to a graveyard already containing one.
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    let mut object = g.state.object(u).clone();
    object.id = id;
    object.card = CardId::Divination;
    object.zone = Zone::Graveyard;
    object.pump_power = 0;
    object.pump_toughness = 0;
    g.state.objects.insert(id, object);
    g.state.players[0].graveyard.push(id);
    let a = g.legal_actions()[0].action.clone();
    submit_ok(&mut g, P0, a);
    assert_eq!(
        (g.state.object(u).power(), g.state.object(u).toughness()),
        (4, 4)
    );
}

#[test]
fn requirement_ends_at_end_combat_but_pump_and_indestructible_do_not() {
    let mut g = combat(54104, 1);
    let u = requirements(&g)[0].0;
    let turn = g.state.turn.number;
    let a = g.legal_actions()[0].action.clone();
    submit_ok(&mut g, P0, a);
    for _ in 0..128 {
        if g.state.turn.step == Step::PostcombatMain {
            break;
        }
        let p = g.decision_owner().unwrap();
        let a = if g.state.pending.is_some() {
            g.legal_actions()[0].action.clone()
        } else {
            Action::Pass
        };
        submit_ok(&mut g, p, a);
    }
    assert_eq!(g.state.turn.step, Step::PostcombatMain);
    assert_eq!(g.state.turn.number, turn);
    assert!(requirements(&g).is_empty());
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| e.target == u && e.kind == ContinuousKind::Indestructible)
    );
    assert_eq!(
        (g.state.object(u).power(), g.state.object(u).toughness()),
        (4, 4)
    );
}

#[test]
fn bounced_source_still_mills_but_does_not_receive_battlefield_effects() {
    let mut g = fixture_n(54105, 4)
        .battlefield(P0, &[CardId::UrsineMonstrosity, CardId::Island])
        .hand(P0, &[CardId::Unsummon])
        .library(P0, &[CardId::Forest; 16])
        .library(P1, &[CardId::Forest; 16])
        .library(2, &[CardId::Forest; 16])
        .library(3, &[CardId::Forest; 16])
        .main(P0, Step::PrecombatMain)
        .build();
    let u = battlefield_card(&g, P0, CardId::UrsineMonstrosity);
    for _ in 0..32 {
        if g.state.turn.step == Step::BeginCombat && !g.state.stack.is_empty() {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(g.state.turn.step, Step::BeginCombat);
    assert_eq!(g.state.stack.len(), 1);
    let bounce = hand_card(&g, P0, CardId::Unsummon);
    let action = Action::CastSpell {
        card: bounce,
        targets: vec![Target::Permanent(u)],
    };
    assert!(g.legal_actions().iter().any(|a| a.action == action));
    submit_ok(&mut g, P0, action);
    let mut events = Vec::new();
    for _ in 0..64 {
        if g.state.stack.is_empty()
            && g.state.trigger_queue.is_empty()
            && g.state.resolving.is_none()
        {
            break;
        }
        let p = g.decision_owner().unwrap();
        events.extend(submit_ok(&mut g, p, Action::Pass));
    }
    assert!(g.state.stack.is_empty());
    assert_eq!(g.state.object(u).zone, Zone::Hand);
    assert!(
        events.iter().any(|e| matches!(e, Event::Milled { .. })),
        "source-left trigger still mills"
    );
    assert!(
        !g.state.effects.iter().any(|e| e.target == u),
        "departed source must not carry a new indestructible grant"
    );
    assert_eq!(
        (g.state.object(u).power(), g.state.object(u).toughness()),
        (3, 3)
    );
}

#[test]
fn tapped_required_attacker_may_decline_and_zero_toughness_still_dies() {
    let mut g = combat(54106, 1);
    let u = requirements(&g)[0].0;
    // Controlled fixture premise: the source was tapped before declaration.
    g.state.objects.get_mut(&u).unwrap().tapped = true;
    let no_attack = Action::DeclareAttackers {
        attackers: vec![],
        defender: None,
    };
    assert!(g.legal_actions().iter().any(|a| a.action == no_attack));
    submit_ok(&mut g, P0, no_attack);
    // A zero-toughness move is not destruction (CR704.5f, 702.12b).
    // Use an explicit 7c fixture effect, stable through layer recomputation.
    let mut effect = g
        .state
        .effects
        .iter()
        .find(|e| e.target == u)
        .unwrap()
        .clone();
    effect.id = g.state.next_effect;
    g.state.next_effect += 1;
    effect.timestamp = g.state.next_timestamp;
    g.state.next_timestamp += 1;
    effect.layer = aperture_core::layer::Layer::Modifying;
    effect.kind = ContinuousKind::ModifyPowerToughness {
        power: 0,
        toughness: -20,
    };
    g.state.effects.push(effect);
    let p = g.decision_owner().unwrap();
    submit_ok(&mut g, p, Action::Pass);
    assert_eq!(g.state.object(u).zone, Zone::Graveyard);
}
