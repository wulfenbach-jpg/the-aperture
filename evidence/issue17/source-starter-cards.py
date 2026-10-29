#!/usr/bin/env python3
"""Capture real Scryfall printing data for the issue-17 starter package.

Usage: python evidence/issue17/source-starter-cards.py

For every card name below this script performs one `GET /cards/named?exact=...`
lookup and then one `GET /cards/{set}/{collector_number}` lookup for the concrete
printing that the named lookup returned. Requests are paced at 120 ms. The raw
responses, request URLs and retrieval timestamp are written to
evidence/issue17/scryfall-responses.json; the compact catalog declaration is
written to fixtures/starter-corpus/sourced-cards.json.

No card data is invented here: every field in the compact declaration is copied
from the Scryfall printing response. Re-running the script re-fetches and
replaces both files (the deck and card snapshots are generated separately by
generate-starter-fixtures.py so their embedded digests stay consistent).
"""

import json
import pathlib
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone

UA = "the-aperture-issue17/1.0 (offline supported-corpus fixture capture)"
ROOT = pathlib.Path(__file__).resolve().parents[2]

NEW_VANILLA = [
    # white
    "Savannah Lions",
    "Elite Vanguard",
    "Devoted Hero",
    "Eager Cadet",
    "Staunch Shieldmate",
    "Valiant Guard",
    "Yoked Ox",
    "Silvercoat Lion",
    "Glory Seeker",
    "Knight Errant",
    "Oreskos Swiftclaw",
    "Blade of the Sixth Pride",
    "Raptor Companion",
    "Armored Warhorse",
    "Alaborn Trooper",
    "Bastion Enforcer",
    "Loxodon Line Breaker",
    "Pearled Unicorn",
    "Pillarfield Ox",
    "Siege Mastodon",
    "Silent Artisan",
    "Indomitable Ancients",
    "Looming Altisaur",
    # green
    "Norwood Ranger",
    "Willow Elf",
    "Trained Jackal",
    "Balduvian Bears",
    "Bear Cub",
    "Cylian Elf",
    "Kalonian Tusker",
    "Swordwise Centaur",
    "Terrain Elemental",
    "Alpine Grizzly",
    "Nessian Courser",
    "Trained Armodon",
    "Golden Bear",
    "Axebane Beast",
    "Hollowhenge Beast",
    "Durkwood Boars",
    "Feral Krushok",
    "Blanchwood Treefolk",
    "Alpha Tyrranax",
    "Vastwood Gorger",
    "Enormous Baloth",
    # green/white
    "Watchwolf",
    "Armored Wolf-Rider",
    "Grizzled Leotau",
]

EXISTING_NONLAND = [
    "Grizzly Bears",
    "Runeclaw Bear",
    "Centaur Courser",
    "Craw Wurm",
    "Giant Growth",
    "Fog",
    "Swords to Plowshares",
    "Mending Hands",
    "Elvish Visionary",
    "Rootwalla",
    "Brindle Boar",
]

BASICS = ["Plains", "Forest"]
COMMANDER = "Jasmine Boreal"


def get_json(url):
    request = urllib.request.Request(
        url, headers={"User-Agent": UA, "Accept": "application/json"}
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def main():
    names = [COMMANDER] + NEW_VANILLA + EXISTING_NONLAND + BASICS
    assert len(names) == len(set(names)), "card names must be unique"
    retrieved_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    roles = {}
    for name in NEW_VANILLA:
        roles[name] = "vanilla"
    for name in EXISTING_NONLAND:
        roles[name] = "existing"
    for name in BASICS:
        roles[name] = "basic_land"
    roles[COMMANDER] = "commander"

    records = []
    responses = {"retrieved_at": retrieved_at, "cards": {}}
    for index, name in enumerate(names):
        named_url = "https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(name)
        named = get_json(named_url)
        time.sleep(0.12)
        set_code = named["set"]
        number = named["collector_number"]
        printing_url = f"https://api.scryfall.com/cards/{set_code}/{urllib.parse.quote(number)}"
        printing = get_json(printing_url)
        time.sleep(0.12)
        responses["cards"][name] = {
            "named_url": named_url,
            "printing_url": printing_url,
            "named": named,
            "printing": printing,
        }
        for card in (named, printing):
            if card["name"] != name:
                raise SystemExit(f"{name}: provider returned {card['name']!r}")
            if card["layout"] != "normal":
                raise SystemExit(f"{name}: layout {card['layout']!r} is not normal")
        records.append(
            {
                "name": printing["name"],
                "role": roles[name],
                "oracle_id": printing["oracle_id"],
                "scryfall_id": printing["id"],
                "set": set_code,
                "collector_number": number,
                "layout": printing["layout"],
                "released_at": printing.get("released_at"),
                "mana_cost": printing.get("mana_cost", ""),
                "type_line": printing["type_line"],
                "power": printing.get("power"),
                "toughness": printing.get("toughness"),
                "oracle_text": printing.get("oracle_text", ""),
                "color_identity": printing["color_identity"],
                "keywords": printing["keywords"],
                "legalities_commander": printing["legalities"]["commander"],
                "named_url": named_url,
                "printing_url": printing_url,
                "retrieved_at": retrieved_at,
            }
        )
        print(f"[{index + 1:02d}/{len(names)}] {name}: {set_code}/{number}")

    records.sort(key=lambda record: record["name"].lower())
    evidence = ROOT / "evidence/issue17/scryfall-responses.json"
    evidence.write_text(
        json.dumps(responses, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    fixture = ROOT / "fixtures/starter-corpus/sourced-cards.json"
    fixture.parent.mkdir(parents=True, exist_ok=True)
    fixture.write_text(
        json.dumps(
            {
                "schema": "aperture.starter-sourced-cards.v1",
                "provider": "scryfall",
                "retrieved_at": retrieved_at,
                "note": "Every card field except 'role' is copied from the recorded printing response in evidence/issue17/scryfall-responses.json. 'role' is the starter-package classification. Names, costs, type lines and Oracle text are Wizards of the Coast content.",
                "cards": records,
            },
            indent=1,
            ensure_ascii=False,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"wrote {evidence} and {fixture}")


if __name__ == "__main__":
    main()
