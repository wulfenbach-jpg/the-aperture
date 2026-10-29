#!/usr/bin/env python3
"""Native independent corpus audit (slice 41: The Earth King).

Reads the real sources of truth -- catalog.rs for supported names and the
desired deck files -- and recomputes the eight-list union. Fails loudly on any
manifest disagreement or on a deck that is not exactly 99 + 1.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CATALOG = (ROOT / "crates/core/src/catalog.rs").read_text(encoding="utf-8")
MANIFEST = json.loads((ROOT / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))

# `card!(Id, "Name", ...` and the helper macros card_artifact!/card_defender!/...
names = set()
for macro in ("card", "card_artifact", "card_defender", "card_flash", "card_green", "card_islandwalk", "card_trample"):
    for m in re.finditer(rf"\b{macro}!\(\s*\w+,\s*\"([^\"]+)\"", CATALOG):
        names.add(m.group(1).lower())
assert len(names) == 153, f"catalog parse found {len(names)} names, expected 153"

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
    declared = sorted(starter["missing_cards"], key=str.lower)
    assert missing == declared, (
        f"{starter['id']}: computed missing {len(missing)} != manifest {len(declared)}\n"
        f"  only-computed={sorted(set(missing) - set(declared))}\n"
        f"  only-manifest={sorted(set(declared) - set(missing))}"
    )
    report.append((starter["id"], starter["status"], len(missing)))

union_size = len(union)
supported = union_size - sum(1 for n in union if n not in names)
missing_union = union_size - supported
print(f"catalog_cards      = {len(names)}")
print(f"union_names        = {union_size}")
print(f"supported_union    = {supported}")
print(f"missing_union      = {missing_union}")
for sid, status, missing in report:
    print(f"  {sid:26s} status={status:9s} missing={missing}")
print("per_list_missing   =", [m for _, _, m in report])
certified = [sid for sid, status, _ in report if status == "certified"]
print("certified_lists    =", len(certified), certified)
assert union_size == 459, union_size
assert supported == 141 and missing_union == 318, (supported, missing_union)
assert [m for _, _, m in report] == [55, 0, 51, 48, 12, 52, 52, 53]
print("AUDIT OK")
