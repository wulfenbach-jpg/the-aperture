"""Fetch authoritative printed text for the still-missing Ayula starter cards.

Reads the missing names straight out of the slice-11 native audit (never a
hand-typed list), asks Scryfall's collection endpoint for the default printing,
and records the exact printed fields plus the retrieval instant so every later
claim can be traced back to a source and a date.
"""

from pathlib import Path
import json
import urllib.request
import datetime

audit = json.loads(Path("evidence/issue17/slice11/corpus-audit.json").read_text(encoding="utf-8"))
names = audit["per_list"]["ayula-bear-counters"]["missing_names"]

body = json.dumps({"identifiers": [{"name": name} for name in names]}).encode()
request = urllib.request.Request(
    "https://api.scryfall.com/cards/collection",
    data=body,
    headers={
        "Content-Type": "application/json",
        "User-Agent": "the-aperture-corpus/1.0",
        "Accept": "application/json",
    },
)
retrieved_at = datetime.datetime.now(datetime.timezone.utc).isoformat()
with urllib.request.urlopen(request, timeout=60) as response:
    payload = json.loads(response.read())

cards = []
for card in payload.get("data", []):
    cards.append(
        {
            "name": card["name"],
            "mana_cost": card.get("mana_cost", ""),
            "mana_value": card.get("cmc"),
            "type_line": card.get("type_line"),
            "oracle_text": card.get("oracle_text", ""),
            "power": card.get("power"),
            "toughness": card.get("toughness"),
            "keywords": card.get("keywords", []),
            "set": card.get("set"),
            "collector_number": card.get("collector_number"),
            "prints_search_uri": card.get("prints_search_uri"),
            "scryfall_uri": card.get("scryfall_uri"),
        }
    )

out = {
    "source": "https://api.scryfall.com/cards/collection",
    "retrieved_at_utc": retrieved_at,
    "requested": len(names),
    "returned": len(cards),
    "not_found": payload.get("not_found", []),
    "cards": cards,
}
Path("evidence/issue17/slice11/ayula-missing-printings.json").write_text(
    json.dumps(out, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
)
print("requested", len(names), "returned", len(cards), "not_found", len(payload.get("not_found", [])))
for card in cards:
    text = (card["oracle_text"] or "").replace("\n", " / ")
    print(f"{card['name']:26s} {card['mana_cost']:12s} {card['power']}/{card['toughness']}  {card['type_line']}  :: {text[:90]}")
