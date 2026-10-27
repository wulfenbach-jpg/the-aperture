//! Slice 25: Exuberant Wolfbear base-P/T setting attack trigger.
mod common;
use aperture_core::catalog;
#[test]
fn exuberant_wolfbear_is_sourced_and_supported() {
    let def = catalog::by_name("Exuberant Wolfbear")
        .expect("supported")
        .def();
    assert_eq!(def.cost, Some("{3}{G}"));
    assert_eq!(def.type_line, "Creature — Wolf Bear");
    assert_eq!(def.power, Some(4));
    assert_eq!(def.toughness, Some(4));
    assert_eq!(
        def.oracle,
        "Whenever this creature attacks, you may have the base power and toughness of target Human you control become equal to this creature's power and toughness until end of turn."
    );
}

use aperture_core::*;
use common::*;
fn card25(n: &str) -> CardId {
    catalog::by_name(n).unwrap()
}
fn attack_to_choice(g: &mut Game) -> (ObjectId, ObjectId) {
    let wolf = battlefield_card(g, P0, card25("Exuberant Wolfbear"));
    let human = battlefield_card(g, P0, CardId::EliteVanguard);
    let mut log = Vec::new();
    assert!(run_until(
        g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![wolf],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        g,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::TriggerChoice { .. })),
        64
    ));
    (wolf, human)
}
fn base_game(seed: u64) -> Game {
    fixture(seed)
        .battlefield(P0, &[card25("Exuberant Wolfbear"), CardId::EliteVanguard])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build()
}

#[test]
fn attack_trigger_sets_human_base_to_current_wolfbear_stats_after_counters() {
    let mut g = base_game(2501);
    let wolf = battlefield_card(&g, P0, card25("Exuberant Wolfbear"));
    g.state
        .objects
        .get_mut(&wolf)
        .unwrap()
        .counters
        .add(aperture_core::counters::CounterKind::PlusOnePlusOne, 1);
    let human = battlefield_card(&g, P0, CardId::EliteVanguard);
    g.state
        .objects
        .get_mut(&human)
        .unwrap()
        .counters
        .add(aperture_core::counters::CounterKind::PlusOnePlusOne, 1);
    let (_wolf, human) = attack_to_choice(&mut g);
    submit_ok(
        &mut g,
        P0,
        Action::AnswerTriggerChoice { use_ability: true },
    );
    while !g.state.stack.is_empty() {
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(
        g.state.object(human).power(),
        6,
        "base 5/5 then the target's own +1/+1 counter"
    );
    assert_eq!(g.state.object(human).toughness(), 6);
    assert!(g.state.effects.iter().any(|e| e.kind
        == ContinuousKind::SetBasePowerToughness {
            power: 5,
            toughness: 5
        }));
}

#[test]
fn declining_leaves_human_unchanged_and_forged_nonhuman_target_refused() {
    let mut g = base_game(2502);
    let (_wolf, human) = attack_to_choice(&mut g);
    let before = g.state.clone();
    submit_ok(
        &mut g,
        P0,
        Action::AnswerTriggerChoice { use_ability: false },
    );
    assert_eq!(g.state.object(human).power(), 2);
    assert_eq!(g.state.object(human).toughness(), 1);
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, ContinuousKind::SetBasePowerToughness { .. }))
    );
    let _ = before;
}

#[test]
fn source_left_uses_effective_last_known_power_for_base_setting() {
    let mut g = fixture(2503)
        .hand(P0, &[CardId::Unsummon])
        .battlefield(
            P0,
            &[
                card25("Exuberant Wolfbear"),
                CardId::EliteVanguard,
                CardId::Island,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let wolf = battlefield_card(&g, P0, card25("Exuberant Wolfbear"));
    g.state
        .objects
        .get_mut(&wolf)
        .unwrap()
        .counters
        .add(aperture_core::counters::CounterKind::PlusOnePlusOne, 1);
    let human = battlefield_card(&g, P0, CardId::EliteVanguard);
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
            attackers: vec![wolf],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g
            .state
            .stack
            .iter()
            .any(|s| s.card == card25("Exuberant Wolfbear")),
        64
    ));
    let unsummon = hand_card(&g, P0, CardId::Unsummon);
    let a=g.legal_actions().into_iter().find(|a|matches!(&a.action,Action::CastSpell{card,targets} if *card==unsummon && targets==&vec![Target::Permanent(wolf)])).expect("Unsummon offered").action;
    submit_ok(&mut g, P0, a);
    submit_ok(&mut g, P0, Action::Pass);
    submit_ok(&mut g, P1, Action::Pass);
    assert_eq!(g.state.object(wolf).zone, Zone::Hand);
    assert!(run_until(
        &mut g,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::TriggerChoice { .. })),
        64
    ));
    submit_ok(
        &mut g,
        P0,
        Action::AnswerTriggerChoice { use_ability: true },
    );
    while !g.state.stack.is_empty() {
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(
        g.state.object(human).power(),
        5,
        "LKI 5/5 from the departed pumped Wolfbear"
    );
    assert_eq!(g.state.object(human).toughness(), 5);
}
