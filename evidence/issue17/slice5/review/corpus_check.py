"""Independent corpus-number check for slice 5 (review only).

Recomputes the union/supported/missing numbers the README and
docs/issue17-starter-corpus.md state, directly from the eight desired deck
lists plus the compiled catalog, and checks the manifest's per-list
`missing_cards` and each new card's claimed home list.
"""
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[4]
MANIFEST = ROOT / "decks/starter-corpus/manifest.json"
CATALOG = ROOT / "crates/core/src/catalog.rs"

NEW_CARDS = {
    "Stitcher's Supplier": "elas-sacrifice",
    "Ruin Crab": "tatyova-landfall",
    "Negate": "talrand-draw-go",
    "Preordain": "talrand-draw-go",
    "Pyretic Ritual": "zada-spells",
    "Seething Song": "zada-spells",
}
BASICS = {"Island", "Mountain", "Forest", "Plains", "Swamp"}


def catalog_names():
    text = CATALOG.read_text(encoding="utf-8")
    return re.findall(
        r'(?:card|card_flash|card_trample|card_flying|card_defender)!\(\s*\w+,\s*"([^"]+)"',
        text,
    )


def deck_names(path):
    names = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or line.startswith("//"):
            continue
        m = re.match(r"^(\d+)\s+(.+)$", line)
        if m:
            names.append(m.group(2).strip())
    return names


def main():
    pool = set(catalog_names())
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    union = set()
    failures = []
    per_list = {}
    for starter in manifest["starters"]:
        names = deck_names(ROOT / starter["deck_path"])
        if starter["commander"] not in names:
            names = names + [starter["commander"]]
        union |= set(names)
        missing = sorted(n for n in set(names) if n not in pool)
        per_list[starter["id"]] = (names, missing)
        if missing != sorted(starter["missing_cards"]):
            failures.append(
                f"{starter['id']}: recomputed missing ({len(missing)}) != manifest "
                f"missing_cards ({len(starter['missing_cards'])})"
            )
        print(
            f"{starter['id']:<26} cards={len(names):<4} "
            f"supported={len(set(names)) - len(missing):<3} missing={len(missing)}"
        )

    supported = sorted(n for n in union if n in pool)
    missing = sorted(n for n in union if n not in pool)
    print()
    print(f"union names      : {len(union)}  (docs claim 459)")
    print(f"catalog size     : {len(pool)}  (docs claim 103)")
    print(f"supported in union: {len(supported)}  (docs claim 91)")
    print(f"missing in union : {len(missing)}  (docs claim 368)")

    for card, slug in NEW_CARDS.items():
        holders = [s for s, (names, _) in per_list.items() if card in names]
        ok = slug in holders
        print(f"home list {card:<20} -> {holders} (claimed {slug}) {'ok' if ok else 'MISSING'}")
        if not ok:
            failures.append(f"{card} is not in its claimed list {slug}")

    elas_names, elas_missing = per_list["elas-sacrifice"]
    elas_nonland = [n for n in elas_names if n not in BASICS]
    elas_unsupported_nonland = sorted({n for n in elas_nonland if n not in pool})
    print(
        f"\nelas unsupported nonland names: {len(elas_unsupported_nonland)}  "
        f"(docs claim 57)"
    )

    print("\nFAILURES:" if failures else "\nno failures")
    for f in failures:
        print(" -", f)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
