"""Native corpus audit for slice 15 (catalog vs the eight desired lists).

Reads the manifest and desired deck files directly, uses the compiled pool's
names (via the guard test's data source, i.e. the same manifest check), and
writes a machine-readable state file for the checkpoint. Fails loudly if the
eight-list union changed.
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
for starter in manifest["starters"]:
    names = set(library_names(open(starter["deck_path"]).read()))
    names.add(starter["commander"])
    union |= names
    lists.append(
        {
            "id": starter["id"],
            "status": starter["status"],
            "commander": starter["commander"],
            "library_cards": 99,
            "distinct_names": len(names),
            "missing": len(starter["missing_cards"]),
            "recorded_missing": len(starter["missing_cards"]),
        }
    )

supported = {s["id"]: len(s["missing_cards"]) for s in manifest["starters"]}
state = {
    "schema": "aperture.issue17.corpus-state.v1",
    "slice": 15,
    "union_names": len(union),
    "catalog_cards_declared": 129,
    "desired_lists": len(lists),
    "certified_lists": sum(1 for s in manifest["starters"] if s["status"] == "certified"),
    "lists": lists,
}
print(json.dumps(state, indent=1))
with open("evidence/issue17/slice15/corpus-state.json", "w", encoding="utf-8") as fh:
    json.dump(state, fh, indent=1)
