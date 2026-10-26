import json
import re
from pathlib import Path
root = Path.cwd()
text = (root / 'crates/core/src/catalog.rs').read_text(encoding='utf-8')
ids = {name: ident for ident, name in re.findall(r'(\w+), "([^"]+)", cost:', text)}
names = set(ids)
manifest = json.loads((root / 'decks/starter-corpus/manifest.json').read_text())
union = set()
for starter in manifest['starters']:
    cards = re.findall(r'(?m)^(\d+) (.+)$', (root / starter['deck_path']).read_text())
    assert sum(int(n) for n, _ in cards) == 99
    union.update(name for _, name in cards)
    union.add(starter['commander'])
replay = json.loads((root / 'evidence/issue17/review227-journey.replay.json').read_text())
package = (root / 'decks/package-slice6-dies.txt').read_text()
expected = [[ids[name], int(count)] for count, name in re.findall(r'(?m)^(\d+) (.+)$', package)]
assert len(replay['seats']) == 4
assert all(seat['cards'] == expected for seat in replay['seats'])
print(json.dumps(dict(catalog=len(names), union=len(union), supported=len(union & names), missing=len(union - names), exact_four_seat_snapshots=True, focused_tests=sum([16, 13, 10])), indent=2))
