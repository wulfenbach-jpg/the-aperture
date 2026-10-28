import json, urllib.request, urllib.parse, os, hashlib, datetime, time

names = ["Dúnedain Rangers", "Evercoat Ursine", "Owlbear Cub", "Rampaging Yao Guai",
         "Ruxa, Patient Professor", "Thijarian Witness", "Ursine Monstrosity"]
base = os.path.dirname(os.path.abspath(__file__))
out = os.path.join(base, "sources")
os.makedirs(out, exist_ok=True)
stamp = datetime.datetime.now(datetime.timezone.utc).isoformat()
hdr = {"User-Agent": "aperture-corpus/1.0", "Accept": "application/json"}
for n in names:
    got = None
    for kind, url in (
        ("named_fuzzy", "https://api.scryfall.com/cards/named?fuzzy=" + urllib.parse.quote(n)),
        ("search_exact", "https://api.scryfall.com/cards/search?q=" + urllib.parse.quote('!"%s"' % n)),
    ):
        req = urllib.request.Request(url, headers=hdr)
        try:
            with urllib.request.urlopen(req, timeout=45) as r:
                raw = r.read()
            d = json.loads(raw)
            if kind == "search_exact":
                d = d["data"][0]
            got = (kind, url, raw, d)
            break
        except Exception as e:
            body = ""
            try:
                body = e.read().decode()[:300]
            except Exception:
                pass
            print("fail", kind, n, e, body)
        time.sleep(0.2)
    if not got:
        print("UNAVAILABLE", n)
        continue
    kind, url, raw, d = got
    slug = "".join(c.lower() if c.isalnum() else "-" for c in d["name"])
    slug = "-".join(filter(None, slug.split("-")))
    with open(os.path.join(out, slug + ".json"), "wb") as f:
        f.write(raw)
    meta = {
        "name": d["name"], "set": d.get("set", "").upper(), "collector_number": d.get("collector_number"),
        "mana_cost": d.get("mana_cost"), "type_line": d.get("type_line"),
        "power": d.get("power"), "toughness": d.get("toughness"), "oracle_text": d.get("oracle_text"),
        "printing_id": d["id"], "oracle_id": d["oracle_id"], "retrieved_utc": stamp,
        "request_url": url, "request_kind": kind,
        "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw),
    }
    with open(os.path.join(out, slug + ".stamp.json"), "w") as f:
        json.dump(meta, f, indent=2, ensure_ascii=False)
    print("==", meta["name"], meta["set"], meta["collector_number"], meta["mana_cost"], "|", meta["type_line"],
          "|", meta["power"], meta["toughness"])
    print(meta["oracle_text"])
    print("ids", meta["printing_id"], meta["oracle_id"], "sha", meta["sha256"][:16], meta["bytes"])
