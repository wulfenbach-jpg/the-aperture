"""Fresh slice-23 corpus audit (independent of earlier slices' logs).

Recomputes the compiled pool size from catalog.rs, then the union, supported and
missing splits from the checked-in desired deck snapshots and the manifest's own
missing arrays. Asserts the numbers this slice reports.
"""
import json
import re
from pathlib import Path

root = Path("C:/AI/aperture-worktrees/issue-17")
src = (root / "crates/core/src/catalog.rs").read_text(encoding="utf-8")
pool = src.partition("pub static TOKENS")[0]
names = re.findall(r'card(?:_[a-z]+)?!\(\s*[A-Za-z0-9_]+,\s*"([^"]+)"', pool, re.S)
print("catalog pool entries:", len(names))
dups = [n for n in set(names) if names.count(n) > 1]
print("duplicate names:", dups)

manifest = json.loads((root / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))
union, missing_union = set(), set()
print("\nper-list: cards / missing / status")
for starter in manifest["starters"]:
    deck = (root / starter["deck_path"]).read_text(encoding="utf-8")
    entries = []
    for line in deck.splitlines():
        line = line.strip()
        if not line or line.startswith("//"):
            continue
        qty, _, name = line.partition(" ")
        entries.append((int(qty), name.strip()))
    total = sum(q for q, _ in entries)
    assert total == 99, (starter["id"], total)
    union.add(starter["commander"])
    union.update(n for _, n in entries)
    missing = set(starter["missing_cards"])
    missing_union |= missing
    bogus = sorted(missing & set(names))
    assert not bogus, (starter["id"], bogus)
    print(f"  {starter['id']:24s} {total:3d}  {len(missing):3d}  {starter['status']}")

pool_names = set(names)
supported = {n for n in union if n in pool_names}
missing_in_union = union - pool_names
print(f"\nunion names            : {len(union)}")
print(f"union supported        : {len(supported)}")
print(f"union missing          : {len(missing_in_union)}")
print(f"manifest missing union : {len(missing_union)}")
ayula = next(s for s in manifest["starters"] if s["id"] == "ayula-bear-counters")
print("ayula missing          :", len(ayula["missing_cards"]))
print("certified lists        :", [s["id"] for s in manifest["starters"] if s["status"] == "certified"])

assert len(names) == 142, len(names)
assert len(union) == 459, len(union)
assert len(supported) == 130, len(supported)
assert len(missing_in_union) == 329, len(missing_in_union)
assert missing_union == missing_in_union, (len(missing_union), len(missing_in_union))
assert len(ayula["missing_cards"]) == 23, len(ayula["missing_cards"])
assert [s["id"] for s in manifest["starters"] if s["status"] == "certified"] == [
    "jasmine-vanilla-stompy"
]
assert "Goreclaw, Terror of Qal Sisma" in pool_names
print("\nAUDIT OK: catalog 142, union 459 = 130 supported + 329 missing, Ayula 23, 1 certified")
