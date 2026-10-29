import json, pathlib, re, collections

d = pathlib.Path(__file__).resolve().parent
cards = json.loads((d / 'scryfall-missing-union.json').read_text())
root = pathlib.Path(__file__).resolve().parents[3]
m = json.loads((root / 'decks/starter-corpus/manifest.json').read_text())
listof = collections.defaultdict(list)
for s in m['starters']:
    for n in s['missing_cards']:
        listof[n].append(s['id'])

# Cards implementable with only flash/trample/vigilance/reach + supported effects
kwline = re.compile(r'^(Flash|Trample|Vigilance|Reach)( \(.*?\))?\.?$', re.I)
etb_draw = re.compile(r'^((?:Keen Senses|Bear Witness) — )?When [^,]{1,80} enters(?: the battlefield)?, draw a card\.$', re.I)
cands = []
for name, c in sorted(cards.items()):
    ot = (c['oracle_text'] or '').strip()
    tl = c['type_line'] or ''
    if 'Creature' not in tl or c.get('card_faces') or not ot:
        continue
    lines = [ln.strip() for ln in ot.split('\n') if ln.strip()]
    kws = []
    rest = []
    for ln in lines:
        mm = kwline.match(ln)
        if mm:
            kws.append(mm.group(1).capitalize())
        elif etb_draw.match(ln):
            rest.append('ETB-DRAW')
        else:
            rest.append(ln)
    if rest and all(r == 'ETB-DRAW' for r in rest) and kws:
        cands.append((name, c['mana_cost'], tl, c['power'], c['toughness'], kws, rest, listof[name]))
    elif not rest and kws:
        cands.append((name, c['mana_cost'], tl, c['power'], c['toughness'], kws, rest, listof[name]))

print('CANDIDATES', len(cands))
for x in cands:
    print('  ', x)
