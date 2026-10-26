import json, urllib.request, pathlib, datetime

names = ["Ordinary Bear", "Ashcoat Bear", "Owlbear", "Soul Warden"]
roles = {
    "Ordinary Bear": "vanilla",
    "Ashcoat Bear": "flash",
    "Owlbear": "trample",
    "Soul Warden": "another_creature_enters",
}
stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def get(name):
    url = "https://api.scryfall.com/cards/named?exact=" + urllib.request.quote(name)
    req = urllib.request.Request(url, headers={
        "User-Agent": "aperture-corpus/1.0", "Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.loads(r.read())


# one printing each; also collect the distinct oracle texts across printings to
# prove the wording did not change between printings
out = []
for name in names:
    c = get(name)
    prints = json.loads(urllib.request.urlopen(urllib.request.Request(
        "https://api.scryfall.com/cards/search?unique=prints&q=" +
        urllib.request.quote('!"%s"' % name),
        headers={"User-Agent": "aperture-corpus/1.0", "Accept": "application/json"}),
        timeout=60).read())
    texts = sorted({p.get("oracle_text", "") for p in prints["data"]})
    out.append({
        "name": c["name"],
        "role": roles[name],
        "oracle_id": c["oracle_id"],
        "scryfall_id": c["id"],
        "set": c["set"],
        "collector_number": c["collector_number"],
        "set_name": c["set_name"],
        "released_at": c["released_at"],
        "layout": c["layout"],
        "mana_cost": c.get("mana_cost", ""),
        "type_line": c["type_line"],
        "power": c.get("power"),
        "toughness": c.get("toughness"),
        "oracle_text": c.get("oracle_text", ""),
        "color_identity": c.get("color_identity", []),
        "keywords": c.get("keywords", []),
        "legalities_commander": c["legalities"]["commander"],
        "printing_url": "https://api.scryfall.com/cards/%s/%s" % (c["set"], c["collector_number"]),
        "printings_checked": len(prints["data"]),
        "distinct_printing_texts": len(texts),
        "retrieved_at": stamp,
    })
    print(name, c["set"], c["collector_number"], "printings", len(prints["data"]),
          "distinct texts", len(texts), repr(c.get("oracle_text")))

doc = {
    "schema": "aperture.starter-sourced-cards.v1",
    "provider": "scryfall",
    "retrieved_at": stamp,
    "note": ("Four records read from live Scryfall this run (named+exact and "
             "unique=prints); every printing of each card carries the same "
             "oracle text."),
    "cards": out,
}
root = pathlib.Path(__file__).resolve().parents[3]
(root / "fixtures/starter-corpus/slice4-sourced.json").write_text(
    json.dumps(doc, indent=1) + "\n")
(pathlib.Path(__file__).resolve().parent / "slice4-sourced-raw.json").write_text(
    json.dumps(doc, indent=1) + "\n")
print("wrote", root / "fixtures/starter-corpus/slice4-sourced.json")
