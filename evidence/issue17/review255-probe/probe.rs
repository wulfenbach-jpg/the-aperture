#[path = "../../../crates/core/tests/common/mod.rs"]
mod common;
use aperture_core::*;
use common::*;
fn drain(g: &mut Game, events: &mut Vec<Event>) {
    for _ in 0..100 {
        if g.state.stack.is_empty() && g.state.trigger_queue.is_empty() && g.state.resolving.is_none() && g.state.pending.is_none() { return; }
        let p = g.decision_owner().expect("decision owner");
        events.extend(submit_ok(g, p, Action::Pass));
    }
    panic!("did not fully drain");
}
#[test]
fn removed_little_bear_target_fizzles_without_touching_graveyard() {
    let mut g = fixture_n(255, 2).main(P0, Step::PrecombatMain)
        .mana(P0, ManaPool {green: 1, colorless: 2, ..Default::default()})
        .mana(P1, ManaPool {red: 1, ..Default::default()})
        .hand(P0, &[CardId::LittleBear]).hand(P1, &[CardId::Shock])
        .battlefield(P0, &[CardId::GrizzlyBears]).build();
    let target = battlefield_card(&g,P0,CardId::GrizzlyBears);
    object_mut(&mut g.state,target).tapped=true;
    let little = hand_card(&g,P0,CardId::LittleBear);
    let mut log = submit_ok(&mut g,P0,Action::CastSpell{card:little,targets:vec![]});
    for _ in 0..12 {
        if g.state.stack.last().is_some_and(|x| matches!(x.entry, StackEntry::Triggered{..})) {break;}
        let p=g.decision_owner().unwrap(); log.extend(submit_ok(&mut g,p,Action::Pass));
    }
    assert!(g.state.stack.last().is_some_and(|x| matches!(x.entry, StackEntry::Triggered{..})));
    if g.decision_owner()==Some(P0) {log.extend(submit_ok(&mut g,P0,Action::Pass));}
    assert_eq!(g.decision_owner(),Some(P1));
    let shock=hand_card(&g,P1,CardId::Shock);
    log.extend(submit_ok(&mut g,P1,Action::CastSpell{card:shock,targets:vec![Target::Permanent(target)]}));
    drain(&mut g,&mut log);
    assert_eq!(g.state.object(target).zone,Zone::Graveyard);
    assert_eq!(g.state.object(target).counters.get(aperture_core::counters::CounterKind::PlusOnePlusOne),0);
    assert!(log.iter().any(|x| matches!(x,Event::AbilityFizzled{card:CardId::LittleBear,..})));
}
#[test]
fn fade_makes_one_bear_not_one_per_qualifying_permanent() {
    let mut g=fixture_n(256,4).main(P0,Step::PrecombatMain)
        .mana(P0,ManaPool{green:2,colorless:2,..Default::default()})
        .hand(P0,&[CardId::FadeFromHistory])
        .battlefield(P1,&[CardId::SolRing,CardId::Bearscape,CardId::MindStone]).build();
    let spell=hand_card(&g,P0,CardId::FadeFromHistory);
    let mut log=submit_ok(&mut g,P0,Action::CastSpell{card:spell,targets:vec![]});
    drain(&mut g,&mut log);
    assert_eq!(g.state.battlefield(P1).iter().filter(|o| o.card==CardId::BearToken).count(),1);
    assert_eq!(log.iter().filter(|e| matches!(e,Event::Destroyed{card:CardId::SolRing | CardId::Bearscape | CardId::MindStone,..})).count(),3);
    assert_eq!(g.state.battlefield(P0).len(),0);
}
