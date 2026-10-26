"""Slice 16 bookkeeping: sourced fixture + manifest missing_cards.

Reads the freshly fetched Scryfall records (immutable printing URLs and
retrieval stamps) and writes the two-card fixture, then removes exactly those
two names from the Ayula list's missing_cards.
"""

import json

RAW = "evidence/issue17/slice16/sources-ayula-raw.json"
OUT = "fixtures/starter-corpus/slice16-sourced.json"
MANIFEST = "decks/starter-corpus/manifest.json"
WANTED = ["Fade from History", "Little Bear"]

raw = json.load(open(RAW, encoding="utf-8"))
by_name = {c["name"]: c for c in raw["cards"]}
records = []
for name in WANTED:
    c = by_name[name]
    records.append(
        {
            "name": c["name"],
            "oracle_id": c["oracle_id"],
            "scryfall_id": c["scryfall_id"],
            "set": c["set"],
            "collector_number": c["collector_number"],
            "layout": c["layout"],
            "mana_cost": c["mana_cost"],
            "type_line": c["type_line"],
            "power": c["power"],
            "toughness": c["toughness"],
            "oracle_text": c["oracle_text"],
            "color_identity": c["color_identity"],
            "keywords": c["keywords"],
            "legalities_commander": c["legalities_commander"],
            "printing_url": c["printing_url"],
            "retrieved_at": c["retrieved_at"],
        }
    )

fixture = {
    "schema": "aperture.starter-sourced-cards.v1",
    "provider": "scryfall",
    "retrieved_at": raw["retrieved_at"],
    "note": "Slice-16 sourced cards: two records, one per card (Fade from History, Little Bear). Each record is the exact Scryfall printing the catalog string was copied from.",
    "cards": records,
}
with open(OUT, "w", encoding="utf-8") as fh:
    json.dump(fixture, fh, indent=1, ensure_ascii=False)

manifest = json.load(open(MANIFEST, encoding="utf-8"))
ayula = [s for s in manifest["starters"] if s["id"] == "ayula-bear-counters"][0]
before = len(ayula["missing_cards"])
ayula["missing_cards"] = [n for n in ayula["missing_cards"] if n not in WANTED]
ayula["missing_interactions"] = [
    "commander fight/counter trigger (Ayula itself)",
    "equipment (Lightning Greaves, Swiftfoot Boots)",
    "soulbond, hideaway, saddled mounts and Ring temptation",
]
with open(MANIFEST, "w", encoding="utf-8") as fh:
    json.dump(manifest, fh, indent=1, ensure_ascii=False)
    fh.write("\n")

print("fixture cards:", [r["name"] for r in records])
print("ayula missing:", before, "->", len(ayula["missing_cards"]))
print("still missing:", ", ".join(ayula["missing_cards"]))
