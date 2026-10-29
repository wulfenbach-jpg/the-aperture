"""review-289 independent corpus audit.

Supported names come from the candidate crate itself (catalog::all() dumped by
C:/AI/r289probe/src/bin/dump_catalog.rs), not from a regex over catalog.rs.
Deck content comes from the eight manifest deck files.
"""
import json
import re
import sys

ROOT = "C:/AI/aperture-worktrees/issue-17"
NAMES = [line.strip() for line in open("C:/AI/r289-names.txt", encoding="utf-8") if line.strip()]
supported = {n.lower() for n in NAMES}
print("catalog_rows =", len(NAMES))
assert len(supported) == len(NAMES), "duplicate names in the pool"

manifest = json.load(open(f"{ROOT}/decks/starter-corpus/manifest.json", encoding="utf-8"))
union = set()
rows = []
disagreements = []
for starter in manifest["starters"]:
    text = open(f"{ROOT}/{starter['deck_path']}", encoding="utf-8").read()
    cards = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("//") or line.startswith("#"):
            continue
        m = re.match(r"^(\d+)\s+(.+?)\s*$", line)
        if not m:
            print("UNPARSED LINE", starter["id"], repr(line))
            sys.exit(1)
        cards.extend([m.group(2)] * int(m.group(1)))
    assert len(cards) == 99, f"{starter['id']}: {len(cards)} library cards"
    names = list(dict.fromkeys(cards + [starter["commander"]]))
    union.update(n.lower() for n in names)
    missing = sorted({n for n in names if n.lower() not in supported}, key=str.lower)
    declared = sorted(starter["missing_cards"], key=str.lower)
    if missing != declared:
        disagreements.append(
            (
                starter["id"],
                sorted(set(missing) - set(declared)),
                sorted(set(declared) - set(missing)),
            )
        )
    rows.append((starter["id"], starter["status"], len(missing), len(names)))

unsupported = {n for n in union if n not in supported}
print("union_names   =", len(union))
print("supported     =", len(union) - len(unsupported))
print("missing       =", len(unsupported))
for sid, status, miss, total in rows:
    print(f"  {sid:24s} status={status:9s} distinct={total:3d} missing={miss}")
print("per_list_missing =", [r[2] for r in rows])
print("per_list_total   =", [r[3] for r in rows])
print("certified        =", sum(1 for r in rows if r[1] == "certified"),
      [r[0] for r in rows if r[1] == "certified"])
print("manifest_disagreements =", disagreements)
ayula = [r for r in rows if r[0] == "ayula-bear-counters"][0]
print("ayula_missing =", ayula[2])
