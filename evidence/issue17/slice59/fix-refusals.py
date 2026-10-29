from pathlib import Path

edits = [
    ('crates/core/src/card.rs',
     """/// Explicit command-zone practice allow-list. This is deliberately narrower
/// than CR 903.3: no general legendary/Vehicle/Spacecraft support exists in
/// this slice. Every entry is a genuine printed vanilla legendary creature; new
/// commanders are added here and to the catalog, never inferred from a type
/// line. Commander-only cards are refused in ordinary starting libraries
/// because the legend rule is not modelled.""",
     """/// Explicit command-zone practice allow-list. This is deliberately narrower
/// than CR 903.3: every entry is a genuine printed legendary creature added here
/// and to the catalog, never inferred from a type line. Commander-only cards are
/// refused in ordinary starting libraries because the named command-zone profile
/// owns their setup; the CR 704.5j legend rule itself is implemented separately."""),
    ('crates/core/src/deck.rs',
     '"line {}: \'{card_name}\' is commander-only in the command-zone practice profile; it cannot be in an ordinary starting library because the legend rule is not modeled"',
     '"line {}: \'{card_name}\' is commander-only in the command-zone practice profile; it cannot be in an ordinary starting library (the named command-zone profile owns its setup)"'),
    ('crates/import/src/lib.rs',
     '"\'{}\' is commander-only in the command-zone practice profile; it cannot be certified as an ordinary practice library card (the legend rule is not modeled)"',
     '"\'{}\' is commander-only in the command-zone practice profile; it cannot be certified as an ordinary practice library card (the named command-zone profile owns its setup)"'),
]
for path, old, new in edits:
    p = Path(path)
    s = p.read_text(encoding='utf-8')
    assert s.count(old) == 1, (path, s.count(old))
    p.write_text(s.replace(old, new), encoding='utf-8', newline='\n')
print('refusal rationales corrected')
