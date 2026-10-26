"""Slice-11 native corpus audit.

Recomputes, from the tracked files themselves, the numbers this slice reports:
the supported catalog size, the eight-list union, and per-starter supported /
missing names with the stale entries the manifest still lists.
"""

from pathlib import Path
import json
import re

ROOT = Path(".")
manifest = json.loads((ROOT / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))

card_re = re.compile(r'\bcard(?:_[a-z]+)?!\(\s*\w+,\s*"([^"]+)"')
catalog = set(card_re.findall((ROOT / "crates/core/src/catalog.rs").read_text(encoding="utf-8")))
tokens = set(card_re.findall((ROOT / "crates/core/src/tokens.rs").read_text(encoding="utf-8")))
supported = catalog | tokens

union = set()
per_list = {}
for entry in manifest["starters"]:
    names = {entry["commander"]}
    for line in (ROOT / entry["deck_path"]).read_text(encoding="utf-8").splitlines():
        match = re.match(r"^\s*\d+\s+(.+?)\s*$", line)
        if match:
            names.add(match.group(1))
    missing = sorted(names - supported)
    stale = sorted(set(entry["missing_cards"]) - (names - supported))
    per_list[entry["id"]] = {
        "status": entry["status"],
        "unique_names": len(names),
        "supported": len(names & supported),
        "missing": len(missing),
        "missing_names": missing,
        "manifest_missing_entries": len(entry["missing_cards"]),
        "manifest_entries_that_are_now_supported": stale,
    }
    union |= names

result = {
    "catalog_cards": len(catalog),
    "token_definitions": len(tokens),
    "supported_names": len(supported),
    "union_names": len(union),
    "union_supported": len(union & supported),
    "union_missing": len(union - supported),
    "implemented_lists": sum(1 for e in manifest["starters"] if e["status"] in ("implemented", "certified")),
    "certified_lists": sum(1 for e in manifest["starters"] if e["status"] == "certified"),
    "starters": len(manifest["starters"]),
    "per_list": per_list,
}
print(json.dumps({k: v for k, v in result.items() if k != "per_list"}, indent=2))
for sid, row in per_list.items():
    print(f"{sid:26s} status={row['status']:10s} names={row['unique_names']:3d} supported={row['supported']:3d} missing={row['missing']:3d} manifest_missing={row['manifest_missing_entries']:3d} stale={len(row['manifest_entries_that_are_now_supported'])}")
Path("evidence/issue17/slice11").mkdir(parents=True, exist_ok=True)
Path("evidence/issue17/slice11/corpus-audit.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
