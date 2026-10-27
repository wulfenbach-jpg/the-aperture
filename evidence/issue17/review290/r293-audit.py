import json, re, sys
root = "C:/AI/r293-review"
pool = set()
marker = None
for line in open("C:/AI/r293-probe.log", encoding="utf-8"):
    line = line.rstrip("\n")
    if line.startswith("PROBE_MARKER"):
        marker = line
        continue
    if line:
        pool.add(line)
assert marker == "PROBE_MARKER count=149", marker
assert len(pool) == 149, len(pool)

m = json.load(open(root + "/decks/starter-corpus/manifest.json", encoding="utf-8"))
union = set()
per_list = []
for s in m["starters"]:
    path = root + "/" + s["deck_path"]
    names = []
    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if not line or line.startswith("//"):
            continue
        mm = re.match(r"^(\d+)\s+(.+)$", line)
        if not mm:
            print("UNPARSED DECK LINE:", path, repr(line)); sys.exit(2)
        names.append(mm.group(2).strip())
    cards = set(names)
    deck = cards | {s["commander"]}
    missing = sorted(deck - pool)
    declared = set(s["missing_cards"])
    per_list.append(len(missing))
    union |= deck
    if declared != set(missing):
        print("MANIFEST MISMATCH", s["id"], "declared-only:", sorted(declared - set(missing))[:5],
              "audit-only:", sorted(set(missing) - declared)[:5])
        sys.exit(3)
    print(f"{s['id']:24s} status={s['status']:9s} deckcards={len(cards):3d} +commander={s['commander']!r} missing={len(missing):3d} declared={len(declared):3d}")

supported = sorted(union & pool)
missing = sorted(union - pool)
print()
print("pool", len(pool), "union", len(union), "supported", len(supported), "missing", len(missing))
print("per_list_missing", per_list)
print("certified", sum(1 for s in m["starters"] if s["status"] == "certified"), "of", len(m["starters"]))
