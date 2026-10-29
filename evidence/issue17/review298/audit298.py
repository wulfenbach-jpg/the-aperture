#!/usr/bin/env python3
"""Reviewer's own corpus audit (review 298): independent of the lane's script."""
import json, re, sys
from pathlib import Path

ROOT = Path(r"C:/AI/aperture-worktrees/issue-17")
CAT = (ROOT / "crates/core/src/catalog.rs").read_text(encoding="utf-8")

# Any `xxx!(Id, "Name"` macro invocation: capture the quoted name.
names = {}
for m in re.finditer(r'\b(\w+)!\(\s*\w+\s*,\s*"([^"]+)"', CAT):
    macro, name = m.group(1), m.group(2)
    if macro.startswith("card"):
        names.setdefault(name.lower(), set()).add(macro)
print("catalog names:", len(names))
print("macros seen:", sorted({m for s in names.values() for m in s}))

man = json.loads((ROOT / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))
print("manifest starters:", len(man["starters"]))

union = set()
rows = []
for st in man["starters"]:
    text = (ROOT / st["deck_path"]).read_text(encoding="utf-8")
    lib = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith(("#", "//")):
            continue
        m = re.match(r"^(\d+)\s+(.+)$", line)
        if m:
            lib.extend([m.group(2).strip()] * int(m.group(1)))
    cmd = st["commander"]
    assert len(lib) == 99, (st["id"], len(lib))
    assert cmd, st["id"]
    uniq = list(dict.fromkeys(lib + [cmd]))
    union.update(n.lower() for n in uniq)
    missing = sorted([n for n in uniq if n.lower() not in names], key=str.lower)
    declared = sorted(st["missing_cards"], key=str.lower)
    ok = missing == declared
    rows.append((st["id"], cmd, len(uniq), len(missing), ok, missing))
    if not ok:
        print("  MANIFEST MISMATCH", st["id"], "only-computed", set(missing) - set(declared), "only-manifest", set(declared) - set(missing))

print("union:", len(union))
supported = {n for n in union if n in names}
missing_union = sorted(union - supported, key=str.lower)
print("supported:", len(supported), "missing:", len(missing_union))
print("per list:", [(r[0], r[3]) for r in rows])
certified = [r[0] for r in rows if r[3] == 0]
print("certified:", certified, len(certified), "of", len(rows))
ayula = [r for r in rows if r[0].lower().startswith("ayula") or "ayula" in r[1].lower()]
for r in ayula:
    print("AYULA row:", r[0], "commander", r[1], "missing", r[3])
    print("  names:", r[5])
print("all manifests agree:", all(r[4] for r in rows))
