import collections
import hashlib
import json
import pathlib
import re
import subprocess

root = pathlib.Path(__file__).resolve().parents[2]
catalog = (root / 'crates/core/src/catalog.rs').read_text(encoding='utf-8')
name_to_id = {name: cid for cid, name in re.findall(r'card!\(\s*(\w+),\s*"([^"]+)"', catalog)}
manifest = json.loads((root / 'decks/starter-corpus/manifest.json').read_text())

def entries(path):
    result = collections.Counter()
    for line in (root / path).read_text(encoding='utf-8').splitlines():
        line = line.split('//', 1)[0].split('#', 1)[0].strip()
        if line:
            count, name = line.split(maxsplit=1)
            result[name] += int(count)
    return result

union, missing_union = set(), set()
starters = []
for starter in manifest['starters']:
    deck = entries(starter['deck_path'])
    names = set(deck) | {starter['commander']}
    missing = names - name_to_id.keys()
    assert sorted(missing) == sorted(starter['missing_cards']), starter['id']
    assert sum(deck.values()) + 1 == 100, starter['id']
    original = subprocess.check_output(['git', 'show', '86db7d3:' + starter['deck_path']], cwd=root).decode('utf-8').replace('\r\n', '\n')
    current = (root / starter['deck_path']).read_text(encoding='utf-8').replace('\r\n', '\n')
    assert original == current, starter['deck_path']
    union |= names
    missing_union |= missing
    starters.append({'id': starter['id'], 'status': starter['status'], 'missing_names': len(missing), 'exact_desired_content_preserved': True})

journeys = []
for fixture, replay_path in [
    ('decks/package-krenko-tokens.txt', 'evidence/issue17/slice2-repairs-krenko-game.replay.json'),
    ('decks/package-talrand-drakes.txt', 'evidence/issue17/slice2-talrand-game.replay.json'),
]:
    desired = entries(fixture)
    expected = collections.Counter({name_to_id[name]: count for name, count in desired.items()})
    replay = json.loads((root / replay_path).read_text())
    assert len(replay['seats']) == 4
    for seat in replay['seats']:
        assert collections.Counter(dict(seat['cards'])) == expected, fixture
    assert not replay['truncated'] and 'Win' in replay['outcome'], fixture
    journeys.append({'fixture': fixture, 'library_size': sum(desired.values()), 'exact_seats_verified': 4, 'terminal': True, 'actions': len(replay['steps']), 'digest': replay['final_digest'], 'note': '40-card mechanic package; NOT the desired 100-card Commander starter'})

result = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(), 'catalog_cards': len(name_to_id), 'desired_lists': len(starters), 'implemented_lists': sum(s['status'] == 'implemented' for s in starters), 'certified_lists': sum(s['status'] == 'certified' for s in starters), 'exact_union_names': len(union), 'missing_union_names': len(missing_union), 'starters': starters, 'journeys': journeys, 'full_issue17_accepted': False}
output = root / 'evidence/issue17/slice2-native-verification.json'
output.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result, indent=2))
