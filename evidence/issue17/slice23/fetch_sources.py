"""Fetch authoritative Scryfall records for the remaining Ayula-list names."""
import json
import pathlib
import urllib.request

NAMES = [
    "Beorn the Fierce", "Beorn's Hospitality", "Circle of the Moon Druid",
    "Drover Grizzly", "Druid's Familiar", "Dúnedain Rangers", "Evercoat Ursine",
    "Exuberant Wolfbear", "Goreclaw, Terror of Qal Sisma", "Kamahl's Summons",
    "Lightning Greaves", "Lumra, Bellow of the Woods", "Owlbear Cub",
    "Rampaging Yao Guai", "Ruxa, Patient Professor", "Spider-Ham, Peter Porker",
    "Spirit of the Aldergard", "Stuffed Bear", "Surrak and Goreclaw",
    "Swiftfoot Boots", "The Earth King", "Thijarian Witness",
    "Titania's Command", "Ursine Champion", "Ursine Monstrosity",
    "Words of Wilding",
]

root = pathlib.Path("C:/AI/aperture-worktrees/issue-17")
out_dir = root / "evidence/issue17/slice23/raw"
out_dir.mkdir(parents=True, exist_ok=True)

body = json.dumps({"identifiers": [{"name": n} for n in NAMES]}).encode()
req = urllib.request.Request(
    "https://api.scryfall.com/cards/collection",
    data=body,
    headers={
        "Content-Type": "application/json",
        "User-Agent": "aperture-issue17/1.0",
        "Accept": "application/json",
    },
)
data = json.loads(urllib.request.urlopen(req, timeout=60).read())
(out_dir / "cards-collection.json").write_text(json.dumps(data, indent=1), encoding="utf-8")
print("not_found:", [x.get("name") for x in data.get("not_found", [])])
for card in data["data"]:
    print(
        "==", card["name"], "|", card["set"].upper(), card["collector_number"],
        "|", card.get("mana_cost"), "|", card["type_line"], "|",
        card.get("power"), card.get("toughness"), "|", card.get("keywords"),
    )
    print(card.get("oracle_text", "").replace("\n", " // "))
