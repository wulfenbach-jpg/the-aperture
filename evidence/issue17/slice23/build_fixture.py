"""Build fixtures/starter-corpus/slice23-sourced.json from the raw Scryfall record.

The raw bytes were fetched this run into
evidence/issue17/slice23/raw/cards-collection.json; this script only selects the
recorded printing (CMM 293) and normalizes the fields the repository fixture
schema uses. Nothing is authored by hand.
"""
import datetime
import json
import pathlib

root = pathlib.Path("C:/AI/aperture-worktrees/issue-17")
raw = json.loads(
    (root / "evidence/issue17/slice23/raw/cards-collection.json").read_text(encoding="utf-8")
)
card = next(
    c
    for c in raw["data"]
    if c["name"] == "Goreclaw, Terror of Qal Sisma" and c["set"] == "cmm" and c["collector_number"] == "293"
)
retrieved_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
fixture = {
    "schema": "aperture.starter-sourced-cards.v1",
    "provider": "scryfall",
    "retrieved_at": retrieved_at,
    "cards": [
        {
            "name": card["name"],
            "oracle_id": card["oracle_id"],
            "scryfall_id": card["id"],
            "set": card["set"],
            "collector_number": card["collector_number"],
            "layout": card["layout"],
            "mana_cost": card["mana_cost"],
            "type_line": card["type_line"],
            "power": card.get("power"),
            "toughness": card.get("toughness"),
            "oracle_text": card["oracle_text"],
            "color_identity": card["color_identity"],
            "keywords": card["keywords"],
            "legalities_commander": card["legalities"]["commander"],
            "printing_url": f"https://api.scryfall.com/cards/{card['id']}",
            "retrieved_at": retrieved_at,
        }
    ],
}
out = root / "fixtures/starter-corpus/slice23-sourced.json"
out.write_text(json.dumps(fixture, indent=1) + "\n", encoding="utf-8")
print(out)
print(json.dumps(fixture["cards"][0], indent=1))
