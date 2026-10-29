//! Native coordinator additions for slice 42: atomic equip refusals and live
//! target protection.
//!
//! These tests are authored by the integration owner (the same run that fixed
//! the animated-permanent defect and that wrote this file), not by the saved
//! writer, and they are not an independent review.
mod common;

use aperture_core::state::Object;
use aperture_core::*;
use common::*;

fn greaves() -> CardId {
    catalog::by_name("Lightning Greaves").unwrap()
}

fn boots() -> CardId {
    catalog::by_name("Swiftfoot Boots").unwrap()
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
        .find(|a| matches!(&a.action, Action::CastSpell { card, .. } if *card == id))
        .expect("offered cast")
        .action;
    submit_ok(g, P0, a);
    drain(g);
}

fn equip(g: &mut Game, source: ObjectId, target: ObjectId) {
    let a = g
        .legal_actions()
        .into_iter()
        .find(|a| {
            matches!(&a.action, Action::ActivateAbility { source: s, targets, .. }
            if *s == source && targets == &vec![Target::Permanent(target)])
        })
        .expect("equip offered")
        .action;
    submit_ok(g, P0, a);
    drain(g);
}

fn attached_to(g: &Game, target: ObjectId) -> usize {
    g.state
        .effects
        .iter()
        .filter(|e| matches!(e.kind, ContinuousKind::Equipped { .. }) && e.target == target)
        .count()
}

fn put_in_hand(g: &mut Game, p: PlayerId, card: CardId) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        Object {
            id,
            card,
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

fn game(seed: u64) -> Game {
    fixture(seed)
        .hand(P0, &[greaves(), boots()])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::GrizzlyBears,
                CardId::GrizzlyBears,
            ],
        )
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Mountain; 10])
        .mana(
            P0,
            ManaPool {
                colorless: 10,
                green: 10,
                ..ManaPool::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build()
}

/// `ActivateAbility` carries no cost field: the cost is the card's printed one,
/// so a refused activation can never be a payment the caller invented.
fn activate(source: ObjectId, ability: u8, mode: u8, target: ObjectId) -> Action {
    Action::ActivateAbility {
        source,
        ability,
        mode,
        targets: vec![Target::Permanent(target)],
    }
}

#[test]
fn equip_refusals_are_atomic_for_control_target_and_cost() {
    let mut g = game(425001);
    let gv = hand_card(&g, P0, greaves());
    cast(&mut g, gv);
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let legal = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action, Action::ActivateAbility { source, .. } if *source == gv))
        .expect("equip offered in a main phase")
        .action;
    let Action::ActivateAbility { ability, mode, .. } = legal else {
        unreachable!("equip is an activated ability")
    };

    // CR 602.2: only the Equipment's controller may activate it.
    let before = g.state.clone();
    assert!(
        g.submit_action(P1, activate(gv, ability, mode, bear))
            .is_err(),
        "a non-controller cannot activate equip"
    );
    assert_eq!(g.state, before, "control refusal leaves state untouched");
    assert_eq!(attached_to(&g, bear), 0);

    // CR 301.5: a noncreature permanent is not an equip target.
    let forest = battlefield_card(&g, P0, CardId::Forest);
    let before = g.state.clone();
    assert!(
        g.submit_action(P0, activate(gv, ability, mode, forest))
            .is_err(),
        "a land is not a creature to equip"
    );
    assert_eq!(g.state, before, "target refusal leaves state untouched");

    // The Equipment itself is not "a creature you control".
    let before = g.state.clone();
    assert!(
        g.submit_action(P0, activate(gv, ability, mode, gv))
            .is_err(),
        "Equipment cannot equip itself"
    );
    assert_eq!(g.state, before);

    // CR 118.3: Boots' {1} cannot be paid from an empty pool with no untapped
    // mana source either, and a refused activation attaches nothing.
    let mut g = fixture(425002)
        .hand(P0, &[boots()])
        .battlefield(P0, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Mountain; 10])
        .mana(
            P0,
            ManaPool {
                colorless: 2,
                ..ManaPool::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let bt = hand_card(&g, P0, boots());
    cast(&mut g, bt);
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    // Read the real offer while {1} is payable, then remove the mana: the same
    // action must now be refused rather than attaching for free.
    g.state.players[P0 as usize].mana = ManaPool {
        colorless: 1,
        ..ManaPool::default()
    };
    let offered = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action, Action::ActivateAbility { source, .. } if *source == bt))
        .expect("equip offered while {1} is payable")
        .action;
    let Action::ActivateAbility { ability, mode, .. } = offered else {
        unreachable!()
    };
    g.state.players[P0 as usize].mana = ManaPool::default();
    let before = g.state.clone();
    assert!(
        g.submit_action(P0, activate(bt, ability, mode, bear))
            .is_err(),
        "an unpayable equip cost is refused"
    );
    assert_eq!(g.state, before, "cost refusal leaves state untouched");
    assert_eq!(attached_to(&g, bear), 0);
}

#[test]
fn equip_is_refused_while_a_spell_is_on_the_stack() {
    let mut g = game(425003);
    let gv = hand_card(&g, P0, greaves());
    cast(&mut g, gv);
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let legal = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action, Action::ActivateAbility { source, .. } if *source == gv))
        .expect("equip offered with an empty stack")
        .action;
    let Action::ActivateAbility { ability, mode, .. } = legal else {
        unreachable!()
    };

    // Put a real spell on the stack through an offered cast, then try to equip.
    let second = put_in_hand(&mut g, P0, boots());
    let cast_boots = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action, Action::CastSpell { card, .. } if *card == second))
        .expect("boots cast offered")
        .action;
    submit_ok(&mut g, P0, cast_boots);
    assert!(!g.state.stack.is_empty(), "a spell is on the stack");
    let before = g.state.clone();
    assert!(
        g.submit_action(P0, activate(gv, ability, mode, bear))
            .is_err(),
        "equip is sorcery-speed only (CR 307.5)"
    );
    assert_eq!(g.state, before, "timing refusal leaves state untouched");
    assert_eq!(attached_to(&g, bear), 0);
}

#[test]
fn protection_is_read_live_and_two_equipment_coexist_on_one_creature() {
    // Boots' hexproof is opponents-only; Greaves then adds shroud for every
    // player. No supported instant attaches Equipment, so a creature cannot gain
    // shroud while a spell is already on the stack. What this test does
    // establish is the direction that matters for targeting: protection is
    // evaluated from the live attachment each time a target is offered or
    // validated, and it never hides the controller as a player target.
    let mut g = game(425004);
    let gv = hand_card(&g, P0, greaves());
    let bt = hand_card(&g, P0, boots());
    cast(&mut g, gv);
    cast(&mut g, bt);
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    equip(&mut g, bt, bear);
    assert_eq!(attached_to(&g, bear), 1, "Boots attached");

    let own = put_in_hand(&mut g, P0, CardId::GiantGrowth);
    assert!(
        g.validate(
            P0,
            &Action::CastSpell {
                card: own,
                targets: vec![Target::Permanent(bear)]
            }
        )
        .is_ok(),
        "hexproof does not stop the controller's own spell"
    );
    let enemy = put_in_hand(&mut g, P1, CardId::Shock);
    g.state.players[P1 as usize].mana = ManaPool {
        red: 4,
        ..ManaPool::default()
    };
    let before = g.state.clone();
    assert!(
        g.submit_action(
            P1,
            Action::CastSpell {
                card: enemy,
                targets: vec![Target::Permanent(bear)]
            }
        )
        .is_err(),
        "hexproof refuses the opponent's spell"
    );
    assert_eq!(g.state, before, "refused cast leaves the state untouched");

    // Greaves adds shroud on top: now the controller's own spell is refused too,
    // while both attachments stay (CR 301.5 lets one creature carry any number).
    equip(&mut g, gv, bear);
    assert_eq!(attached_to(&g, bear), 2, "two Equipment on one creature");
    let before = g.state.clone();
    assert!(
        g.submit_action(
            P0,
            Action::CastSpell {
                card: own,
                targets: vec![Target::Permanent(bear)]
            }
        )
        .is_err(),
        "shroud refuses the controller's spell as well"
    );
    assert_eq!(g.state, before);
    assert!(
        !g.legal_targets(P0, TargetSpec::AnyTarget, ObjectId(u32::MAX))
            .contains(&Target::Permanent(bear))
            && !g
                .legal_targets(P1, TargetSpec::AnyTarget, ObjectId(u32::MAX))
                .contains(&Target::Permanent(bear)),
        "no seat is offered the shrouded creature as a target"
    );
    // Player targets are untouched by a creature keyword.
    assert!(
        g.legal_targets(P0, TargetSpec::AnyTarget, ObjectId(u32::MAX))
            .contains(&Target::Player(P1)),
        "a shrouded creature does not hide its controller"
    );
}

/// CR 702.6b: equip is a *targeted* activated ability, so shroud and hexproof
/// apply to the equip target exactly as they apply to a spell's target — the
/// grant is read from the live attachment at offer time and again at submit.
///
/// This is the direction the test above could not reach: it only proved that an
/// already-attached Equipment keeps its place when a second attachment arrives.
/// Here the creature is protected *first*, so the equip itself must refuse.
#[test]
fn equip_cannot_target_a_protected_creature_but_keeps_existing_attachments() {
    let mut g = game(425005);
    let gv = hand_card(&g, P0, greaves());
    let bt = hand_card(&g, P0, boots());
    cast(&mut g, gv);
    cast(&mut g, bt);
    let bears: Vec<ObjectId> = g
        .state
        .objects
        .values()
        .filter(|o| o.zone == Zone::Battlefield && o.card == CardId::GrizzlyBears)
        .map(|o| o.id)
        .collect();
    assert_eq!(bears.len(), 2, "two creatures to equip");
    let (bear1, bear2) = (bears[0], bears[1]);

    // Greaves onto bear1 (Equip {0}): bear1 now has shroud for every player.
    equip(&mut g, gv, bear1);
    assert_eq!(attached_to(&g, bear1), 1);
    assert!(
        g.state.effects.iter().any(|e| matches!(
            &e.kind,
            ContinuousKind::Equipped { shroud: true, .. }
        ) && e.target == bear1),
        "Greaves grants shroud to its equipped creature"
    );

    // The controller's own equip is a targeted ability, so it is refused too.
    let boots_equip = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action, Action::ActivateAbility { source, .. } if *source == bt))
        .expect("Boots equip offered")
        .action;
    let Action::ActivateAbility { ability, mode, .. } = boots_equip else {
        unreachable!("equip is an activated ability")
    };
    assert!(
        !g.legal_actions().into_iter().any(|a| matches!(
            &a.action,
            Action::ActivateAbility { source, targets, .. }
                if *source == bt && targets == &vec![Target::Permanent(bear1)]
        )),
        "a shrouded creature is not offered as an equip target, even to its controller"
    );
    let before = g.state.clone();
    assert!(
        g.submit_action(P0, activate(bt, ability, mode, bear1))
            .is_err(),
        "shroud refuses the controller's own equip (CR 702.6b)"
    );
    assert_eq!(g.state, before, "shroud refusal leaves the state untouched");
    assert_eq!(attached_to(&g, bear1), 1, "bear1 keeps only Greaves");
    assert_eq!(attached_to(&g, bear2), 0, "nothing moved to bear2");

    // Hexproof is opponents-only, so the controller's equip is legal on bear2.
    equip(&mut g, bt, bear2);
    assert_eq!(attached_to(&g, bear2), 1);

    // bear2 then takes a second Equipment *while* it has hexproof, and that
    // attachment predates the shroud Greaves is about to add: a keyword gained
    // after attachment never retroactively detaches anything.
    equip(&mut g, gv, bear2);
    assert_eq!(
        attached_to(&g, bear2),
        2,
        "both Equipment stay after shroud"
    );
    assert!(
        g.state.effects.iter().any(|e| matches!(
            &e.kind,
            ContinuousKind::Equipped { shroud: true, .. }
        ) && e.target == bear2),
        "bear2 now has shroud from Greaves"
    );

    // Moving Greaves away leaves only Boots: no stale attachment survives.
    equip(&mut g, gv, bear1);
    assert_eq!(attached_to(&g, bear2), 1, "only Boots remains on bear2");
    assert_eq!(attached_to(&g, bear1), 1, "Greaves moved back to bear1");
    assert_eq!(
        g.state
            .effects
            .iter()
            .filter(|e| matches!(e.kind, ContinuousKind::Equipped { .. }))
            .count(),
        2,
        "exactly one attachment per Equipment"
    );
}
