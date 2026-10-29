#!/usr/bin/env python3
"""Slice 66 provenance fetch: Pashalik Mons (DMR 133) and Goblin Trashmaster
(M19 144).

Fetches the authoritative printing from Scryfall, then every printing of the
same oracle id, and records how many distinct oracle texts exist. The catalog
strings in crates/core/src/catalog.rs are written from this file, never from
memory. Network access is required; the result is committed so tests never need
it.
"""
import json
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "fixtures/starter-corpus/slice66-sourced.json"
UA = {"User-Agent": "aperture-issue17/1.0", "Accept": "application/json"}


def get(url: str):
    request = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def main() -> None:
    wanted = ["Pashalik Mons", "Goblin Trashmaster"]
    cards = []
    retrieved = None
    for name in wanted:
        exact = get("https://api.scryfall.com/cards/named?" + urllib.parse.urlencode({"exact": name}))
        retrieved = retrieved or exact["object"] and None
        prints = get(exact["prints_search_uri"] + "&unique=prints")
        texts = {p.get("oracle_text", "") for p in prints["data"]}
        assert len(texts) == 1, (name, len(texts))
        assert exact.get("oracle_text") in texts, name
        data = prints["data"]
        cards.append(
            {
                "name": exact["name"],
                "set": exact["set"],
                "collector_number": exact["collector_number"],
                "oracle_id": exact["oracle_id"],
                "mana_cost": exact.get("mana_cost", ""),
                "type_line": exact["type_line"],
                "power": exact.get("power"),
                "toughness": exact.get("toughness"),
                "oracle_text": exact["oracle_text"],
                "keywords": exact.get("keywords", []),
                "colors": exact.get("colors", []),
                "color_identity": exact.get("color_identity", []),
                "layout": exact["layout"],
                "released_at": exact["released_at"],
                "scryfall_id": exact["id"],
                "legalities_commander": exact["legalities"]["commander"],
                "mana_value": float(exact.get("cmc", 0)),
                "printings_checked": prints["total_cards"],
                "printings_distinct_oracle_texts": len(texts),
                "printings": sorted(
                    f"{p['set']}/{p['collector_number']}" for p in data
                ),
            }
        )
    payload = {
        "schema": "aperture.sourced-cards.v1",
        "provider": "Scryfall per-card API (cards/named with exact=, then the printing search by oracle id)",
        "retrieved_at": __import__("datetime").datetime.now(__import__("datetime").timezone.utc).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        ),
        "note": (
            "Oracle text is fetched, never typed. Every printing was enumerated and each card "
            "has exactly one distinct oracle text, so the implemented wording cannot be a "
            "reprint variant."
        ),
        "cards": cards,
    }
    OUT.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    for card in cards:
        print(
            f"{card['name']:20s} {card['set']}/{card['collector_number']:>4s} "
            f"{card['mana_cost']:10s} {card['power']}/{card['toughness']} "
            f"printings={card['printings_checked']} texts={card['printings_distinct_oracle_texts']}"
        )
    print("retrieved_at", payload["retrieved_at"])


if __name__ == "__main__":
    main()
