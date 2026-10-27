"""Review 296 independent corpus audit.

Recomputes the eight-list union from the deck files themselves, compares it with
each starter's recorded `missing_cards`, and (when given a supported-name list)
splits the union into supported / missing. Written by the reviewer; it does not
reuse the lane's audit tool.
"""
import json
import subprocess
import sys

ROOT = "C:/AI/aperture-worktrees/issue-17"

manifest = json.load(open(f"{ROOT}/decks/starter-corpus/manifest.json", encoding="utf-8"))

union = set()
rows = []
for starter in manifest["starters"]:
    path = f"{ROOT}/{starter['deck_path']}"
    names = []
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if not line or line.startswith("#") or line.startswith("//"):
            continue
        if line.lower().startswith("commander:"):
            continue
        # Deck lines are "<qty> <name>".
        if line[0].isdigit():
            line = line.split(None, 1)[1]
        names.append(line.strip())
    cards = set(names)
    cards.add(starter["commander"])
    recorded = set(starter.get("missing_cards", []))
    rows.append(
        {
            "id": starter["id"],
            "status": starter.get("status"),
            "cards": len(cards),
            "recorded_missing": len(recorded),
            "_cards": cards,
            "_recorded": recorded,
        }
    )
    union |= cards

print(f"union names: {len(union)}")

supported = None
if len(sys.argv) > 1:
    supported = {line.strip() for line in open(sys.argv[1], encoding="utf-8") if line.strip()}
    print(f"supported (from {sys.argv[1]}): {len(supported)}")

ok = True
for row in rows:
    cards = row.pop("_cards")
    recorded = row.pop("_recorded")
    line = f"{row['id']:28s} status={row['status']:10s} cards={row['cards']:3d} missing={row['recorded_missing']:3d}"
    if supported is not None:
        missing = cards - supported
        line += f" recomputed_missing={len(missing):3d} recorded_matches={missing == recorded}"
        if missing != recorded:
            ok = False
    print(line)

if supported is not None:
    missing_union = union - supported
    print(f"union {len(union)} = supported {len(union & supported)} + missing {len(missing_union)}")
    certified = []
    for row_id in [r["id"] for r in rows]:
        pass
    # A list is certified only when every one of its own names is supported AND
    # the certification evidence says so; here we report the name-level fact.
    for starter in manifest["starters"]:
        path = f"{ROOT}/{starter['deck_path']}"
        names = {
            line.split(None, 1)[1].strip() if line.strip()[0].isdigit() else line.strip()
            for line in open(path, encoding="utf-8")
            if line.strip() and not line.strip().startswith(("#", "//"))
            and not line.strip().lower().startswith("commander:")
        }
        names.add(starter["commander"])
        fully_supported = names <= supported
        print(f"list {starter['id']:28s} fully_supported={fully_supported} unsupported={len(names - supported)}")
    print("ALL_RECORDED_MISSING_MATCH", ok)
