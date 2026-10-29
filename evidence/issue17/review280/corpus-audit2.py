"""Independent each-list corpus audit for issue 17 (review run 280).

Source of truth: the eight desired starter deck files plus each list's declared
commander, checked against the implemented pool parsed out of catalog.rs (every
card!/card_*! macro invocation before the TOKENS table).

Fails if any deck file is not a 99-card library, if the parsed pool disagrees
with the declared CARDS length, or if a computed missing set disagrees with
decks/starter-corpus/manifest.json.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MANIFEST = ROOT / "decks" / "starter-corpus" / "manifest.json"
CATALOG = ROOT / "crates" / "core" / "src" / "catalog.rs"
BASIC_LANDS = {
    "Plains", "Island", "Swamp", "Mountain", "Forest", "Wastes",
    "Snow-Covered Plains", "Snow-Covered Island", "Snow-Covered Swamp",
    "Snow-Covered Mountain", "Snow-Covered Forest",
}


def deck_entries(path: Path):
    entries = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("//"):
            continue
        qty, name = line.split(" ", 1)
        entries.append((int(qty), name.strip()))
    return entries


def pool_names(path: Path):
    text = path.read_text(encoding="utf-8")
    cards_block = text.partition("pub static TOKENS")[0]
    names = set(
        re.findall(r'card(?:_[a-z0-9]+)?!\(\s*[A-Za-z0-9_]+,\s*"([^"]+)"', cards_block, re.S)
    )
    declared = re.search(r"pub static CARDS: \[CardDef; (\d+)\]", text)
    return names, int(declared.group(1)) if declared else None


def main():
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    pool, declared_len = pool_names(CATALOG)
    problems = []
    if declared_len is not None and declared_len != len(pool):
        problems.append(f"pool parsed {len(pool)} names but CARDS declares {declared_len}")

    union = set()
    rows = []
    for starter in manifest["starters"]:
        entries = deck_entries(ROOT / starter["deck_path"])
        total = sum(qty for qty, _ in entries)
        if total != 99:
            problems.append(f"{starter['id']}: deck file sums to {total}, expected 99")
        required = {name for _, name in entries} | {starter["commander"]}
        missing = required - pool
        declared_missing = set(starter.get("missing_cards", []))
        if missing != declared_missing:
            problems.append(
                f"{starter['id']}: computed missing differs from manifest "
                f"(only-manifest={sorted(declared_missing - missing)}, "
                f"only-computed={sorted(missing - declared_missing)})"
            )
        nonland = {n for n in required if n not in BASIC_LANDS}
        union |= nonland
        rows.append(
            {
                "id": starter["id"],
                "status": starter["status"],
                "library_cards": total,
                "nonland_names": len(nonland),
                "supported_here": len(nonland & pool),
                "missing_here": len(nonland - pool),
            }
        )

    result = {
        "catalog": len(pool),
        "union": len(union),
        "supported": len(union & pool),
        "missing": len(union - pool),
        "certified_lists": sum(1 for s in manifest["starters"] if s["status"] == "certified"),
        "desired_lists": len(manifest["starters"]),
        "lists": rows,
        "per_list_missing": [row["missing_here"] for row in rows],
        "problems": problems,
    }
    print(json.dumps(result, indent=2))
    (Path(__file__).parent / "corpus.json").write_text(json.dumps(result, indent=2))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
