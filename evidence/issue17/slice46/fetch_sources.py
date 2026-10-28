import json, urllib.request, urllib.parse, sys, os, hashlib, datetime

names = ["Dúnedain Rangers", "Evercoat Ursine", "Owlbear Cub", "Rampaging Yao Guai",
         "Ruxa, Patient Professor", "Thijarian Witness", "Ursine Monstrosity"]
out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sources")
os.makedirs(out, exist_ok=True)
stamp = datetime.datetime.now(datetime.timezone.utc).isoformat()
for n in names:
    url = "https://api.scryfall.com/cards/named?fuzzy=" + urllib.parse.quote(n)
    with urllib.request.urlopen(url, timeout=45) as r:
        raw = r.read()
    d = json.loads(raw)
    slug = "".join(c.lower() if c.isalnum() else "-" for c in d["name"]).strip("-")
    slug = "-".join(filter(None, slug.split("-")))
    with open(os.path.join(out, slug + ".json"), "wb") as f:
        f.write(raw)
    meta = {
        "name": d["name"], "set": d.get("set", "").upper(), "collector_number": d.get("collector_number"),
        "mana_cost": d.get("mana_cost"), "type_line": d.get("type_line"),
        "power": d.get("power"), "toughness": d.get("toughness"), "oracle_text": d.get("oracle_text"),
        "printing_id": d["id"], "oracle_id": d["oracle_id"],
        "retrieved_utc": stamp, "request_url": url,
        "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw),
    }
    with open(os.path.join(out, slug + ".stamp.json"), "w") as f:
        json.dump(meta, f, indent=2, ensure_ascii=False)
    print("==", meta["name"], meta["set"], meta["collector_number"], meta["mana_cost"], "|", meta["type_line"],
          "|", meta["power"], meta["toughness"])
    print(meta["oracle_text"])
    print("ids", meta["printing_id"], meta["oracle_id"], "sha", meta["sha256"][:16], meta["bytes"])
