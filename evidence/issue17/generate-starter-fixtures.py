#!/usr/bin/env python3
"""Generate the offline analyzer snapshots for the issue-17 starter package.

Inputs (all real, already captured by source-starter-cards.py):
  evidence/issue17/scryfall-responses.json
  fixtures/starter-corpus/sourced-cards.json
  decks/starter-corpus/jasmine-vanilla-stompy.txt

Outputs:
  fixtures/starter-corpus/jasmine-vanilla-stompy.deck.json
  fixtures/starter-corpus/jasmine-vanilla-stompy.cards.json
  fixtures/starter-corpus/manifest.json

The snapshots use the existing aperture.commander-deck.v1 /
aperture.card-metadata.v1 schemas. This script only binds bytes; it is not a
legality engine. Certification runs through `aperture-cli commander-check`.
"""

import hashlib
import json
import pathlib
import urllib.request
from datetime import datetime, timezone

UA = "the-aperture-issue17/1.0 (offline supported-corpus fixture capture)"
ROOT = pathlib.Path(__file__).resolve().parents[2]
CORPUS = ROOT / "fixtures/starter-corpus"
DECK_TEXT = ROOT / "decks/starter-corpus/jasmine-vanilla-stompy.txt"


def sha(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def fetch_bulk_metadata():
    cache = ROOT / "evidence/issue17/scryfall-bulk-data.json"
    if cache.exists():
        return read_json(cache)
    request = urllib.request.Request(
        "https://api.scryfall.com/bulk-data",
        headers={"User-Agent": UA, "Accept": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        payload = json.load(response)
    cache.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    return payload


def parse_deck_text(text):
    """Mirror DeckList::parse_practice for this exact supported list."""
    entries = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("//") or line.startswith("#"):
            continue
        quantity, name = line.split(" ", 1)
        entries.append((name.strip(), int(quantity)))
    return entries


def main():
    responses = read_json(ROOT / "evidence/issue17/scryfall-responses.json")
    sourced = read_json(CORPUS / "sourced-cards.json")
    by_name = {card["name"]: card for card in sourced["cards"]}
    captured_at = responses["retrieved_at"]
    deck_entries = parse_deck_text(DECK_TEXT.read_text(encoding="utf-8"))

    commander_name = "Jasmine Boreal"
    if commander_name not in by_name:
        raise SystemExit("missing commander source record")
    commander = by_name[commander_name]
    if commander["color_identity"] != ["G", "W"]:
        raise SystemExit("commander identity drifted")

    library = []
    for name, quantity in deck_entries:
        record = by_name.get(name)
        if record is None:
            raise SystemExit(f"{name}: no sourced record")
        library.append(
            {
                "name": record["name"],
                "oracle_id": record["oracle_id"],
                "quantity": quantity,
            }
        )
    raw_deck = json.dumps(
        {
            "commanders": [
                {
                    "name": commander["name"],
                    "oracle_id": commander["oracle_id"],
                    "quantity": 1,
                }
            ],
            "companions": [],
            "excluded": [],
            "library": library,
            "sideboard": [],
        },
        separators=(",", ":"),
        sort_keys=True,
    )
    deck_snapshot = {
        "schema": "aperture.commander-deck.v1",
        "provenance": {
            "kind": "constructed",
            "provider": "the-aperture starter corpus",
            "source": "exact decks/starter-corpus/jasmine-vanilla-stompy.txt library plus the separately specified commander; identities from the recorded Scryfall printings",
            "captured_at": captured_at,
            "raw_sha256": sha(raw_deck),
            "note": "raw_sha256 covers the exact UTF-8 bytes of raw_deck. This is a constructed singleton starter list, not a captured third-party deck.",
        },
        "raw_deck": raw_deck,
    }
    (CORPUS / "jasmine-vanilla-stompy.deck.json").write_text(
        json.dumps(deck_snapshot, indent=1) + "\n", encoding="utf-8"
    )

    referenced = [commander_name] + [name for name, _ in deck_entries]
    if len(set(referenced)) != len(referenced):
        raise SystemExit("the fixture deck must name each unique card once")
    missing = [name for name in referenced if name not in responses["cards"]]
    if missing:
        raise SystemExit(f"no captured printing for {missing}")
    printings = []
    for name in sorted(referenced, key=str.lower):
        printings.append(responses["cards"][name]["printing"])
    raw_cards = "\n".join(
        json.dumps(card, separators=(",", ":"), sort_keys=True) for card in printings
    ) + "\n"
    cards_sha = sha(raw_cards)
    bulk = fetch_bulk_metadata()
    oracle = next(item for item in bulk["data"] if item["type"] == "oracle_cards")
    cards_snapshot = {
        "schema": "aperture.card-metadata.v1",
        "provenance": {
            "kind": "real",
            "provider": "scryfall",
            "api_url": "https://api.scryfall.com/bulk-data",
            "download_url": oracle["jsonl_download_uri"],
            "bulk_type": "oracle_cards",
            "api_updated_at": oracle["updated_at"],
            "captured_at": captured_at,
            "cards_sha256": cards_sha,
            "raw_sha256": cards_sha,
            "selection": "one concrete printing per referenced starter card, fetched with GET /cards/named?exact=... then GET /cards/{set}/{collector_number}; URLs are recorded per card in evidence/issue17/scryfall-responses.json",
            "card_count": len(printings),
            "note": "raw_cards_jsonl contains the exact real Scryfall printing objects for the starter package, one compact JSON object per line with a trailing newline. cards_sha256 and raw_sha256 both bind those bytes. Bulk metadata is recorded from /bulk-data as the snapshot timestamp/download reference only; the selected lines were fetched individually, not extracted from the bulk artifact.",
        },
        "raw_cards_jsonl": raw_cards,
    }
    (CORPUS / "jasmine-vanilla-stompy.cards.json").write_text(
        json.dumps(cards_snapshot, indent=1) + "\n", encoding="utf-8"
    )

    fixture_manifest = {
        "schema": "aperture.fixture-manifest.v1",
        "kind": "real",
        "source": "Scryfall per-card API (cards/named and cards/{set}/{collector_number}); bulk timestamp reference from /bulk-data",
        "captured_at": captured_at,
        "license_note": "Card names and Oracle text are Wizards of the Coast content. This is a minimal interoperability test subset, not a redistributable card database. Scryfall is the metadata source. No images or prices are used.",
        "selection": "every card referenced by decks/starter-corpus/jasmine-vanilla-stompy.txt plus its commander",
        "cards_sha256": cards_sha,
        "cards_hash_domain": "SHA-256 over the exact UTF-8 bytes of raw_cards_jsonl",
        "raw_sha256_note": "raw_sha256 and cards_sha256 both bind the exact embedded raw_cards_jsonl bytes. No full-bulk artifact digest is verified or presented as candidate provenance.",
        "deck_fixture_note": "jasmine-vanilla-stompy.deck.json is the exact starter list; it is a constructed list bound to real card identities, not a captured third-party deck.",
        "deck_sha256": sha(raw_deck),
        "deck_text_sha256": sha(DECK_TEXT.read_text(encoding="utf-8")),
        "bulk_metadata_updated_at": oracle["updated_at"],
        "bulk_metadata_download_url": oracle["jsonl_download_uri"],
        "cards": [
            {
                "name": responses["cards"][name]["printing"]["name"],
                "oracle_id": responses["cards"][name]["printing"]["oracle_id"],
                "scryfall_id": responses["cards"][name]["printing"]["id"],
                "layout": responses["cards"][name]["printing"]["layout"],
                "set": responses["cards"][name]["printing"]["set"],
                "collector_number": responses["cards"][name]["printing"]["collector_number"],
                "named_url": responses["cards"][name]["named_url"],
                "printing_url": responses["cards"][name]["printing_url"],
                "retrieved_at": captured_at,
            }
            for name in sorted(referenced, key=str.lower)
        ],
    }
    (CORPUS / "manifest.json").write_text(
        json.dumps(fixture_manifest, indent=1) + "\n", encoding="utf-8"
    )
    print(
        f"deck entries={len(deck_entries)} unique_cards={len(referenced)} "
        f"cards_sha256={cards_sha[:16]}"
    )


if __name__ == "__main__":
    main()
