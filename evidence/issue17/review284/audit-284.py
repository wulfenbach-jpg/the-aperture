"""Review 284 independent corpus audit for issue #17 slice 28.

Reads the compiled catalog source of truth (card! macro entries), the eight
desired starter deck files and the manifest, and recomputes:
  * supported catalog size,
  * per-list referenced names / missing names,
  * union of referenced names, supported and missing split,
  * how many of the eight lists are certified (zero missing).
It also cross-checks the manifest's own missing_cards arrays against a fresh
computation from the deck files, so a stale manifest cannot hide.
Run: python evidence/issue17/review284/audit-284.py
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CATALOG = ROOT / "crates/core/src/catalog.rs"
MANIFEST = ROOT / "decks/starter-corpus/manifest.json"

CARD_MACROS = ("card!", "card_artifact!", "card_defender!", "card_flash!",
               "card_islandwalk!", "card_trample!")

BASIC_LANDS = {"Plains", "Island", "Swamp", "Mountain", "Forest"}


def catalog_names() -> list[str]:
    text = CATALOG.read_text(encoding="utf-8")
    pattern = re.compile(
        r"card(?:_[a-z]+)?!\s*\(\s*[A-Za-z0-9_]+\s*,\s*\"((?:[^\"\\]|\\.)*)\"")
    names = [m.group(1).replace('\\"', '"') for m in pattern.finditer(text)]
    if not names:
        raise SystemExit("no catalog entries parsed")
    return names


def deck_names(path: Path) -> list[str]:
    names = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("//") or line.startswith("#"):
            continue
        m = re.match(r"^(\d+)\s*(?:x\s+)?(.+)$", line)
        if not m:
            raise SystemExit(f"cannot parse deck line in {path}: {raw!r}")
        names.append(m.group(2).strip())
    return names


def main() -> int:
    supported = catalog_names()
    supported_set = set(supported)
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))

    union: set[str] = set()
    rows = []
    problems = []
    certified = 0
    for starter in manifest["starters"]:
        path = ROOT / starter["deck_path"]
        refs = deck_names(path)
        # The desired list names its commander in the header comment and the
        # manifest's `commander` field, not as a library line, and the manifest
        # counts it in the referenced set; include it so the echo check is exact.
        if starter["commander"] not in refs:
            refs = refs + [starter["commander"]]
        missing = sorted(n for n in refs if n not in supported_set)
        union.update(refs)
        declared = sorted(starter["missing_cards"])
        if missing != declared:
            only_fresh = sorted(set(missing) - set(declared))
            only_manifest = sorted(set(declared) - set(missing))
            problems.append(
                f"{starter['id']}: manifest missing_cards disagrees with a fresh "
                f"computation (fresh-only {only_fresh}, manifest-only {only_manifest})")
        if not missing:
            certified += 1
        rows.append({
            "id": starter["id"],
            "commander": starter["commander"],
            "status": starter["status"],
            "referenced": len(refs),
            "unique_referenced": len(set(refs)),
            "missing": len(missing),
            "missing_duplicates": len(refs) - len(set(refs)),
        })

    missing_union = sorted(n for n in union if n not in supported_set)
    report = {
        "catalog_supported": len(supported),
        "catalog_unique": len(set(supported)),
        "union_names": len(union),
        "union_supported": len(union) - len(missing_union),
        "union_missing": len(missing_union),
        "certified_lists": certified,
        "starters": rows,
        "per_list_missing": [r["missing"] for r in rows],
        "ayula_missing": next((r["missing"] for r in rows
                               if "ayula" in r["id"]), None),
        "manifest_agrees": not problems,
        "problems": problems,
    }
    print(json.dumps(report, indent=1))
    if problems:
        print("MANIFEST DISAGREEMENT", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
