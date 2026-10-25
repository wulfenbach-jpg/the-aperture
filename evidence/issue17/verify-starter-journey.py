#!/usr/bin/env python3
"""Assert the recorded four-seat starter journey is exact and digest-verified.

Reads the artifacts produced by:
  aperture-cli selfplay --seed 1 --max-steps 12000 \
    --deck decks/starter-corpus/jasmine-vanilla-stompy.txt (x4) \
    --commander-combat-practice --commander 0..3:Jasmine Boreal \
    --verbose --out ... --replay-out ...
"""

import collections
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
GAME = ROOT / "evidence/issue17/jasmine-starter-game.json"
REPLAY = ROOT / "evidence/issue17/jasmine-starter-game.replay.json"
LOG = ROOT / "evidence/issue17/jasmine-starter-game.verbose.log"
DECK_TEXT = ROOT / "decks/starter-corpus/jasmine-vanilla-stompy.txt"
SOURCED = ROOT / "fixtures/starter-corpus/sourced-cards.json"


def variant(name):
    """CardId enum variant for a printed name (same rule as the catalog)."""
    return "".join(part.capitalize() for part in re.findall(r"[A-Za-z0-9]+", name))


def main():
    deck = collections.Counter()
    for line in DECK_TEXT.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("//"):
            continue
        quantity, name = line.split(" ", 1)
        deck[variant(name.strip())] += int(quantity)
    assert sum(deck.values()) == 99 and len(deck) == 60

    report = json.loads(GAME.read_text(encoding="utf-8"))
    replay = json.loads(REPLAY.read_text(encoding="utf-8"))
    log = LOG.read_text(encoding="utf-8")
    assert report["status"] == "terminal", report["status"]
    assert report["starting_life"] == 40
    assert report["seat_count"] == 4
    assert report["pool_size"] == 79
    assert report["final_digest"] == replay["final_digest"]
    assert report["replay_digest_matches"] is True

    for seat, snapshot in enumerate(replay["seats"]):
        cards = collections.Counter()
        for card, quantity in snapshot["cards"]:
            cards[card] += quantity
        assert dict(cards) == dict(deck), f"seat {seat} library is not the exact list"
        assert sum(cards.values()) == 99
    assert [row["card"] for row in replay["commanders"]] == ["JasmineBoreal"] * 4
    assert [row["source"] for row in report["seats"]] == [
        "decks/starter-corpus/jasmine-vanilla-stompy.txt"
    ] * 4

    casts = collections.Counter(re.findall(r"P\d casts (.+)", log))
    assert casts["Jasmine Boreal"] >= 4, "every seat must cast its commander"
    sourced = json.loads(SOURCED.read_text(encoding="utf-8"))
    vanilla = [card["name"] for card in sourced["cards"] if card["role"] == "vanilla"]
    used = sorted(name for name in vanilla if casts[name])
    assert len(used) >= 20, f"only {len(used)} new vanilla cards were cast"

    print(
        json.dumps(
            {
                "status": report["status"],
                "winner": report["winner"],
                "steps": report["steps"],
                "turns": report["turns"],
                "starting_life": report["starting_life"],
                "seat_libraries": [99] * 4,
                "commanders": ["Jasmine Boreal"] * 4,
                "final_digest": report["final_digest"],
                "replay_matches": True,
                "casts_total": sum(casts.values()),
                "new_vanilla_cards_cast": len(used),
                "commander_casts": casts["Jasmine Boreal"],
            },
            indent=1,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
