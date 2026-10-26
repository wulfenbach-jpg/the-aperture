#[path = "C:/AI/aperture-worktrees/issue-17/crates/core/tests/common/mod.rs"]
mod common;
use aperture_core::*;
use common::*;

fn drained(g: &Game) -> bool {
    g.state.pending.is_none() && g.state.resolving.is_none()
        && g.state.stack.is_empty() && g.state.trigger_queue.is_empty()
}

#[test]
fn all_simultaneous_death_triggers_resolve_exactly_once() {
    for cards in [[CardId::ZulaportCutthroat, CardId::TrainedJackal],
                  [CardId::TrainedJackal, CardId::ZulaportCutthroat]] {
        let mut g = fixture_n(224, 4).battlefield(P0, &cards)
            .main(P0, Step::PrecombatMain).build();
        for o in g.state.objects.values_mut() { o.damage = 5; }
        let owner = g.decision_owner().unwrap();
        submit_ok(&mut g, owner, Action::Pass);
        let mut log = vec![];
        assert!(run_until(&mut g, &mut log, drained, 100));
        let life: Vec<_> = g.state.players.iter().map(|p| p.life).collect();
        println!("fully drained batch {:?}: lives {:?}", cards, life);
        assert_eq!(life, vec![22,18,18,18], "two deaths must produce exactly two drains");
    }
}

#[test]
fn single_watcher_death_fires_only_once() {
    let mut g = fixture_n(226,4).battlefield(P0, &[CardId::ZulaportCutthroat])
        .main(P0,Step::PrecombatMain).build();
    for o in g.state.objects.values_mut() { o.damage=5; }
    submit_ok(&mut g,P0,Action::Pass);
    let mut log=vec![];
    assert!(run_until(&mut g,&mut log,drained,100));
    let life:Vec<_>=g.state.players.iter().map(|p|p.life).collect();
    println!("single watcher death fully drained: {:?}",life);
    assert_eq!(life,vec![21,19,19,19]);
}

#[test]
fn surviving_watcher_control_two_deaths() {
    let mut g = fixture_n(225,4).battlefield(P0, &[CardId::ZulaportCutthroat,
        CardId::TrainedJackal, CardId::GrizzlyBears])
        .main(P0,Step::PrecombatMain).build();
    for o in g.state.objects.values_mut() {
        if o.card != CardId::ZulaportCutthroat { o.damage = 5; }
    }
    submit_ok(&mut g, P0, Action::Pass);
    let mut log=vec![];
    assert!(run_until(&mut g,&mut log,drained,100));
    let life:Vec<_>=g.state.players.iter().map(|p|p.life).collect();
    println!("surviving watcher fully drained: {:?}",life);
    assert_eq!(life,vec![22,18,18,18]);
}
