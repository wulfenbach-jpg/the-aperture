#!/usr/bin/env python3
"""Source seven planned exact desired starter lists + analyzer snapshots.

These lists are DESIRED, not supported. The engine practice parser still refuses
every unsupported name; the snapshots are metadata-only analysis inputs.
"""

import hashlib
import json
import pathlib
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone

UA = "the-aperture-issue17/1.0 (planned starter corpus capture)"
ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT_DECKS = ROOT / "decks/starter-corpus/planned"
OUT_FIX = ROOT / "fixtures/starter-corpus/planned"

ARCHETYPES = [
    ("isamaru-white-aggro", "Isamaru, Hound of Konda", "W", {"Plains": 42}),
    ("talrand-draw-go", "Talrand, Sky Summoner", "U", {"Island": 42}),
    ("krenko-goblin-tokens", "Krenko, Mob Boss", "R", {"Mountain": 42}),
    ("ayula-bear-counters", "Ayula, Queen Among Bears", "G", {"Forest": 42}),
    ("elas-sacrifice", "Elas il-Kor, Sadistic Pilgrim", "WB", {"Plains": 21, "Swamp": 21}),
    ("tatyova-landfall", "Tatyova, Benthic Druid", "UG", {"Island": 21, "Forest": 21}),
    ("zada-spells", "Zada, Hedron Grinder", "R", {"Mountain": 42}),
]
BASICS = ["Plains", "Island", "Swamp", "Mountain", "Forest"]
QUOTAS = [(0, 2, 14), (3, 3, 12), (4, 4, 11), (5, 5, 10), (6, 99, 10)]
THEME = {
    "isamaru-white-aggro": "(t:human or t:soldier or t:warrior or o:equipment)",
    "talrand-draw-go": "(t:instant or t:sorcery)",
    "krenko-goblin-tokens": "(t:goblin or o:goblin)",
    "ayula-bear-counters": "(t:bear or o:bear)",
    "elas-sacrifice": "(o:sacrifice or o:dies)",
    "tatyova-landfall": '(o:landfall or o:"search your library for a land" or o:"play an additional land")',
    "zada-spells": '(t:instant or t:sorcery or o:"target creature")',
}
UNRESOLVED = {
    "isamaru-white-aggro": ["legend rule", "equipment/aura permanents", "enough supported white aggressive permanents"],
    "talrand-draw-go": ["token creation and token permanents", "spell-cast triggered abilities"],
    "krenko-goblin-tokens": ["token creation", "activated token-producing abilities", "goblin tribal payoffs", "haste keyword"],
    "ayula-bear-counters": ["+1/+1 counters as rules objects", "bear tribal triggered abilities", "token creation"],
    "elas-sacrifice": ["death triggers", "sacrifice outlets", "life-drain triggers"],
    "tatyova-landfall": ["landfall triggers", "extra land drops"],
    "zada-spells": ["spell copying for each other target", "token permanents"],
}


def get(url):
    for attempt in range(6):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/json"})
            with urllib.request.urlopen(req, timeout=60) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            if error.code != 429 or attempt == 5:
                raise
            time.sleep(float(error.headers.get("Retry-After", "2")) + attempt)
    raise SystemExit("unreachable")


def sha(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def special(card):
    text = (card.get("oracle_text") or "").lower()
    return any(
        phrase in text
        for phrase in (
            "can be your commander",
            "partner",
            "choose a background",
            "friends forever",
            "doctor's companion",
            "companion —",
        )
    )


def pick(cards, commander, priority_cards):
    pool = []
    seen = set()
    for card in list(priority_cards) + list(cards):
        if card.get("name") == commander or card["name"] in seen:
            continue
        if card.get("layout") != "normal" or "//" in card["name"]:
            continue
        if card["legalities"].get("commander") != "legal":
            continue
        if card["name"] in BASICS or "Land" in card["type_line"]:
            continue
        if special(card):
            continue
        seen.add(card["name"])
        pool.append(card)
    chosen = []
    used = set()
    for card in [c for c in pool if c in priority_cards][:20]:
        chosen.append(card)
        used.add(card["name"])
    for low, high, quota in QUOTAS:
        take = [c for c in pool if low <= c.get("cmc", 99) <= high and c["name"] not in used]
        for card in take[:quota]:
            chosen.append(card)
            used.add(card["name"])
    for card in pool:
        if len(chosen) >= 57:
            break
        if card["name"] not in used:
            chosen.append(card)
            used.add(card["name"])
    return chosen[:57]


def main():
    OUT_DECKS.mkdir(parents=True, exist_ok=True)
    OUT_FIX.mkdir(parents=True, exist_ok=True)
    retrieved = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    basics = {}
    for name in BASICS:
        basics[name] = get("https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(name))
        time.sleep(0.12)
    index = {"provider": "scryfall", "retrieved_at": retrieved, "archetypes": {}}
    manifest = json.loads((ROOT / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))
    catalog_names = set(re.findall(r'card!\(\s*\w+,\s*"([^"]+)"', (ROOT / "crates/core/src/catalog.rs").read_text(encoding="utf-8")))

    for starter in manifest["starters"]:
        if starter["id"] == "jasmine-vanilla-stompy":
            continue
        id_, commander_name, identity, land_split = next(
            row for row in ARCHETYPES if row[0] == starter["id"]
        )
        query = f"id<={identity.lower()} legal:commander -t:land -is:funny -is:digital"
        url = "https://api.scryfall.com/cards/search?q=" + urllib.parse.quote(query) + "&unique=cards&order=edhrec"
        commander = get("https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(commander_name))
        time.sleep(0.12)
        data = get(url)
        time.sleep(0.12)
        theme_query = f"id<={identity.lower()} legal:commander -t:land -is:funny -is:digital {THEME[id_]}"
        theme_url = "https://api.scryfall.com/cards/search?q=" + urllib.parse.quote(theme_query) + "&unique=cards&order=edhrec"
        theme = get(theme_url)
        time.sleep(0.12)
        chosen = pick(data["data"], commander_name, theme["data"])
        if len(chosen) != 57:
            raise SystemExit(f"{id_}: only {len(chosen)} candidates")
        names = [card["name"] for card in chosen]
        lines = [f"// {id_} — DESIRED, NOT SUPPORTED. 99 library cards; commander: {commander_name}", "// Metadata-only planned list; do not load as a supported/default deck."]
        lines += [f"1 {name}" for name in names]
        lines += ["", "// Basic lands"]
        lines += [f"{qty} {land}" for land, qty in land_split.items()]
        (OUT_DECKS / f"{id_}.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")

        library = [{"name": c["name"], "oracle_id": c["oracle_id"], "quantity": 1} for c in chosen]
        library += [
            {"name": land, "oracle_id": basics[land]["oracle_id"], "quantity": qty}
            for land, qty in land_split.items()
        ]
        raw_deck = json.dumps({
            "commanders": [{"name": commander["name"], "oracle_id": commander["oracle_id"], "quantity": 1}],
            "companions": [], "excluded": [], "library": library, "sideboard": [],
        }, separators=(",", ":"), sort_keys=True)
        (OUT_FIX / f"{id_}.deck.json").write_text(json.dumps({
            "schema": "aperture.commander-deck.v1",
            "provenance": {
                "kind": "constructed", "provider": "the-aperture planned starter corpus",
                "source": f"desired list for {id_}; cards from Scryfall search {url}",
                "captured_at": retrieved, "raw_sha256": sha(raw_deck),
                "note": "DESIRED, NOT SUPPORTED. raw_sha256 binds raw_deck.",
            },
            "raw_deck": raw_deck,
        }, indent=1) + "\n", encoding="utf-8")

        used = [commander] + chosen + [basics[land] for land in land_split]
        raw_cards = "\n".join(json.dumps(card, separators=(",", ":"), sort_keys=True) for card in used) + "\n"
        cards_sha = sha(raw_cards)
        (OUT_FIX / f"{id_}.cards.json").write_text(json.dumps({
            "schema": "aperture.card-metadata.v1",
            "provenance": {
                "kind": "real", "provider": "scryfall",
                "api_url": url.replace("https://api.scryfall.com/cards/search", "https://api.scryfall.com/cards/search"),
                "download_url": "https://data.scryfall.io/oracle-cards/oracle-cards-20261024210157.jsonl.gz",
                "bulk_type": "oracle_cards", "api_updated_at": "2026-10-24T21:01:57.691+00:00",
                "captured_at": retrieved, "cards_sha256": cards_sha, "raw_sha256": cards_sha,
                "selection": f"real Scryfall search response cards for {query}; one printing per card",
                "card_count": len(used),
                "note": "Metadata-only planned list. The engine support report must refuse these cards.",
            },
            "raw_cards_jsonl": raw_cards,
        }, indent=1) + "\n", encoding="utf-8")

        missing = sorted(
            {commander_name, *names, *[land for land in land_split if land not in catalog_names]}
            - catalog_names
        )
        starter["deck_path"] = f"decks/starter-corpus/planned/{id_}.txt"
        starter["missing_cards"] = missing
        starter["missing_interactions"] = list(UNRESOLVED[id_])
        if any(land not in catalog_names for land in land_split):
            starter["missing_interactions"].append(
                "black basic land (Swamp) is not in the supported pool"
            )
        index["archetypes"][id_] = {
            "query_url": url, "theme_query_url": theme_url,
            "commander_url": "https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(commander_name),
            "cards": [{"name": c["name"], "id": c["id"], "set": c["set"], "collector_number": c["collector_number"],
                        "scryfall_uri": c["scryfall_uri"], "oracle_id": c["oracle_id"]} for c in used],
        }
        print(f"{id_}: {len(names)} nonland, basics {sum(land_split.values())}, missing {len(missing)}")

    (OUT_FIX / "source-index.json").write_text(json.dumps(index, indent=1) + "\n", encoding="utf-8")
    (ROOT / "decks/starter-corpus/manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
