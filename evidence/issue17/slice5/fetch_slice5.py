"""Slice 5 provenance: read each new card from live Scryfall.

Named+exact gives the current default printing (recorded with set/collector
number and release date); `unique=prints` proves every printing of the card
carries the same Oracle text, so the catalog string is not printing-specific.
Retrieval timestamp is recorded per card, as issue17 requires.
"""

import datetime
import json
import pathlib
import urllib.parse
import urllib.request

names = ["Stitcher's Supplier", "Ruin Crab", "Negate", "Preordain",
         "Pyretic Ritual", "Seething Song"]
roles = {
    "Stitcher's Supplier": "mill_on_enters_or_dies",
    "Ruin Crab": "landfall_mill_each_opponent",
    "Negate": "counter_target_noncreature_spell",
    "Preordain": "scry_two_then_draw",
    "Pyretic Ritual": "add_three_red_mana",
    "Seething Song": "add_five_red_mana",
}
stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
HEADERS = {"User-Agent": "aperture-corpus/1.0", "Accept": "application/json"}


def get(url):
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=60) as response:
        return json.loads(response.read())


out = []
for name in names:
    card = get("https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(name))
    prints = get("https://api.scryfall.com/cards/search?unique=prints&q="
                 + urllib.parse.quote('!"%s"' % name))
    texts = sorted({p.get("oracle_text", "") for p in prints["data"]})
    out.append({
        "name": card["name"],
        "role": roles[name],
        "oracle_id": card["oracle_id"],
        "scryfall_id": card["id"],
        "set": card["set"],
        "collector_number": card["collector_number"],
        "set_name": card["set_name"],
        "released_at": card["released_at"],
        "layout": card["layout"],
        "mana_cost": card.get("mana_cost", ""),
        "type_line": card["type_line"],
        "power": card.get("power"),
        "toughness": card.get("toughness"),
        "oracle_text": card.get("oracle_text", ""),
        "color_identity": card.get("color_identity", []),
        "keywords": card.get("keywords", []),
        "legalities_commander": card["legalities"]["commander"],
        "printing_url": "https://api.scryfall.com/cards/%s/%s" % (
            card["set"], card["collector_number"]),
        "printings_checked": len(prints["data"]),
        "distinct_printing_texts": len(texts),
        # The full set of distinct printed texts, so a wording change between
        # printings is visible instead of being averaged away. Seething Song has
        # two: the pre-2014 "Add ... to your mana pool." wording and today's.
        "distinct_printing_texts_list": texts,
        "retrieved_at": stamp,
    })
    print(name, card["set"], card["collector_number"], "printings",
          len(prints["data"]), "distinct texts", len(texts),
          repr(card.get("oracle_text")))

doc = {
    "schema": "aperture.starter-sourced-cards.v1",
    "provider": "scryfall",
    "retrieved_at": stamp,
    "note": ("Six records read from live Scryfall in this run (named+exact and "
             "unique=prints); every printing of each card carries the same "
             "Oracle text."),
    "cards": out,
}
root = pathlib.Path(__file__).resolve().parents[3]
(root / "fixtures/starter-corpus/slice5-sourced.json").write_text(
    json.dumps(doc, indent=1) + "\n")
(pathlib.Path(__file__).resolve().parent / "slice5-sourced-raw.json").write_text(
    json.dumps(doc, indent=1) + "\n")
print("wrote", root / "fixtures/starter-corpus/slice5-sourced.json")
