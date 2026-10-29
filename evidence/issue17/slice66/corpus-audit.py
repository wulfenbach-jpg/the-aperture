#!/usr/bin/env python3
"""Slice66 implementer corpus recount and whole-field gap audit (not review).

Reads the real sources of truth -- catalog.rs for supported names and the eight
desired deck files -- and recomputes the union, the per-list gaps and the
declared interaction fields. Fails loudly on any manifest disagreement.
"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CATALOG = (ROOT / "crates/core/src/catalog.rs").read_text(encoding="utf-8")
MANIFEST = json.loads((ROOT / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))

names = set()
for macro in ("card", "card_artifact", "card_defender", "card_flash", "card_green",
              "card_islandwalk", "card_trample", "card_vigilance_trample", "card_flying_haste"):
    for m in re.finditer(rf"\b{macro}!\(\s*\w+,\s*\"([^\"]+)\"", CATALOG):
        names.add(m.group(1).lower())
assert len(names) == 179, f"catalog parse found {len(names)} names, expected 179"

union = set()
report = []
for starter in MANIFEST["starters"]:
    text = (ROOT / starter["deck_path"]).read_text(encoding="utf-8")
    cards = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#") or line.startswith("//"):
            continue
        m = re.match(r"^(\d+)\s+(.+)$", line)
        if m:
            cards.extend([m.group(2).strip()] * int(m.group(1)))
    assert len(cards) == 99, f"{starter['id']}: {len(cards)} library cards, expected 99"
    deck_names = list(dict.fromkeys(cards + [starter["commander"]]))
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

# Whole-field interaction check: no interaction entry may cite a supported card.
supported_names = {n for n in names}
for starter in MANIFEST["starters"]:
    for entry in starter["missing_interactions"]:
        for candidate in supported_names:
            assert candidate not in entry.lower() or candidate == "goblin", (
                f"{starter['id']} interaction cites supported card {candidate!r}: {entry}"
            )

assert union_size == 459, union_size
assert [m for _, _, m in report] == [55, 0, 51, 36, 0, 52, 52, 53], [m for _, _, m in report]
print("AUDIT OK")
