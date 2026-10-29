import json

path = "decks/starter-corpus/manifest.json"
with open(path, encoding="utf-8") as handle:
    manifest = json.load(handle)

starter = next(s for s in manifest["starters"] if s["id"] == "ayula-bear-counters")
starter["missing_cards"] = [c for c in starter["missing_cards"] if c != "Ruxa, Patient Professor"]
starter["missing_interactions"] = [
    i
    for i in starter["missing_interactions"]
    if "Ruxa, Patient Professor" not in i
]

# Whole-field discipline: the two fields must still agree card-for-card.
assert len(starter["missing_cards"]) == 4, starter["missing_cards"]
assert len(starter["missing_interactions"]) == 4, starter["missing_interactions"]
for card in starter["missing_cards"]:
    assert any(card in entry for entry in starter["missing_interactions"]), card
for entry in starter["missing_interactions"]:
    assert any(card in entry for card in starter["missing_cards"]), entry

with open(path, "w", encoding="utf-8", newline="\n") as handle:
    json.dump(manifest, handle, indent=1, ensure_ascii=False)
    handle.write("\n")
print("ayula missing_cards:", starter["missing_cards"])
print("ayula missing_interactions:", starter["missing_interactions"])
