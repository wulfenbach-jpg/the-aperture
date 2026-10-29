#!/usr/bin/env python3
"""Fetch Spider-Ham, Peter Porker (SPM 114) from Scryfall by set/number.

The multi-card collection call in fetch-sources.py covered the other seven
candidate names; this one fetches the card this slice implements, from its
authoritative printing, and retains the raw response and a retrieval stamp.
"""
import json
import pathlib
import urllib.request
from datetime import datetime, timezone

HERE = pathlib.Path(__file__).resolve().parent
SRC = HERE / "sources"
SRC.mkdir(parents=True, exist_ok=True)

url = "https://api.scryfall.com/cards/spm/114"
stamp = datetime.now(timezone.utc).isoformat()
req = urllib.request.Request(
    url, headers={"User-Agent": "aperture-corpus/1.0", "Accept": "application/json"}
)
with urllib.request.urlopen(req, timeout=90) as resp:
    card = json.load(resp)
(SRC / "spider-ham-spm114.json").write_text(json.dumps(card, indent=1), encoding="utf-8")
(SRC / "retrieval-spm114.json").write_text(
    json.dumps({"retrieved_at": stamp, "endpoint": url, "name": card["name"]}, indent=1),
    encoding="utf-8",
)
print("retrieved_at", stamp)
print(card["name"], "|", card["set"], card["collector_number"], "|", card["mana_cost"],
      "|", card["type_line"], "|", card.get("power"), "/", card.get("toughness"),
      "| kw", card.get("keywords"))
print("oracle_id", card["oracle_id"], "scryfall_id", card["id"])
print("legalities.commander", card["legalities"]["commander"])
print("text:", repr(card["oracle_text"]))
