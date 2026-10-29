"""Fetch and normalize the slice-33 sourced card (Beorn's Hospitality, HOB 120).

Run from the worktree root:
    python evidence/issue17/slice33/fetch-source.py

Writes:
    evidence/issue17/slice33/sources/hob-120-raw.json      (verbatim provider bytes)
    evidence/issue17/slice33/sources/hob-120-printings.json (all printings, for the
                                                             distinct-text check)
    fixtures/starter-corpus/slice33-sourced.json           (normalized record used by
                                                             crates/core/tests/card_printings.rs)

Every network call has a hard timeout; a timeout is a failure, never a silent
partial record.
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import pathlib
import subprocess
import sys
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[3]
OUT = ROOT / "evidence" / "issue17" / "slice33" / "sources"
FIXTURE = ROOT / "fixtures" / "starter-corpus" / "slice33-sourced.json"
NAMES = ["Beorn's Hospitality"]
TIMEOUT = 60


def get(url: str) -> dict:
    """Fetch JSON through curl.

    Python's urllib is refused (HTTP 400) by the provider from this host while
    the same URL answers 200 through curl, so the transport is curl. Every call
    is bounded by `--max-time`, and a non-zero exit is a hard failure: a timeout
    can never produce a partial record.
    """
    completed = subprocess.run(
        ["curl", "-s", "-S", "--max-time", str(TIMEOUT), "-A", "aperture-issue17/1.0", url],
        capture_output=True,
        timeout=TIMEOUT + 10,
        check=False,
    )
    if completed.returncode != 0:
        raise SystemExit(f"curl exit {completed.returncode} for {url}: {completed.stderr!r}")
    return json.loads(completed.stdout.decode("utf-8"))


def stamp() -> str:
    return _dt.datetime.now(_dt.timezone.utc).isoformat()


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    retrieved = stamp()
    records = []
    for name in NAMES:
        printed = get(
            "https://api.scryfall.com/cards/named?exact="
            + urllib.parse.quote(name)
        )
        raw_path = OUT / "hob-120-raw.json"
        raw_path.write_text(json.dumps(printed, indent=1) + "\n", encoding="utf-8")
        oracle_id = printed["oracle_id"]
        search = get(
            "https://api.scryfall.com/cards/search?q=oracleid%3A"
            + oracle_id
            + "&unique=prints"
        )
        (OUT / "hob-120-printings.json").write_text(
            json.dumps(search, indent=1) + "\n", encoding="utf-8"
        )
        texts = {card.get("oracle_text") for card in search["data"]}
        if len(texts) != 1:
            print(f"DISTINCT TEXT COUNT {len(texts)} for {name}: refusing", file=sys.stderr)
            return 1
        records.append(
            {
                "name": printed["name"],
                "oracle_id": oracle_id,
                "scryfall_id": printed["id"],
                "set": printed["set"],
                "collector_number": printed["collector_number"],
                "layout": printed["layout"],
                "mana_cost": printed["mana_cost"],
                "type_line": printed["type_line"],
                "power": printed.get("power"),
                "toughness": printed.get("toughness"),
                "oracle_text": printed["oracle_text"],
                "color_identity": printed["color_identity"],
                "keywords": printed["keywords"],
                "legalities_commander": printed["legalities"]["commander"],
                "printing_url": f"https://api.scryfall.com/cards/{printed['id']}",
                "retrieved_at": retrieved,
            }
        )
    FIXTURE.write_text(
        json.dumps(
            {
                "schema": "aperture.starter-sourced-cards.v1",
                "provider": "scryfall",
                "retrieved_at": retrieved,
                "cards": records,
            },
            indent=1,
        )
        + "\n",
        encoding="utf-8",
    )
    print("retrieved_at", retrieved)
    for record in records:
        print(record["name"], record["set"], record["collector_number"], record["scryfall_id"])
        print(record["oracle_text"].replace("\n", " | "))
    return 0


if __name__ == "__main__":
    sys.exit(main())
