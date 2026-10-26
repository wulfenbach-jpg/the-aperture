import json, pathlib, re, collections

d = pathlib.Path(__file__).resolve().parent
cards = json.loads((d / 'scryfall-missing-union.json').read_text())
root = pathlib.Path(__file__).resolve().parents[3]
m = json.loads((root / 'decks/starter-corpus/manifest.json').read_text())
listof = collections.defaultdict(list)
for s in m['starters']:
    for n in s['missing_cards']:
        listof[n].append(s['id'])

kwline = re.compile(r'^(Flash|Trample|Vigilance|Reach)( \(.*?\))?\.?$', re.I)
trig = re.compile(
    r'^(?:(?:When|Whenever) [^,]{1,120}, (?:you may )?(?:draw \w+ cards?|you gain \d+ life|scry \d+|surveil \d+)\.)$',
    re.I)
act = re.compile(r'^\{T\}: This creature deals \d+ damage to any target\.$', re.I)

rows = []
for name, c in sorted(cards.items()):
    ot = (c['oracle_text'] or '').strip()
    tl = c['type_line'] or ''
    if 'Creature' not in tl or c.get('card_faces') or not ot:
        continue
    lines = [ln.strip() for ln in ot.split('\n') if ln.strip()]
    kws, triggers, acts, bad = [], 0, 0, 0
    for ln in lines:
        if kwline.match(ln):
            kws.append(kwline.match(ln).group(1).capitalize())
        elif trig.match(ln):
            triggers += 1
        elif act.match(ln):
            acts += 1
        else:
            bad += 1
    if bad == 0 and (triggers or acts):
        rows.append((name, c['mana_cost'], tl, c['power'], c['toughness'], kws, triggers, acts, listof[name]))
print('SIMPLE TRIGGER CREATURES', len(rows))
for r in rows:
    print('  ', r)

# also: creature with ONLY flash/trample/vigilance/reach line even if 0 triggers (sanity)
solo = []
for name, c in sorted(cards.items()):
    ot = (c['oracle_text'] or '').strip()
    tl = c['type_line'] or ''
    if 'Creature' in tl and ot and not c.get('card_faces'):
        lines = [ln.strip() for ln in ot.split('\n') if ln.strip()]
        if all(kwline.match(ln) for ln in lines):
            solo.append((name, kws if False else [kwline.match(ln).group(1) for ln in lines], listof[name]))
print('KEYWORD-ONLY', len(solo))
for r in solo:
    print('  ', r)
