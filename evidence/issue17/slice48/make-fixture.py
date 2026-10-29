import json

raw = json.load(open('evidence/issue17/slice46/sources/owlbear-cub.json', encoding='utf-8'))
rec = {
    "name": raw["name"],
    "oracle_id": raw["oracle_id"],
    "scryfall_id": raw["id"],
    "set": raw["set"],
    "collector_number": raw["collector_number"],
    "layout": raw["layout"],
    "mana_cost": raw["mana_cost"],
    "type_line": raw["type_line"],
    "power": raw.get("power"),
    "toughness": raw.get("toughness"),
    "oracle_text": raw["oracle_text"],
    "color_identity": raw["color_identity"],
    "keywords": raw.get("keywords", []),
    "legalities_commander": raw["legalities"]["commander"],
    "printing_url": "https://api.scryfall.com/cards/" + raw["id"],
    "retrieved_at": "2026-10-28T13:49:35.417772+00:00",
}
out = {
    "schema": "aperture.starter-sourced-cards.v1",
    "provider": "scryfall",
    "retrieved_at": "2026-10-28T13:49:35.417772+00:00",
    "cards": [rec],
}
json.dump(out, open('fixtures/starter-corpus/slice48-sourced.json', 'w', encoding='utf-8'),
          indent=1, ensure_ascii=False)
print(rec["oracle_text"])
print(rec["set"], rec["collector_number"], rec["mana_cost"], rec["power"], rec["toughness"])
