import re

p = 'src/main.rs'
src = open(p, encoding='utf-8').read()

# 1. package
src = src.replace('"decks/package-slice46.txt"', '"decks/package-slice48.txt"')

# 2. look action: take a creature card from the pending look when one is offered.
look_block = '''        // Owlbear Cub's look: take a creature card from among the examined
        // cards when one is offered, so the "you may put" path is exercised in a
        // real game rather than only in a fixture.
        let look_action = if view.pending_kind.is_some()
            && view.legal.iter().any(|legal| {
                matches!(legal.action, Action::SearchLibraryPick { .. })
            })
        {
            view.legal
                .iter()
                .find(|legal| matches!(legal.action, Action::SearchLibraryPick { .. }))
                .map(|legal| legal.action.clone())
        } else {
            None
        };
'''
anchor = '        let demo_action = attack_action.or(clue_action)'
assert anchor in src
src = src.replace(anchor, look_block + anchor.replace('attack_action.or(clue_action)',
                                                      'attack_action.or(look_action.clone()).or(clue_action)'))

# 3. output paths
src = src.replace('evidence/issue17/slice48', 'evidence/issue17/slice48-cub')
src = src.replace('out.join("demo-save.json")', 'out.join("cub-save.json")')
src = src.replace('out.join("demo-replay.json")', 'out.join("cub-replay.json")')
src = src.replace('out.join("demo-journey.json")', 'out.join("cub-journey.json")')

# 4. replace the Witness-specific assertions with Owlbear Cub ones
old_start = src.index('    let witnesses = casts.get("Thijarian Witness")')
old_end = src.index('    std::fs::write(\n        out.join("cub-replay.json")')
new_asserts = '''    let cubs = casts.get("Owlbear Cub").copied().unwrap_or(0);
    assert!(cubs > 0, "Owlbear Cub must be cast");
    let looks = events
        .iter()
        .filter(|e| {
            matches!(e, Event::StateBasedAction { description }
                if description.contains("looks at the top"))
        })
        .count();
    assert!(
        looks > 0,
        "the printed trigger must fire against a defender with eight or more lands"
    );
    let puts = events
        .iter()
        .filter(|e| {
            matches!(e, Event::StateBasedAction { description }
                if description.contains("onto the battlefield tapped and attacking"))
        })
        .count();
    assert!(
        puts > 0,
        "the put-creature half must be exercised, not only the decline path"
    );
    let bottoms = events
        .iter()
        .filter(|e| {
            matches!(e, Event::StateBasedAction { description }
                if description.contains("rest of the looked-at cards"))
        })
        .count();
    assert!(bottoms > 0, "the rest must go to the bottom");
'''
src = src[:old_start] + new_asserts + src[old_end:]

# 5. report fields
src = src.replace('"solo_attacks_declared": alone_attacks,',
                  '"solo_attacks_declared": alone_attacks,\n        "cub_looks": looks, "cub_puts": puts, "cub_bottoms": bottoms,')
src = src.replace('"clue_tokens_created": clues,', '"witness_style_clues": events.iter().filter(|e| matches!(e, Event::PermanentEntered { card: CardId::ClueToken, .. })).count(),')
src = src.replace('"exile_events": exiles,', '"solo_exile_events": events.iter().filter(|e| matches!(e, Event::StateBasedAction { description } if description.contains("is exiled"))).count(),')
src = src.replace('"driver": "evidence-only masked alone-attacking demonstrator; NOT product heuristic or strength evidence"',
                  '"driver": "evidence-only masked Owlbear Cub demonstrator; NOT product heuristic or strength evidence"')

open(p, 'w', encoding='utf-8').write(src)
print('patched')
for needle in ['package-slice48', 'cub-save.json', 'cub_looks', 'Owlbear Cub must be cast']:
    assert needle in src, needle
print('ok')
