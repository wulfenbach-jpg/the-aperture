"""Independent Scryfall provenance check for slice 5 (review only).

Fetches the six printed cards from live Scryfall and compares oracle text,
mana cost, type line and power/toughness with the catalog strings as compiled
into the crate. Writes scryfall-live.json + prints a table.

Run: python evidence/issue17/slice5/review/scryfall_check.py
"""
import json
import pathlib
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent

PRINTINGS = {
    "Stitcher's Supplier": ("tdc", "196"),
    "Ruin Crab": ("znr", "75"),
    "Negate": ("tmt", "47"),
    "Preordain": ("tdc", "161"),
    "Pyretic Ritual": ("m11", "153"),
    "Seething Song": ("c21", "179"),
}

# Catalog strings exactly as appear in crates/core/src/catalog.rs (candidate eb0a6f5)
CATALOG = {
    "Stitcher's Supplier": dict(
        cost="{B}",
        type_line="Creature \u2014 Zombie",
        power="1",
        toughness="1",
        oracle=(
            "When this creature enters or dies, mill three cards. "
            "(Put the top three cards of your library into your graveyard.)"
        ),
    ),
    "Ruin Crab": dict(
        cost="{U}",
        type_line="Creature \u2014 Crab",
        power="0",
        toughness="3",
        oracle=(
            "Landfall \u2014 Whenever a land you control enters, each opponent mills "
            "three cards. (To mill a card, a player puts the top card of their "
            "library into their graveyard.)"
        ),
    ),
    "Negate": dict(
        cost="{1}{U}",
        type_line="Instant",
        power=None,
        toughness=None,
        oracle="Counter target noncreature spell.",
    ),
    "Preordain": dict(
        cost="{U}",
        type_line="Sorcery",
        power=None,
        toughness=None,
        oracle=(
            "Scry 2, then draw a card. (To scry 2, look at the top two cards of "
            "your library, then put any number of them on the bottom and the rest "
            "on top in any order.)"
        ),
    ),
    "Pyretic Ritual": dict(
        cost="{1}{R}",
        type_line="Instant",
        power=None,
        toughness=None,
        oracle="Add {R}{R}{R}.",
    ),
    "Seething Song": dict(
        cost="{2}{R}",
        type_line="Instant",
        power=None,
        toughness=None,
        oracle="Add {R}{R}{R}{R}{R}.",
    ),
}


def fetch(set_code, number):
    url = f"https://api.scryfall.com/cards/{set_code}/{number}"
    req = urllib.request.Request(
        url, headers={"User-Agent": "aperture-slice5-review/1.0", "Accept": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.load(resp)


def main():
    live = {}
    rows = []
    failures = 0
    for name, (set_code, number) in PRINTINGS.items():
        card = fetch(set_code, number)
        live[name] = card
        cat = CATALOG[name]
        checks = {
            "name": card["name"] == name,
            "cost": card.get("mana_cost") == cat["cost"],
            "type_line": card.get("type_line") == cat["type_line"],
            "power": card.get("power") == cat["power"],
            "toughness": card.get("toughness") == cat["toughness"],
            "oracle": (card.get("oracle_text") or "") == cat["oracle"],
        }
        ok = all(checks.values())
        failures += 0 if ok else 1
        rows.append((name, set_code, number, ok, checks, card))
        print(f"{name} [{set_code} {number}] {'MATCH' if ok else 'MISMATCH'}")
        for key, matched in checks.items():
            print(f"   {key}: {'ok' if matched else 'DIFFERS'}")
        if not ok:
            print(f"   live oracle: {card.get('oracle_text')!r}")
            print(f"   cat  oracle: {cat['oracle']!r}")
            print(f"   live cost  : {card.get('mana_cost')!r} cat: {cat['cost']!r}")
            print(f"   live type  : {card.get('type_line')!r} cat: {cat['type_line']!r}")
            print(
                f"   live P/T   : {card.get('power')!r}/{card.get('toughness')!r} "
                f"cat: {cat['power']!r}/{cat['toughness']!r}"
            )

    slim = {
        name: {
            "scryfall_uri": card.get("scryfall_uri"),
            "set": card.get("set"),
            "set_name": card.get("set_name"),
            "collector_number": card.get("collector_number"),
            "mana_cost": card.get("mana_cost"),
            "type_line": card.get("type_line"),
            "oracle_text": card.get("oracle_text"),
            "power": card.get("power"),
            "toughness": card.get("toughness"),
            "rarity": card.get("rarity"),
            "released_at": card.get("released_at"),
            "retrieved_at_utc": __import__("datetime").datetime.now(
                __import__("datetime").timezone.utc
            ).isoformat(),
        }
        for name, card in live.items()
    }
    (HERE / "scryfall-live.json").write_text(json.dumps(slim, indent=2), encoding="utf-8")
    print(f"\n{len(PRINTINGS) - failures}/{len(PRINTINGS)} cards match the catalog strings")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
