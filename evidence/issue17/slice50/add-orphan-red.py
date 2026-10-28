from pathlib import Path
p=Path('crates/core/tests/issue17_slice46.rs')
s=p.read_text(encoding='utf-8')
s+='''
/// CR 509.1g: a blocker remains blocking after its attacker leaves combat.
/// Losing the pair is NOT removal of the blocking creature from combat.
#[test]
fn blocker_still_blocks_alone_after_its_attacker_is_bounced() {
    let mut g = fixture(5003)
        .hand(P0, &[CardId::Unsummon, CardId::LightningStrike])
        .battlefield(P0, &[
            witness(), CardId::OrdinaryBear, CardId::Island,
            CardId::Mountain, CardId::Mountain,
        ])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attacker = battlefield_card(&g, P0, CardId::OrdinaryBear);
    let blocker = battlefield_card(&g, P1, CardId::GrizzlyBears);
    attack_and_block(&mut g, &[attacker], vec![BlockDecl { blocker, attacker }]);
    cast_at_creature(&mut g, P0, CardId::Unsummon, attacker);
    assert_eq!(g.state.object(attacker).zone, Zone::Hand);
    assert!(g.state.turn.attackers.is_empty());
    assert_eq!(g.state.object(blocker).zone, Zone::Battlefield);
    g.state.validate().unwrap();
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blocker);
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(blocker).zone, Zone::Exile);
    assert_eq!(clue_tokens(&g, P0).len(), 1, "the blocker never left combat before dying");
}
'''
p.write_text(s, encoding='utf-8')
