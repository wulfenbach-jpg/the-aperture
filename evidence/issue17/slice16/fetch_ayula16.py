"""Fetch authoritative printed text for the remaining Ayula missing names (slice 16)."""

import json
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone

UA = "aperture-issue17/1.0 (starter corpus sourcing)"


def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def main():
    manifest = json.load(open("decks/starter-corpus/manifest.json"))
    ayula = [s for s in manifest["starters"] if s["id"] == "ayula-bear-counters"][0]
    names = ayula["missing_cards"]
    out = {"schema": "aperture.starter-sourced-cards.v1", "provider": "scryfall",
           "retrieved_at": datetime.now(timezone.utc).isoformat(), "cards": []}
    errors = []
    for name in names:
        url = "https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(name)
        try:
            c = get(url)
        except Exception as exc:  # noqa: BLE001
            errors.append({"name": name, "error": str(exc)})
            time.sleep(0.12)
            continue
        out["cards"].append({
            "name": c["name"],
            "oracle_id": c["oracle_id"],
            "scryfall_id": c["id"],
            "set": c["set"],
            "collector_number": c["collector_number"],
            "layout": c.get("layout"),
            "mana_cost": c.get("mana_cost"),
            "type_line": c.get("type_line"),
            "power": c.get("power"),
            "toughness": c.get("toughness"),
            "oracle_text": c.get("oracle_text"),
            "color_identity": c.get("color_identity", []),
            "keywords": c.get("keywords", []),
            "legalities_commander": c.get("legalities", {}).get("commander"),
            "printing_url": "https://api.scryfall.com/cards/" + c["id"],
            "retrieved_at": datetime.now(timezone.utc).isoformat(),
        })
        time.sleep(0.12)
    out["errors"] = errors
    with open("evidence/issue17/slice16/sources-ayula-raw.json", "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1, ensure_ascii=False)
    print("fetched", len(out["cards"]), "errors", len(errors))
    for c in out["cards"]:
        print("====", c["name"], "|", c["mana_cost"], "|", c["type_line"], "|",
              c["power"], "/", c["toughness"], "|", c["set"], c["collector_number"])
        print("   ", (c["oracle_text"] or "").replace("\n", " // "))
        print("    keywords:", c["keywords"])
    for e in errors:
        print("ERR", e)


if __name__ == "__main__":
    main()
