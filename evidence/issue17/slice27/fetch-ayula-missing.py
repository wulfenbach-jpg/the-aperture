"""Fetch authoritative printing text for Ayula's missing names (slice 27 triage).

One Scryfall /cards/collection call; the raw response and a UTC retrieval stamp
are retained so any later implementation cites a real source.
"""
import json
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

OUT = Path("evidence/issue17/slice27/ayula-missing-sources")
OUT.mkdir(parents=True, exist_ok=True)

manifest = json.loads(Path("decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))
ayula = next(s for s in manifest["starters"] if s["id"] == "ayula-bear-counters")
names = sorted(ayula["missing_cards"])

req = urllib.request.Request(
    "https://api.scryfall.com/cards/collection",
    data=json.dumps({"identifiers": [{"name": n} for n in names]}).encode(),
    headers={
        "Content-Type": "application/json",
        "User-Agent": "the-aperture-issue17-corpus/1.0",
        "Accept": "application/json",
    },
    method="POST",
)
with urllib.request.urlopen(req, timeout=60) as resp:
    raw = resp.read().decode("utf-8")
stamp = datetime.now(timezone.utc).isoformat()
(OUT / "collection-raw.json").write_text(raw, encoding="utf-8")
(OUT / "retrieval.json").write_text(
    json.dumps({"retrieved_utc": stamp, "endpoint": req.full_url,
                "requested_names": names}, indent=2), encoding="utf-8")

data = json.loads(raw)
not_found = [i.get("name") for i in data.get("not_found", [])]
rows = []
for card in data["data"]:
    rows.append({
        "name": card["name"],
        "set": card["set"],
        "collector_number": card["collector_number"],
        "scryfall_id": card["id"],
        "oracle_id": card["oracle_id"],
        "mana_cost": card.get("mana_cost"),
        "type_line": card.get("type_line"),
        "power": card.get("power"),
        "toughness": card.get("toughness"),
        "oracle_text": card.get("oracle_text"),
        "colors": card.get("colors"),
        "color_identity": card.get("color_identity"),
        "cmc": card.get("cmc"),
    })
rows.sort(key=lambda r: r["name"])
(OUT / "cards.json").write_text(json.dumps(rows, indent=2), encoding="utf-8")

print("retrieved_utc", stamp)
print("found", len(rows), "not_found", len(not_found), not_found)
for r in rows:
    text = (r["oracle_text"] or "").replace("\n", " | ")
    print(f"\n## {r['name']} ({r['set'].upper()} {r['collector_number']}) {r['mana_cost']} "
          f"{r['type_line']} {r['power']}/{r['toughness']}\n   {text}")
