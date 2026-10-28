import json, datetime, shutil, os

os.makedirs('evidence/issue17/slice43/sources', exist_ok=True)
src = os.path.join(os.environ['TMPDIR'], 'lumra.json')
if not os.path.exists('evidence/issue17/slice43/sources/lumra-blb-183.json'):
    shutil.copy(src, 'evidence/issue17/slice43/sources/lumra-blb-183.json')
raw = json.load(open('evidence/issue17/slice43/sources/lumra-blb-183.json'))
stamp = datetime.datetime.now(datetime.timezone.utc).isoformat()
card = {k: raw[k] for k in ('name', 'oracle_id', 'set', 'collector_number', 'layout',
                            'mana_cost', 'type_line', 'oracle_text', 'color_identity',
                            'keywords', 'power', 'toughness')}
card['scryfall_id'] = raw['id']
card['legalities_commander'] = raw['legalities']['commander']
card['printing_url'] = raw['uri']
card['retrieved_at'] = stamp
fixture = {"schema": "aperture.starter-sourced-cards.v1", "provider": "scryfall",
           "retrieved_at": stamp, "cards": [card]}
json.dump(fixture, open('fixtures/starter-corpus/slice43-sourced.json', 'w'), indent=2,
          ensure_ascii=False)
print(json.dumps(card, indent=1))
