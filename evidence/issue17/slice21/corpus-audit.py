import json, re
from pathlib import Path

root = Path("C:/AI/aperture-worktrees/issue-17")
src = (root / "crates/core/src/catalog.rs").read_text(encoding="utf-8")
# Only the compiled-in pool, not the token table.
pool_src = src.partition("pub static TOKENS")[0]
names = re.findall(r'card(?:_[a-z]+)?!\(\s*[A-Za-z0-9_]+,\s*"([^"]+)"', pool_src, re.S)
print("catalog pool entries:", len(names))
print("duplicate names:", [n for n in set(names) if names.count(n) > 1])

man = json.loads((root / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))
union = set()
missing_union = set()
print("\nper-list: total / missing")
for s in man["starters"]:
    deck = (root / s["deck_path"]).read_text(encoding="utf-8")
    entries = []
    for line in deck.splitlines():
        line = line.strip()
        if not line or line.startswith("//"):
            continue
        qty, _, cardname = line.partition(" ")
        entries.append((int(qty), cardname.strip()))
    total = sum(q for q, _ in entries)
    union.add(s["commander"])
    union.update(n for _, n in entries)
    miss = set(s["missing_cards"])
    missing_union |= miss
    unsupported_but_not_listed = sorted(miss & set(names))
    print(f"  {s['id']:22s} {total:3d} cards  missing={len(miss):3d}  bogus_missing={unsupported_but_not_listed}")
    assert total == 99, (s["id"], total)
    assert set(s["missing_cards"]) == miss
    assert not unsupported_but_not_listed

catalog = set(names)
supported_union = {n for n in union if n in catalog}
missing_in_union = union - catalog
print(f"\nunion names            : {len(union)}")
print(f"union supported        : {len(supported_union)}")
print(f"union missing          : {len(missing_in_union)}")
print(f"manifest missing union : {len(missing_union)}")
assert len(union) == 459, len(union)
assert len(supported_union) == 126, len(supported_union)
assert len(missing_in_union) == 333, len(missing_in_union)
assert missing_union == missing_in_union, (len(missing_union), len(missing_in_union))
assert len(names) == 138, len(names)
print("\nAUDIT OK: catalog 138, union 459 = 126 supported + 333 missing")
