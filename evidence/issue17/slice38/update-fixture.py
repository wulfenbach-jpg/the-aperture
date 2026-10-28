#!/usr/bin/env python3
"""Build fixtures/starter-corpus/slice38-sourced.json from the retained Scryfall
collection response (slice 38: Spider-Ham, Peter Porker)."""
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
RAW = HERE / "sources" / "collection-raw.json"
OUT = pathlib.Path("fixtures/starter-corpus/slice38-sourced.json")

payload = None
card = json.loads((HERE / "sources" / "spider-ham-spm114.json").read_text(encoding="utf-8"))
record = {
    "name": card["name"],
    "oracle_id": card["oracle_id"],
    "scryfall_id": card["id"],
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
    "legalities_commander": card["legalities"]["commander"],
    "printing_url": card["scryfall_uri"].replace("?utm_source=api", ""),
    "retrieved_at": json.loads(
        (HERE / "sources" / "retrieval-spm114.json").read_text(encoding="utf-8")
    )["retrieved_at"],
}
fixture = {
    "schema": "aperture.starter-sourced-cards.v1",
    "provider": "scryfall",
    "retrieved_at": record["retrieved_at"],
    "cards": [record],
}
OUT.write_text(json.dumps(fixture, indent=1), encoding="utf-8")
print(OUT, "written;", record["set"], record["collector_number"], record["scryfall_id"])
print(repr(record["oracle_text"]))
