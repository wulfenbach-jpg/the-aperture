"""Build the slice-64 sourced-card record from the raw Scryfall fetches.

Reads the four `<slug>-raw.json` / `<slug>-printings.json` files written by curl
in evidence/issue17/slice64/sources, checks that every printing of each card
carries one distinct Oracle text, and writes:

  * evidence/issue17/slice64/sources/<slug>-source.json  (the summary a reviewer
    checks against the live API), and
  * fixtures/starter-corpus/slice64-sourced.json         (the pool's record).

Nothing is invented: every field is copied out of the fetch, and the script
fails loudly if a printing disagrees about the text.
"""

import datetime
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[3]
SOURCES = ROOT / "evidence" / "issue17" / "slice64" / "sources"
FIXTURE = ROOT / "fixtures" / "starter-corpus" / "slice64-sourced.json"

CARDS = [
    ("goblin-sharpshooter", "Goblin Sharpshooter"),
    ("krenko-tin-street-kingpin", "Krenko, Tin Street Kingpin"),
    ("hobgoblin-bandit-lord", "Hobgoblin Bandit Lord"),
    ("goblin-matron", "Goblin Matron"),
]


def main() -> None:
    recorded = []
    retrieved = None
    for slug, name in CARDS:
        raw = json.loads((SOURCES / f"{slug}-raw.json").read_text(encoding="utf-8"))
        if raw.get("object") != "card":
            raise SystemExit(f"{name}: raw fetch is not a card: {raw}")
        if raw["name"] != name:
            raise SystemExit(f"{name}: raw fetch is {raw['name']}")
        printings = json.loads(
            (SOURCES / f"{slug}-printings.json").read_text(encoding="utf-8")
        )
        if printings.get("object") != "list":
            raise SystemExit(f"{name}: printings fetch is not a list: {printings}")
        rows = printings["data"]
        texts = {row.get("oracle_text") for row in rows}
        if len(texts) != 1:
            raise SystemExit(f"{name}: printings disagree about the Oracle text: {texts}")
        if raw.get("oracle_text") not in texts:
            raise SystemExit(f"{name}: chosen printing text is not the shared text")
        # The retrieval timestamp is the printings fetch's own file mtime, so the
        # record cannot claim a time the files do not have.
        stamp = (
            datetime.datetime.utcfromtimestamp(
                (SOURCES / f"{slug}-printings.json").stat().st_mtime
            )
            .replace(microsecond=0)
            .isoformat()
            + "Z"
        )
        retrieved = retrieved or stamp
        summary = {
            "name": raw["name"],
            "set": raw["set"],
            "collector_number": raw["collector_number"],
            "oracle_id": raw["oracle_id"],
            "mana_cost": raw.get("mana_cost"),
            "type_line": raw["type_line"],
            "power": raw.get("power"),
            "toughness": raw.get("toughness"),
            "oracle_text": raw["oracle_text"],
            "keywords": raw.get("keywords", []),
            "colors": raw.get("colors", []),
            "color_identity": raw.get("color_identity", []),
            "layout": raw.get("layout"),
            "released_at": raw.get("released_at"),
            "scryfall_id": raw["id"],
            "retrieved_at_utc": stamp,
            "provider": "scryfall",
            "url": raw["scryfall_uri"].split("?")[0],
            "legalities_commander": raw.get("legalities", {}).get("commander"),
        }
        (SOURCES / f"{slug}-source.json").write_text(
            json.dumps(summary, indent=2) + "\n", encoding="utf-8"
        )
        entry = dict(summary)
        entry.pop("provider", None)
        entry.pop("url", None)
        entry.pop("retrieved_at_utc", None)
        entry["mana_value"] = raw.get("cmc")
        entry["printings_checked"] = len(rows)
        entry["printings_distinct_oracle_texts"] = len(texts)
        entry["printings"] = sorted(
            f"{row['set']}/{row['collector_number']}" for row in rows
        )
        entry["retrieved_at_utc"] = stamp
        recorded.append(entry)

    FIXTURE.write_text(
        json.dumps(
            {
                "schema": "aperture.starter-sourced-cards.v1",
                "provider": "scryfall",
                "retrieved_at": retrieved,
                "cards": recorded,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    for entry in recorded:
        print(
            f"{entry['name']} | {entry['set']} {entry['collector_number']} | "
            f"{entry['mana_cost']} | {entry['type_line']} | "
            f"{entry['power']}/{entry['toughness']} | "
            f"{entry['printings_checked']} printings | "
            f"{entry['printings_distinct_oracle_texts']} text"
        )


if __name__ == "__main__":
    main()
