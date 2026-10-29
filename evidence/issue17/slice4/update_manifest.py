"""Recompute `missing_cards` from the catalog for the eight desired lists and
report the slice-4 corpus movement. The desired 100-card list files are never
modified: only the manifest's derived missing-name lists change.
"""
import json, pathlib, re, collections

root = pathlib.Path(__file__).resolve().parents[3]
manifest_path = root / 'decks/starter-corpus/manifest.json'
manifest = json.loads(manifest_path.read_text())
txt = (root / 'crates/core/src/catalog.rs').read_text()
catalog = set(re.findall(r'(\w+), "([^"]+)", cost:', txt))
names = {n for _, n in catalog}

changed = {}
union, missing = set(), set()
for starter in manifest['starters']:
    deck = root / starter['deck_path']
    cards = collections.Counter()
    for line in deck.read_text().splitlines():
        line = line.strip()
        if re.match(r'^\d+ ', line):
            count, name = line.split(maxsplit=1)
            cards[name] += int(count)
    assert sum(cards.values()) == 99, (starter['id'], sum(cards.values()))
    listed = set(cards) | {starter['commander']}
    before = len(starter['missing_cards'])
    starter['missing_cards'] = sorted(listed - names)
    after = len(starter['missing_cards'])
    if before != after:
        changed[starter['id']] = (before, after)
    union |= listed
    missing |= set(starter['missing_cards'])

manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
result = {
    'catalog_cards': len(catalog),
    'catalog_unique_names': len(names),
    'exact_union_names': len(union),
    'supported_union_names': len(union & names),
    'missing_union_names': len(union - names),
    'manifest_missing_names': len(missing),
    'manifest_matches_catalog': missing == (union - names),
    'changed_lists': changed,
    'implemented_lists': sum(1 for s in manifest['starters'] if s['status'] == 'implemented'),
    'certified_lists': sum(1 for s in manifest['starters'] if s['status'] == 'certified'),
    'newly_supported': sorted((union & names) & {
        'Ordinary Bear', 'Ashcoat Bear', 'Owlbear', 'Soul Warden'}),
}
out = pathlib.Path(__file__).resolve().parent / 'slice4-corpus-state.json'
out.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
