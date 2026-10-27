import json, re
from pathlib import Path
ROOT = Path(r"C:\AI\aperture-worktrees\issue-17")
cat = (ROOT/"crates/core/src/catalog.rs").read_text(encoding="utf-8")
names = set()
for m in re.finditer(r'card(?:_[a-z]+)?!\(\s*\w+,\s*"((?:[^"\\]|\\.)*)"', cat):
    names.add(m.group(1).replace('\\"','"'))
manifest = json.loads((ROOT/"decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))
union=set(); per={}; problems=[]
for s in manifest["starters"]:
    entries=[]
    for line in (ROOT/s["deck_path"]).read_text(encoding="utf-8").splitlines():
        line=line.strip()
        if not line or line.startswith("#") or line.startswith("//"):
            continue
        m=re.match(r'^(\d+)\s+(.+)$', line)
        entries.append(m.group(2).strip() if m else line)
    full=list(entries)
    if s["commander"] not in entries:
        full.append(s["commander"])
    union |= set(full)
    missing=sorted(n for n in set(full) if n not in names)
    per[s["id"]]=missing
    rec=set(s.get("missing_cards",[]))
    if rec!=set(missing):
        problems.append((s["id"], len(missing), len(rec), sorted(set(missing)-rec)[:5], sorted(rec-set(missing))[:5]))
    total=sum(int(m.group(1)) for m in (re.match(r'^(\d+)\s+', l.strip()) for l in (ROOT/s["deck_path"]).read_text(encoding="utf-8").splitlines()) if m)
    print(f'{s["id"]:<26} distinct_entries={len(full):>3} deck_total_cards={total:>3} unsupported={len(missing):>3}')
sup=[n for n in union if n in names]; miss=[n for n in union if n not in names]
print()
print("catalog:", len(names), "union:", len(union), "supported:", len(sup), "missing:", len(miss))
print("per-list missing:", [len(per[s["id"]]) for s in manifest["starters"]])
print("ayula:", len(per["ayula-bear-counters"]), per["ayula-bear-counters"])
print("set mismatches:", problems if problems else "none")
print("statuses:", {k: sum(1 for s in manifest["starters"] if s["status"]==k) for k in {s["status"] for s in manifest["starters"]}})
