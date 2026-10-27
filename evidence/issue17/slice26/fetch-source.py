"""Fetch authoritative printed text for the Ayula-list gap from Scryfall.

Records the retrieval timestamp, the exact immutable printing id, and the raw
response for every card so later slices can re-verify provenance.
"""
import datetime
import json
import os
import urllib.parse
import urllib.request

NAMES = [
    "Beorn the Fierce",
    "Beorn's Hospitality",
    "Circle of the Moon Druid",
    "Drover Grizzly",
    "Druid's Familiar",
    "Dúnedain Rangers",
    "Evercoat Ursine",
    "Kamahl's Summons",
    "Lightning Greaves",
    "Lumra, Bellow of the Woods",
    "Owlbear Cub",
    "Rampaging Yao Guai",
    "Ruxa, Patient Professor",
    "Spider-Ham, Peter Porker",
    "Spirit of the Aldergard",
    "Stuffed Bear",
    "Swiftfoot Boots",
    "The Earth King",
    "Thijarian Witness",
    "Titania's Command",
    "Ursine Champion",
    "Ursine Monstrosity",
    "Words of Wilding",
]

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "evidence", "issue17", "slice26")
RAW = os.path.join(OUT, "source-raw")
os.makedirs(RAW, exist_ok=True)

stamp = datetime.datetime.now(datetime.timezone.utc).isoformat()
cards = {}
for name in NAMES:
    url = "https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(name)
    try:
        req = urllib.request.Request(
            url,
            headers={"User-Agent": "aperture-issue17/1.0", "Accept": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=30) as response:
            data = json.load(response)
    except Exception as exc:  # noqa: BLE001 - recorded verbatim for provenance
        cards[name] = {"error": str(exc), "url": url}
        continue
    cards[name] = {
        "url": url,
        "id": data.get("id"),
        "oracle_id": data.get("oracle_id"),
        "set": data.get("set"),
        "collector_number": data.get("collector_number"),
        "name": data.get("name"),
        "mana_cost": data.get("mana_cost"),
        "type_line": data.get("type_line"),
        "oracle_text": data.get("oracle_text"),
        "power": data.get("power"),
        "toughness": data.get("toughness"),
        "colors": data.get("colors"),
        "layout": data.get("layout"),
    }
    with open(
        os.path.join(RAW, f"{data.get('set')}-{data.get('collector_number')}.json"),
        "w",
        encoding="utf-8",
    ) as handle:
        json.dump(data, handle, indent=1, ensure_ascii=False)

payload = {
    "schema": 1,
    "kind": "issue17-slice26-source-retrieval",
    "retrieved_at": stamp,
    "source": "https://api.scryfall.com/cards/named?exact=<name>&format=json",
    "cards": cards,
}
with open(os.path.join(OUT, "source-retrieval.json"), "w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=1, ensure_ascii=False)

print("retrieved", stamp)
for name, card in cards.items():
    if "error" in card:
        print("ERR", name, card["error"])
        continue
    print(
        f"{name} | {card['set']} {card['collector_number']} | {card['mana_cost']} | "
        f"{card['type_line']} | {card['power']}/{card['toughness']} || {card['oracle_text']}"
    )
