from pathlib import Path
p=Path('crates/core/tests/issue17_slice46.rs')
s=p.read_text(encoding='utf-8')
s+='''
/// A creature put onto the battlefield attacking counts in LIVE combat even
/// though it never attacked. Cub is declared alone but no longer IS attacking
/// alone once its put joins it. After Cub dies, the put creature IS alone.
#[test]
fn cub_put_attacker_counts_without_a_declaration() {
    let mut g = fixture(5001)
        .battlefield(P0, &[witness(), CardId::OwlbearCub])
        .battlefield(P1, &[CardId::Forest; 8])
        .battlefield(P1, &[CardId::Mountain; 4])
        .hand(P1, &[CardId::LightningStrike; 2])
        .library(P0, &[
            CardId::GrizzlyBears, CardId::Forest, CardId::Forest, CardId::Forest,
            CardId::Forest, CardId::Forest, CardId::Forest, CardId::Forest,
        ])
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let cub = battlefield_card(&g, P0, CardId::OwlbearCub);
    let put = g.state.player(P0).library[0];
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    submit_ok(&mut g, P0, Action::DeclareAttackers {
        attackers: vec![cub], defender: Some(P1),
    });
    for _ in 0..16 {
        if g.state.pending.is_some() { break; }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(matches!(g.state.pending, Some(Pending::SearchLibrary { .. })));
    let pick = g.legal_actions().into_iter().find(|a| {
        matches!(a.action, Action::SearchLibraryPick { card } if card == put)
    }).expect("the real Cub put is offered").action;
    submit_ok(&mut g, P0, pick);
    submit_ok(&mut g, P0, Action::SearchLibraryDone);
    assert_eq!(g.state.turn.declared_attackers, vec![cub]);
    assert!(g.state.turn.attackers.contains(&put));
    assert_eq!(g.state.turn.attackers.len(), 2);
    assert!(g.state.object(put).tapped);
    assert_eq!(g.decision_owner(), Some(P0));
    submit_ok(&mut g, P0, Action::Pass);
    cast_at_creature(&mut g, P1, CardId::LightningStrike, cub);
    assert_eq!(g.state.object(cub).zone, Zone::Graveyard);
    assert!(clue_tokens(&g, P0).is_empty(), "declared alone is NOT attacking alone");
    assert_eq!(g.state.turn.attackers, vec![put]);
    submit_ok(&mut g, P0, Action::Pass);
    cast_at_creature(&mut g, P1, CardId::LightningStrike, put);
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(put).zone, Zone::Exile);
    assert_eq!(clue_tokens(&g, P0).len(), 1, "undeclared put attacker was attacking alone");
    assert!(g.state.stack.is_empty());
    assert!(g.state.resolving.is_none());
    assert!(g.state.trigger_queue.is_empty());
    assert!(g.state.pending.is_none());
}

/// Legacy saves lack declared_* fields. The serialized public live combat
/// membership is sufficient for Witness; no declaration record is consulted.
#[test]
fn absent_legacy_declaration_does_not_suppress_a_live_alone_death() {
    let mut g = fixture(5002)
        .battlefield(P0, &[witness(), CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::OrdinaryBear])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let blocker = battlefield_card(&g, P1, CardId::OrdinaryBear);
    attack_and_block(&mut g, &[bear], vec![BlockDecl { blocker, attacker: bear }]);
    // Model the serde(default) absent-field shape, then actually round-trip it.
    let mut value = serde_json::to_value(&g.state).unwrap();
    value["turn"].as_object_mut().unwrap().remove("declared_attackers");
    value["turn"].as_object_mut().unwrap().remove("declared_blockers");
    g.state = serde_json::from_value(value).unwrap();
    assert!(g.state.turn.declared_attackers.is_empty());
    assert_eq!(g.state.turn.attackers, vec![bear]);
    g.state.validate().unwrap();
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(bear).zone, Zone::Exile);
    assert_eq!(clue_tokens(&g, P0).len(), 1);
}
'''
p.write_text(s, encoding='utf-8')
