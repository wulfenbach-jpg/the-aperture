import json, urllib.request, pathlib, time

root = pathlib.Path(__file__).resolve().parents[3]
m = json.loads((root / 'decks/starter-corpus/manifest.json').read_text())
miss = sorted({n for s in m['starters'] for n in s['missing_cards']})
print('missing union', len(miss))


def post(names):
    body = json.dumps({"identifiers": [{"name": n} for n in names]}).encode()
    req = urllib.request.Request(
        "https://api.scryfall.com/cards/collection", data=body,
        headers={"Content-Type": "application/json",
                 "User-Agent": "aperture-corpus/1.0",
                 "Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.loads(r.read())


found, notfound = {}, []
for i in range(0, len(miss), 75):
    chunk = miss[i:i + 75]
    d = post(chunk)
    for c in d.get('data', []):
        found[c['name']] = c
    for nf in d.get('not_found', []):
        notfound.append(nf.get('name'))
    time.sleep(0.2)
    print(i, len(found), len(notfound))

out = pathlib.Path(__file__).resolve().parent
(out / 'scryfall-missing-union.json').write_text(json.dumps({k: {
    "name": v["name"], "mana_cost": v.get("mana_cost"), "cmc": v.get("cmc"),
    "type_line": v.get("type_line"), "oracle_text": v.get("oracle_text", ""),
    "power": v.get("power"), "toughness": v.get("toughness"),
    "colors": v.get("colors"), "color_identity": v.get("color_identity"),
    "keywords": v.get("keywords"), "set": v.get("set"),
    "collector_number": v.get("collector_number"), "released_at": v.get("released_at"),
    "layout": v.get("layout"), "card_faces": v.get("card_faces"),
} for k, v in found.items()}, indent=1))
print('notfound', notfound)
print('found', len(found))
