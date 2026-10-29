"""Native corpus audit for slice 16 (catalog vs the eight desired lists).

Reads the manifest and the desired deck files directly and writes a
machine-readable state file for the checkpoint. Fails loudly if a desired list
is not 99 library cards or if the union changed.
"""

import json

ROOT = "decks/starter-corpus"
manifest = json.load(open(f"{ROOT}/manifest.json"))


def library_names(text):
    names = []
    total = 0
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("//") or line.startswith("#"):
            continue
        count, name = line.split(" ", 1)
        total += int(count)
        names.append(name.strip())
    assert total == 99, f"desired list is {total} cards, not 99"
    return names


lists = []
union = set()
missing_names = set()
for starter in manifest["starters"]:
    names = set(library_names(open(starter["deck_path"]).read()))
    names.add(starter["commander"])
    union |= names
    missing = set(starter["missing_cards"])
    assert missing <= names, f"{starter['id']} lists a missing name not in its list"
    missing_names |= missing
    lists.append(
        {
            "id": starter["id"],
            "status": starter["status"],
            "commander": starter["commander"],
            "library_cards": 99,
            "distinct_names": len(names),
            "missing": len(missing),
        }
    )

assert len(union) == 459, f"union changed: {len(union)}"
assert len(missing_names) == 340, f"distinct missing changed: {len(missing_names)}"
missing = len(missing_names)
state = {
    "schema": "aperture.issue17.corpus-state.v1",
    "slice": 16,
    "union_names": len(union),
    "catalog_cards_declared": 131,
    "supported_union_names": len(union) - missing,
    "missing_union_names": missing,
    "desired_lists": len(lists),
    "certified_lists": sum(1 for s in manifest["starters"] if s["status"] == "certified"),
    "lists": lists,
}
print(json.dumps(state, indent=1))
with open("evidence/issue17/slice16/corpus-state.json", "w", encoding="utf-8") as fh:
    json.dump(state, fh, indent=1)
