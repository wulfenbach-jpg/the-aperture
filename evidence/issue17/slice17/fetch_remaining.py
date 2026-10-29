"""Capture remaining Ayula texts for SAME issue17 continuation; no support claim."""
import datetime
import json
from pathlib import Path
import time
import urllib.parse
import urllib.request

root = Path(__file__).resolve().parents[3]
manifest = json.loads((root / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))
names = next(s["missing_cards"] for s in manifest["starters"] if s["id"] == "ayula-bear-counters")
out = {"purpose": "source preparation only; every card here remains unsupported", "cards": [], "errors": []}
dest = Path(__file__).with_name("remaining-ayula-sources.json")
for name in names:
    time.sleep(0.4)
    url = "https://api.scryfall.com/cards/named?exact=" + urllib.parse.quote(name)
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "ApertureCorpus/1.0", "Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=20) as response:
            card = json.load(response)
        assert card["name"] == name, (name, card["name"])
        fields = ["name", "id", "oracle_id", "set", "collector_number", "layout", "mana_cost", "type_line", "oracle_text", "power", "toughness", "color_identity", "keywords", "legalities"]
        record = {k: card.get(k) for k in fields}
        record["printing_url"] = "https://api.scryfall.com/cards/" + card["id"]
        record["retrieved_at"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        out["cards"].append(record)
    except Exception as error:
        out["errors"].append({"name": name, "error": str(error)})
    dest.write_text(json.dumps(out, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps({"requested": len(names), "captured": len(out["cards"]), "errors": out["errors"]}, indent=2))
