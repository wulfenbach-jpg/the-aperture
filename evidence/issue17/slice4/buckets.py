import json, pathlib, re, collections

d = pathlib.Path(__file__).resolve().parent
cards = json.loads((d / 'scryfall-missing-union.json').read_text())
root = pathlib.Path(__file__).resolve().parents[3]
m = json.loads((root / 'decks/starter-corpus/manifest.json').read_text())
listof = {}
for s in m['starters']:
    for n in s['missing_cards']:
        listof.setdefault(n, []).append(s['id'])

kwcount = collections.Counter()
manadork = []
etb_draw = []
simple_gain = []
fight = []
counter_plus = []
for name, c in cards.items():
    ot = (c['oracle_text'] or '').strip()
    for k in (c['keywords'] or []):
        kwcount[k] += 1
    tl = c['type_line'] or ''
    if 'Creature' in tl and re.fullmatch(r'\{T\}: Add \{[WUBRG]\}\.', ot):
        manadork.append((name, c['mana_cost'], ot, tl))
    if 'Creature' in tl and re.fullmatch(r'When [^.]* enters[^,]*, draw a card\.', ot):
        etb_draw.append((name, c['mana_cost'], ot))
    if 'Creature' in tl and re.fullmatch(r'When [^.]* enters[^,]*, you gain \d+ life\.', ot):
        simple_gain.append((name, c['mana_cost'], ot))

print('KEYWORD FREQ', kwcount.most_common())
print('\nMANA DORKS', len(manadork))
for x in manadork:
    print('  ', x, listof.get(x[0]))
print('\nETB DRAW', len(etb_draw))
for x in etb_draw:
    print('  ', x, listof.get(x[0]))
print('\nETB GAIN', len(simple_gain))
for x in simple_gain:
    print('  ', x, listof.get(x[0]))

# template frequency of first sentence
tpl = collections.Counter()
for name, c in cards.items():
    ot = (c['oracle_text'] or '').strip()
    if not ot:
        continue
    s = re.sub(r'\d+', 'N', ot)
    s = re.sub(r'\b[A-Z][a-zA-Z\'-]+(?:,? [A-Z][a-zA-Z\'-]+)*\b', 'X', s)
    tpl[s[:120]] += 1
print('\nTOP TEMPLATES')
for t, n in tpl.most_common(30):
    print(n, '|', t)
