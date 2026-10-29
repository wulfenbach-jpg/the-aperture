"""Build the slice-15 provenance fixture from the fetched Ayula sources.

The three records are copied verbatim from `sources-ayula-raw.json` (one
Scryfall printing each, fetched by this slice), so the fixture cannot drift from
the authoritative text the catalog strings were copied from.
"""

import json

WANTED = ["Ayula's Influence", "Bearscape", "Mother Bear"]
raw = json.load(open("evidence/issue17/slice15/sources-ayula-raw.json"))
by_name = {card["name"]: card for card in raw["cards"]}
cards = []
for name in WANTED:
    card = by_name[name]
    cards.append(
        {
            "name": card["name"],
            "oracle_id": card["oracle_id"],
            "scryfall_id": card["scryfall_id"],
            "set": card["set"],
            "collector_number": card["collector_number"],
            "layout": card["layout"],
            "mana_cost": card["mana_cost"],
            "type_line": card["type_line"],
            "power": card["power"],
            "toughness": card["toughness"],
            "oracle_text": card["oracle_text"],
            "color_identity": card["color_identity"],
            "keywords": card["keywords"],
            "legalities_commander": card["legalities_commander"],
            "printing_url": card["printing_url"],
            "retrieved_at": card["retrieved_at"],
        }
    )
out = {
    "schema": "aperture.starter-sourced-cards.v1",
    "provider": "scryfall",
    "retrieved_at": raw["retrieved_at"],
    "note": "Slice-15 Bear-token package: three records, one per sourced card. "
    "Each record is the exact Scryfall printing the catalog string was copied from.",
    "cards": cards,
}
with open("fixtures/starter-corpus/slice15-sourced.json", "w", encoding="utf-8") as fh:
    json.dump(out, fh, indent=1, ensure_ascii=False)
print("wrote", len(cards), "records")
for card in cards:
    print(" ", card["name"], card["set"], card["collector_number"], card["mana_cost"])
