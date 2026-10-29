import json

d = json.load(open('evidence/issue17/slice15/sources-ayula-raw.json'))
for c in sorted(d['cards'], key=lambda c: c['name']):
    text = (c['oracle_text'] or '').replace('\n', ' / ')
    print(f"{c['name']} | {c['mana_cost']} | {c['type_line']} | {c['power']}/{c['toughness']} | kw={c['keywords']} | {text}")
