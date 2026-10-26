"""Publish the slice-6 provenance fixture from the raw Scryfall capture."""

import json
import pathlib

here = pathlib.Path(__file__).resolve().parent
root = here.parents[2]
raw = json.loads((here / "slice6-sourced-raw.json").read_text(encoding="utf-8"))
# Elas il-Kor is captured in the raw evidence but NOT added to the catalog this
# slice: it prints Deathtouch, which the rules layer does not model, so the card
# stays refused at import and must not appear in the sourcing fixture that the
# pool-consistency test counts.
implemented = {"Blood Artist", "Zulaport Cutthroat", "Hedron Crab"}
cards = [card for card in raw["cards"] if card["name"] in implemented]
assert len(cards) == 3, [card["name"] for card in cards]
out = {
    "schema": "aperture.starter-sourced-cards.v1",
    "provider": "scryfall",
    "retrieved_at": raw["retrieved_at"],
    "source": raw["source"],
    "note": (
        "Three records read from live Scryfall in this run (named+exact and "
        "unique=prints); every printing of each card carries the same Oracle "
        "text, so the catalog string is not printing-specific. Elas il-Kor was "
        "also captured but is not implemented (its printed Deathtouch keyword "
        "is unmodelled), so it is recorded only in slice6-sourced-raw.json."
    ),
    "cards": cards,
}
path = root / "fixtures/starter-corpus/slice6-sourced.json"
path.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print("wrote", path, len(out["cards"]), "cards")
