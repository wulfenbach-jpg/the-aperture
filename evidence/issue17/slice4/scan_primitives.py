import json, pathlib, re, collections

d = pathlib.Path(__file__).resolve().parent
cards = json.loads((d / 'scryfall-missing-union.json').read_text())
root = pathlib.Path(__file__).resolve().parents[3]
m = json.loads((root / 'decks/starter-corpus/manifest.json').read_text())
listof = collections.defaultdict(list)
for s in m['starters']:
    for n in s['missing_cards']:
        listof[n].append(s['id'])

KW = {'Flying', 'Trample', 'Vigilance', 'Haste', 'Reach', 'First strike', 'Double strike',
      'Lifelink', 'Deathtouch', 'Menace', 'Indestructible', 'Flash', 'Hexproof', 'Defender',
      'Ward', 'Prowess', 'Shadow', 'Devoid', 'Echo', 'Flashback', 'Storm', 'Splice'}

kw_only = []
for name, c in sorted(cards.items()):
    ot = (c['oracle_text'] or '').strip()
    tl = c['type_line'] or ''
    if 'Creature' not in tl or not ot:
        continue
    parts = [p.strip().rstrip('.') for p in re.split(r'[,\n]', ot) if p.strip()]
    if parts and all(p in KW for p in parts) and not c.get('card_faces'):
        kw_only.append((name, c['mana_cost'], tl, parts, c['set'], c['collector_number']))
print('KEYWORD-ONLY CREATURES', len(kw_only))
for x in kw_only:
    print('  ', x, listof[x[0]])

# spell-text templates mapping to existing Effect variants
T = [
    (r'^Draw (two|three|four) cards\.$', 'draw-N'),
    (r'^Draw a card\.$', 'draw-1'),
    (r'^Scry (\d)\.$', 'scry'),
    (r'^Surveil (\d)\.$', 'surveil'),
    (r'^Scry (\d), then draw a card\.$', 'scry-draw'),
    (r'^You gain (\d+) life\.$', 'gain'),
    (r'^Target creature gets \+(\d+)\/\+(\d+) until end of turn\.$', 'pump+'),
    (r'^Target creature gets -(\d+)\/-(\d+) until end of turn\.$', 'pump-'),
    (r'^Return target creature to its owner\'s hand\.$', 'bounce'),
    (r'^Put target creature on top of its owner\'s library\.$', 'timeebb'),
    (r'^Counter target spell\.$', 'counter'),
    (r'^Exile target creature\. Its controller gains life equal to its power\.$', 'swords'),
    (r'^Prevent all combat damage that would be dealt this turn\.$', 'fog'),
    (r'^Prevent the next (\d+) damage that would be dealt to any target this turn\.$', 'shield'),
]
selfdmg = re.compile(r'^(?:\S+(?: \S+)*?) deals (\d+) damage to (any target|target creature|each creature|target player|each opponent)\.$')
hits = collections.defaultdict(list)
for name, c in sorted(cards.items()):
    ot = (c['oracle_text'] or '').strip()
    tl = c['type_line'] or ''
    for pat, tag in T:
        if re.match(pat, ot):
            hits[tag].append((name, c['mana_cost'], tl, ot))
            break
    else:
        mm = selfdmg.match(ot)
        if mm:
            hits['selfdamage-' + mm.group(2).replace(' ', '_')].append((name, c['mana_cost'], tl, ot))
        # ETB simple
        if re.match(r'^When \S.*? enters(?: the battlefield)?, draw a card\.$', ot):
            hits['etb-draw'].append((name, c['mana_cost'], tl, ot))
        if re.match(r'^When \S.*? enters(?: the battlefield)?, you gain \d+ life\.$', ot):
            hits['etb-gain'].append((name, c['mana_cost'], tl, ot))

print('\nSPELL TEMPLATE HITS')
for tag, xs in sorted(hits.items()):
    print(tag, len(xs))
    for x in xs:
        print('   ', x[0], '|', x[1], '|', x[3][:70], listof[x[0]])
