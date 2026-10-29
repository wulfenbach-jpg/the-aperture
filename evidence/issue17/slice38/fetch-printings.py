#!/usr/bin/env python3
"""List every Scryfall printing of Spider-Ham, Peter Porker and check the oracle
text is identical across them (provenance note for the slice-38 record)."""
import json
import pathlib
import urllib.parse
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
SRC = HERE / "sources"

query = '!"Spider-Ham, Peter Porker" unique:prints'
url = "https://api.scryfall.com/cards/search?q=" + urllib.parse.quote(query)
req = urllib.request.Request(
    url, headers={"User-Agent": "aperture-corpus/1.0", "Accept": "application/json"}
)
with urllib.request.urlopen(req, timeout=90) as resp:
    payload = json.load(resp)
(SRC / "printings.json").write_text(json.dumps(payload, indent=1), encoding="utf-8")
texts = {card["oracle_text"] for card in payload["data"]}
print("url", url)
print("printings", [(c["set"], c["collector_number"]) for c in payload["data"]])
print("distinct oracle texts", len(texts))
print("distinct type lines", {c["type_line"] for c in payload["data"]})
print("distinct costs", {c["mana_cost"] for c in payload["data"]})
print("distinct pt", {(c.get("power"), c.get("toughness")) for c in payload["data"]})
