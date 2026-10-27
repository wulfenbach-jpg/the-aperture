"""Retry the three names that hit Scryfall's 429 rate limit in fetch-source.py."""
import datetime
import json
import os
import time
import urllib.parse
import urllib.request

NAMES = ["Ursine Champion", "Ursine Monstrosity", "Words of Wilding"]

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "evidence", "issue17", "slice26")
RAW = os.path.join(OUT, "source-raw")

path = os.path.join(OUT, "source-retrieval.json")
with open(path, encoding="utf-8") as handle:
    payload = json.load(handle)

stamp = datetime.datetime.now(datetime.timezone.utc).isoformat()
payload.setdefault("retry_retrievals", []).append({"retrieved_at": stamp, "names": NAMES})

for name in NAMES:
    url = "https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(name)
    for attempt in range(6):
        try:
            req = urllib.request.Request(
                url,
                headers={"User-Agent": "aperture-issue17/1.0", "Accept": "application/json"},
            )
            with urllib.request.urlopen(req, timeout=30) as response:
                data = json.load(response)
            break
        except Exception as exc:  # noqa: BLE001
            print("retry", name, attempt, exc)
            time.sleep(4 * (attempt + 1))
    else:
        print("FAILED", name)
        continue
    payload["cards"][name] = {
        "url": url,
        "id": data.get("id"),
        "oracle_id": data.get("oracle_id"),
        "set": data.get("set"),
        "collector_number": data.get("collector_number"),
        "name": data.get("name"),
        "mana_cost": data.get("mana_cost"),
        "type_line": data.get("type_line"),
        "oracle_text": data.get("oracle_text"),
        "power": data.get("power"),
        "toughness": data.get("toughness"),
        "colors": data.get("colors"),
        "layout": data.get("layout"),
        "retrieved_at": stamp,
    }
    with open(
        os.path.join(RAW, f"{data.get('set')}-{data.get('collector_number')}.json"),
        "w",
        encoding="utf-8",
    ) as handle:
        json.dump(data, handle, indent=1, ensure_ascii=False)
    print(
        f"{name} | {data.get('set')} {data.get('collector_number')} | {data.get('mana_cost')} | "
        f"{data.get('type_line')} | {data.get('power')}/{data.get('toughness')} || {data.get('oracle_text')}"
    )
    time.sleep(2)

with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=1, ensure_ascii=False)
