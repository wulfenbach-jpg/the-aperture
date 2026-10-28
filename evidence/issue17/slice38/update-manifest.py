#!/usr/bin/env python3
"""Record Spider-Ham, Peter Porker as supported in the starter-corpus manifest.

The manifest's per-list `missing_cards` is the public record of what each desired
list still needs; a card that lands in the pool must leave that list. Written as
a script so the CRLF/LF shape and key order of the file are preserved by
json.dump of the loaded document (the file is regenerated, not hand-edited).
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[3]
PATH = ROOT / "decks/starter-corpus/manifest.json"

manifest = json.loads(PATH.read_text(encoding="utf-8"))
ayula = next(s for s in manifest["starters"] if s["id"] == "ayula-bear-counters")
before = len(ayula["missing_cards"])
ayula["missing_cards"] = [n for n in ayula["missing_cards"] if n != "Spider-Ham, Peter Porker"]
assert len(ayula["missing_cards"]) == before - 1, "Spider-Ham was not in the list"
ayula["missing_interactions"] = [
    "equipment (Lightning Greaves, Swiftfoot Boots)",
    "library search / hideaway / Ring temptation (Dúnedain Rangers, Evercoat Ursine, Spirit of the Aldergard, The Earth King, Titania's Command)",
    "combat-damage-as-though-unblocked (Ruxa) and modal spells (Titania's Command)",
]
PATH.write_text(json.dumps(manifest, indent=1) + "\n", encoding="utf-8")
print("ayula missing_cards:", before, "->", len(ayula["missing_cards"]))
print("remaining:", ", ".join(ayula["missing_cards"]))
