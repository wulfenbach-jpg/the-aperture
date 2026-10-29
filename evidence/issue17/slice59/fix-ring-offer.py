from pathlib import Path

p = Path('crates/core/src/engine.rs')
s = p.read_text(encoding='utf-8')
old = """            Pending::EffectDiscard { player } => vec![LegalAction {
                action: Action::Discard {
                    cards: self.sample_discard(*player, 1),
                },
                label: "Discard a card (The Ring)".to_string(),
                rules: vec!["701.54a"],
            }],"""
new = """            Pending::EffectDiscard { player } => {
                // Reuse the canonical hand-choice offer (#57): one named offer per
                // card in hand, so the Ring discard cannot present one hard-coded
                // sample while `check_selection` accepts any legal card.
                self.hand_choice_offers(*player, 1, HandChoice::Discard)
            }"""
assert s.count(old) == 1, s.count(old)
p.write_text(s.replace(old, new), encoding='utf-8', newline='\n')
print('Ring discard offer now uses hand_choice_offers')
