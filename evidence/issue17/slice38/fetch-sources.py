#!/usr/bin/env python3
"""Fetch authoritative printed text for the slice-38 candidate Ayula names.

One Scryfall /cards/collection call; the raw response and a retrieval stamp are
retained next to this script so every card's text can be re-checked later.
"""
import json
import pathlib
import urllib.error
import urllib.request
from datetime import datetime, timezone

HERE = pathlib.Path(__file__).resolve().parent
SRC = HERE / "sources"
SRC.mkdir(parents=True, exist_ok=True)

NAMES = [
    "Dúnedain Rangers",
    "Evercoat Ursine",
    "Lightning Greaves",
    "Ruxa, Patient Professor",
    "Spirit of the Aldergard",
    "Swiftfoot Boots",
    "Titania's Command",
]

req = urllib.request.Request(
    "https://api.scryfall.com/cards/collection",
    data=json.dumps({"identifiers": [{"name": n} for n in NAMES]}).encode(),
    headers={
        "User-Agent": "aperture-corpus/1.0",
        "Accept": "application/json",
        "Content-Type": "application/json",
    },
)
stamp = datetime.now(timezone.utc).isoformat()
try:
    with urllib.request.urlopen(req, timeout=90) as resp:
        payload = json.load(resp)
except urllib.error.HTTPError as err:
    print("HTTP", err.code, err.read().decode("utf-8", "replace"))
    raise
(SRC / "collection-raw.json").write_text(json.dumps(payload, indent=1), encoding="utf-8")
(SRC / "retrieval.json").write_text(
    json.dumps({"retrieved_at": stamp, "endpoint": "/cards/collection", "names": NAMES}, indent=1),
    encoding="utf-8",
)

print("retrieved_at", stamp)
print("not_found", payload.get("not_found"))
for card in payload["data"]:
    print("==", card["name"], "|", card["set"], card["collector_number"],
          "|", card.get("mana_cost"), "|", card["type_line"],
          "|", card.get("power"), "/", card.get("toughness"),
          "| kw", card.get("keywords"))
    print("   oracle_id", card["oracle_id"], "scryfall_id", card["id"])
    print("   colors", card.get("colors"), "ci", card.get("color_identity"),
          "mv", card.get("cmc"))
    print("   text:", repr(card.get("oracle_text")))
