#!/usr/bin/env python3
"""Slice 35 corpus bookkeeping: Words of Wilding leaves the Ayula gap.

Removes the newly supported name from the Ayula `missing_cards` list only; the
desired deck snapshots and every other list are untouched.
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[3]
path = ROOT / "decks/starter-corpus/manifest.json"
text = path.read_text(encoding="utf-8")
manifest = json.loads(text)

removed = []
for starter in manifest["starters"]:
    if starter["id"] != "ayula-bear-counters":
        continue
    before = len(starter["missing_cards"])
    starter["missing_cards"] = [c for c in starter["missing_cards"] if c != "Words of Wilding"]
    removed.append((before, len(starter["missing_cards"])))

assert removed == [(16, 15)], removed
path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print("ayula missing_cards 16 -> 15")
