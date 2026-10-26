"""Slice 6 provenance: read each new card from live Scryfall.

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

names = ["Elas il-Kor, Sadistic Pilgrim", "Zulaport Cutthroat", "Blood Artist",
         "Hedron Crab"]
roles = {
    "Elas il-Kor, Sadistic Pilgrim": "another_creature_you_control_dies_drain",
    "Zulaport Cutthroat": "creature_you_control_dies_drain",
    "Blood Artist": "any_creature_dies_drain_target_player",
    "Hedron Crab": "landfall_target_player_mill_three",
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
        "printings_checked": prints.get("total_cards", len(prints["data"])),
        "distinct_printed_texts": texts,
        "retrieved_at": stamp,
    })

path = pathlib.Path(__file__).with_name("slice6-sourced-raw.json")
path.write_text(json.dumps({
    "source": "https://api.scryfall.com (named?exact + search?unique=prints)",
    "retrieved_at": stamp,
    "cards": out,
}, indent=1) + "\n", encoding="utf-8")
print(path)
for card in out:
    print(card["name"], "|", card["set"], card["collector_number"], "|",
          card["mana_cost"], "|", card["type_line"], "|",
          card["power"], "/", card["toughness"], "|", card["printings_checked"],
          "printings,", len(card["distinct_printed_texts"]), "distinct text(s)")
    print("   ", card["oracle_text"].replace("\n", " / "))
