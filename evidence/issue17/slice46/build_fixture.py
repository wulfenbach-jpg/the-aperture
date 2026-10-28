import json

d = json.load(open('evidence/issue17/slice46/sources/thijarian-witness.json'))
s = json.load(open('evidence/issue17/slice46/sources/thijarian-witness.stamp.json'))
cards = [{
    "name": d["name"],
    "oracle_id": d["oracle_id"],
    "scryfall_id": d["id"],
    "set": d["set"],
    "collector_number": d["collector_number"],
    "layout": d["layout"],
    "mana_cost": d["mana_cost"],
    "type_line": d["type_line"],
    "power": d["power"],
    "toughness": d["toughness"],
    "oracle_text": d["oracle_text"],
    "color_identity": d["color_identity"],
    "keywords": d["keywords"],
    "legalities_commander": d["legalities"]["commander"],
    "printing_url": d["uri"],
    "retrieved_at": s["retrieved_utc"],
}]
out = {
    "schema": "aperture.starter-sourced-cards.v1",
    "provider": "scryfall",
    "retrieved_at": s["retrieved_utc"],
    "cards": cards,
}
path = 'fixtures/starter-corpus/slice46-sourced.json'
with open(path, 'w', encoding='utf-8', newline='\n') as handle:
    handle.write(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
print(open(path, encoding='utf-8').read())
