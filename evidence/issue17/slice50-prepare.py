from pathlib import Path
root=Path(__file__).resolve().parents[2]
p=root/'crates/core/tests/issue17_slice46.rs'
s=p.read_text(encoding='utf-8')
a=s.index('/// R325-1 discriminator:')
b=s.index('    let mut g = fixture(4706)', a)
s=s[:a]+'''/// CR 506.5 live-state discriminator: after the first of two attackers dies,
/// the second IS attacking alone even though it did not ATTACK alone.
/// Witness says "was attacking", so its last-known pre-departure state qualifies.
#[test]
fn attacker_killed_after_its_companion_left_combat_is_attacking_alone() {
'''+s[b:]
s=s.replace('''    // The discriminating death: the survivor is alone in the live list, but was
    // declared beside the first attacker, so the printed condition is false.''','''    // The survivor is now attacking alone: the second death qualifies even
    // though both creatures were originally declared together.''')
a=s.index('    assert_eq!(\n        g.state.object(second).zone,',s.index('fn attacker_killed_after_'))
b=s.index('/// The same declaration reading on the blocking side:',a)
s=s[:a]+'''    // The first spell did not trigger; the second death must trigger once.
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(first).zone, Zone::Graveyard);
    assert_eq!(g.state.object(second).zone, Zone::Exile);
    assert!(g.state.turn.attackers.is_empty());
    assert_eq!(exiled_cards(&g, P0), vec![CardId::GrizzlyBears]);
    assert_eq!(clue_tokens(&g, P0).len(), 1);
    assert!(g.state.stack.is_empty());
    assert!(g.state.trigger_queue.is_empty());
    assert!(g.state.resolving.is_none());
    assert!(g.state.pending.is_none());
}

'''+s[b:]
a=s.index('/// The same declaration reading on the blocking side:')
b=s.index('    let mut g = fixture(4707)',a)
s=s[:a]+'''/// On the blocking side, after the first blocker dies to a spell the survivor
/// IS blocking alone when combat damage kills it. Only the second death qualifies.
#[test]
fn blocker_killed_after_its_companion_left_combat_is_blocking_alone() {
'''+s[b:]
a=s.index('    assert!(\n        !exiled_cards(&g, P1)', s.index('fn blocker_killed_after_'))
b=s.index('/// A creature that dies outside combat',a)
s=s[:a]+'''    assert_eq!(g.state.object(first).zone, Zone::Graveyard);
    assert_eq!(g.state.object(second).zone, Zone::Exile);
    assert_eq!(exiled_cards(&g, P1), vec![CardId::GrizzlyBears]);
    assert_eq!(clue_tokens(&g, P0).len(), 1);
    assert!(g.state.stack.is_empty());
    assert!(g.state.trigger_queue.is_empty());
    assert!(g.state.resolving.is_none());
    assert!(g.state.pending.is_none());
}

'''+s[b:]
p.write_text(s,encoding='utf-8')
print(p)
