import json, pathlib, re, collections

d = pathlib.Path(__file__).resolve().parent
cards = json.loads((d / 'scryfall-missing-union.json').read_text())
root = pathlib.Path(__file__).resolve().parents[3]
m = json.loads((root / 'decks/starter-corpus/manifest.json').read_text())

VANILLA = []
LANDS = []
OTHER = []
for name, c in sorted(cards.items()):
    tl = c['type_line'] or ''
    ot = (c['oracle_text'] or '').strip()
    kw = c['keywords'] or []
    faces = c.get('card_faces')
    if 'Land' in tl:
        LANDS.append((name, tl, ot, c['mana_cost']))
        continue
    if ('Creature' in tl and not ot and not kw and not faces
            and c['power'] and c['toughness']
            and re.fullmatch(r'-?\d+', str(c['power'])) and re.fullmatch(r'-?\d+', str(c['toughness']))):
        VANILLA.append((name, c['mana_cost'], tl, c['power'], c['toughness'], c['set'], c['collector_number']))
        continue
    OTHER.append((name, tl, ot, kw))

print('VANILLA', len(VANILLA))
for v in VANILLA:
    print('  ', v)
print('LANDS', len(LANDS))
for v in LANDS:
    print('  ', v)
print('OTHER', len(OTHER))

# per-list impact if all vanilla implemented
by_list = {}
for s in m['starters']:
    van = sorted(set(s['missing_cards']) & {v[0] for v in VANILLA})
    by_list[s['id']] = van
out = d / 'classify.json'
out.write_text(json.dumps({'vanilla': VANILLA, 'lands': LANDS,
                           'vanilla_per_list': by_list,
                           'other_count': len(OTHER)}, indent=1))
for k, v in by_list.items():
    print(k, len(v), v)
