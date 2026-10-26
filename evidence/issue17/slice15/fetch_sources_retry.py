"""Fetch the remaining Ayula names that hit Scryfall's rate limit, with backoff.

Merges into sources-ayula-raw.json in place so the evidence file stays one
complete batch with one retrieval stamp per card.
"""

import json
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone

UA = "aperture-issue17/1.0 (starter corpus sourcing)"
PATH = "evidence/issue17/slice15/sources-ayula-raw.json"


def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def main():
    out = json.load(open(PATH))
    have = {c["name"] for c in out["cards"]}
    pending = [e["name"] for e in out["errors"]]
    still = []
    for name in pending:
        if name in have:
            continue
        url = "https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(name)
        for attempt in range(6):
            try:
                c = get(url)
                break
            except Exception as exc:  # noqa: BLE001
                last = str(exc)
                time.sleep(2.0 + attempt * 2.0)
                c = None
        if c is None:
            still.append({"name": name, "error": last})
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
        time.sleep(1.2)
    out["errors"] = still
    with open(PATH, "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1, ensure_ascii=False)
    print("total", len(out["cards"]), "remaining errors", len(still))
    for e in still:
        print("ERR", e)


if __name__ == "__main__":
    main()
