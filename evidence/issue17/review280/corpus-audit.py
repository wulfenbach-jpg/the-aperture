"""Independent each-list corpus audit for issue 17 (review run 280).

Reads the eight desired starter deck files and the catalog source of truth,
computes the union of desired nonland names, which of them the pool supports,
and compares the per-list missing sets with decks/starter-corpus/manifest.json.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MANIFEST = ROOT / "decks" / "starter-corpus" / "manifest.json"
CATALOG = ROOT / "crates" / "core" / "src" / "catalog.rs"

BASIC_LANDS = {
    "Plains", "Island", "Swamp", "Mountain", "Forest",
    "Wastes", "Snow-Covered Plains", "Snow-Covered Island",
    "Snow-Covered Swamp", "Snow-Covered Mountain", "Snow-Covered Forest",
}


def deck_names(path: Path):
    names = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("//"):
            continue
        m = re.match(r"^(\d+)\s+(.+)$", line)
        if not m:
            raise SystemExit(f"unparsed deck line in {path}: {raw!r}")
        names.append(m.group(2).strip())
    return names


def catalog_names(path: Path):
    text = path.read_text(encoding="utf-8")
    names = re.findall(r'card!\(\s*\w+\s*,\s*"([^"]+)"', text)
    if not names:
        raise SystemExit("no card names parsed from catalog.rs")
    declared = re.search(r"pub static CARDS: \[CardDef; (\d+)\]", text)
    return names, int(declared.group(1)) if declared else None


def main():
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    supported_list, declared = catalog_names(CATALOG)
    supported = set(supported_list)

    union = set()
    per_list = []
    problems = []
    for starter in manifest["starters"]:
        path = ROOT / starter["deck_path"]
        names = deck_names(path)
        nonland = [n for n in names if n not in BASIC_LANDS]
        union.update(nonland)
        computed_missing = sorted({n for n in nonland if n not in supported})
        declared_missing = sorted(starter.get("missing_cards", []))
        if computed_missing != declared_missing:
            problems.append(
                f"{starter['id']}: manifest missing_cards differ from computed "
                f"(only-in-manifest={sorted(set(declared_missing) - set(computed_missing))}, "
                f"only-in-deck={sorted(set(computed_missing) - set(declared_missing))})"
            )
        per_list.append(
            {
                "id": starter["id"],
                "status": starter["status"],
                "cards": len(names),
                "nonland": len(nonland),
                "missing": len(computed_missing),
            }
        )

    missing_union = sorted(n for n in union if n not in supported)
    result = {
        "catalog_declared_len": declared,
        "catalog_names_parsed": len(supported_list),
        "catalog_names_unique": len(supported),
        "lists": per_list,
        "union_names": len(union),
        "supported_union_names": len(union) - len(missing_union),
        "missing_union_names": len(missing_union),
        "certified_lists": sum(1 for s in manifest["starters"] if s["status"] == "certified"),
        "per_list_missing": [entry["missing"] for entry in per_list],
        "problems": problems,
        "ayula_missing": missing_union and None,
    }
    result["ayula_missing"] = next(
        entry["missing"] for entry in per_list if entry["id"] == "ayula-bear-counters"
    )
    print(json.dumps(result, indent=2))
    if problems:
        print("MANIFEST MISMATCH", file=sys.stderr)
        return 1
    if declared is not None and declared != len(supported_list):
        print("CATALOG LEN MISMATCH", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
