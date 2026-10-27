import json
import re
from pathlib import Path

live = json.loads(Path('evidence/issue17/review280/m19-304-live.json').read_text(encoding='utf-8'))
src = Path('crates/core/src/catalog.rs').read_text(encoding='utf-8')
ab = Path('crates/core/src/ability.rs').read_text(encoding='utf-8')
fixture = json.loads(Path('fixtures/starter-corpus/slice26-sourced.json').read_text(encoding='utf-8'))

expect = live['oracle_text']
anchor = src.index('UrsineChampion, "Ursine Champion"')
cat_block = src[anchor:anchor + 800]
m = re.search(r'oracle: "([^"]*)"', cat_block)
cat = m.group(1) if m else None
ab_block = ab[ab.index('const URSINE_CHAMPION') - 900:ab.index('const URSINE_CHAMPION') + 900]
m2 = re.search(r'text: "([^"]*)"', ab_block)
ability_text = m2.group(1) if m2 else None

print('live    :', expect)
print('fixture :', fixture['cards'][0]['oracle_text'])
print('catalog :', cat)
print('ability :', ability_text)
print('FIXTURE_MATCHES_LIVE', fixture['cards'][0]['oracle_text'] == expect)
print('CATALOG_MATCHES_LIVE', cat == expect)
print('ABILITY_MATCHES_LIVE', ability_text == expect)
for field in ['id', 'oracle_id', 'set', 'collector_number', 'mana_cost', 'type_line', 'power', 'toughness']:
    key = {'id': 'scryfall_id'}.get(field, field)
    print(f'  {field}: fixture={fixture["cards"][0].get(key)!r} live={live.get(field)!r}',
          'OK' if fixture['cards'][0].get(key) == live.get(field) else 'MISMATCH')
