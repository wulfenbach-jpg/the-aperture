#!/usr/bin/env python3
"""Slice 69 corpus recount and whole-field gap audit (implementer evidence).

Same sources of truth as `evidence/issue17/slice54/corpus-audit.py` -- catalog.rs
for supported names and the desired deck files -- recomputed for slice 69. It
fails loudly on any manifest disagreement, on a deck that is not exactly 99 + 1,
and on any `missing_interactions` entry that names a card the catalog already
supports.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CATALOG = (ROOT / "crates/core/src/catalog.rs").read_text(encoding="utf-8")
MANIFEST = json.loads((ROOT / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))

names = set()
for macro in ("card", "card_artifact", "card_defender", "card_flash", "card_green",
              "card_islandwalk", "card_trample", "card_flying_haste", "card_vigilance_trample"):
    for m in re.finditer(rf"\b{macro}!\(\s*\w+,\s*\"([^\"]+)\"", CATALOG):
        names.add(m.group(1).lower())
declared = re.search(r"pub static CARDS: \[CardDef; (\d+)\] = \[", CATALOG)
assert declared, "catalog array size not found"
assert len(names) == int(declared.group(1)), (
    f"catalog parse found {len(names)} names, array says {declared.group(1)}"
)

union = set()
report = []
for starter in MANIFEST["starters"]:
    path = ROOT / starter["deck_path"]
    text = path.read_text(encoding="utf-8")
    cards = []
    commander = starter["commander"]
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#") or line.startswith("//"):
            continue
        m = re.match(r"^(\d+)\s+(.+)$", line)
        if m:
            count, name = int(m.group(1)), m.group(2).strip()
            cards.extend([name] * count)
    assert len(cards) == 99, f"{starter['id']}: {len(cards)} library cards, expected 99"
    assert commander, f"{starter['id']}: no Commander line"
    deck_names = list(dict.fromkeys(cards + [commander]))
    union.update(n.lower() for n in deck_names)
    missing = sorted({n for n in deck_names if n.lower() not in names}, key=str.lower)
    declared_missing = sorted(starter["missing_cards"], key=str.lower)
    assert missing == declared_missing, (
        f"{starter['id']}: manifest missing_cards disagree\n"
        f"  computed: {missing}\n  declared: {declared_missing}"
    )
    for entry in starter["missing_interactions"]:
        for named in re.findall(r"[A-Z][A-Za-z'\-]+(?:,? (?:the )?[A-Z][A-Za-z'\-]+)*", entry):
            if named.lower() in names:
                print(f"STALE INTERACTION {starter['id']}: {named}", file=sys.stderr)
                sys.exit(1)
    report.append((starter["id"], len(missing), len(starter["missing_interactions"])))

supported = {n for n in union if n in names}
missing_union = union - names
certified = [
    s["id"]
    for s in MANIFEST["starters"]
    if not s["missing_cards"] and not s["missing_interactions"]
]
print(
    f"catalog={len(names)} union={len(union)} supported={len(supported)} "
    f"missing={len(missing_union)}"
)
print("per-list missing:", [m for _, m, _ in report])
print("per-list interactions:", [i for _, _, i in report])
print("certified:", certified)
total = len(supported) + len(missing_union)
assert total == len(union), f"split {total} != union {len(union)}"
assert len(names) == 183, f"expected the slice-69 pool of 183, found {len(names)}"
assert len(union) == 459, f"expected the 459-name union, found {len(union)}"
assert len(supported) == 169 and len(missing_union) == 290, (
    "split is not the recorded 169 + 290"
)
assert len(certified) == 2, "the certified count must stay 2 of 8"
print("AUDIT OK")
