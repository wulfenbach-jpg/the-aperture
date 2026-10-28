"""Read-only recheck of the exact Ruxa printing already pinned by the slice."""
import datetime
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
fixture = json.loads((ROOT / "fixtures/starter-corpus/slice53-sourced.json").read_text(encoding="utf-8"))["cards"][0]
request = urllib.request.Request(fixture["printing_url"], headers={"User-Agent": "TheAperture-corpus-verification/1.0", "Accept": "application/json"})
with urllib.request.urlopen(request, timeout=45) as response:
    raw = response.read()
    status = response.status
card = json.loads(raw)
assert status == 200
for key in ("name", "oracle_id", "mana_cost", "oracle_text", "type_line", "power", "toughness", "keywords", "colors", "color_identity", "layout", "set", "collector_number"):
    assert card[key] == fixture[key], (key, card[key], fixture[key])
assert card["id"] == fixture["scryfall_id"]
(OUT / "ruxa-printing-live.json").write_bytes(raw)
receipt = {"url": fixture["printing_url"], "retrieved_at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "http_status": status, "id": card["id"], "fixture_fields_match": True, "raw_file": "ruxa-printing-live.json"}
(OUT / "printing-recheck.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
print(json.dumps(receipt, indent=2))
