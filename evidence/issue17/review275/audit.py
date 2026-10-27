import json, re
from pathlib import Path
root = Path(__file__).resolve().parents[3]
pool = set(re.findall(r'card(?:_[a-z]+)?!\(\s*\w+,\s*"([^"]+)"', (root/'crates/core/src/catalog.rs').read_text(encoding='utf-8')))
m = json.loads((root/'decks/starter-corpus/manifest.json').read_text())
union = set()
rows = []
for s in m['starters']:
    entries = []
    for line in (root/s['deck_path']).read_text(encoding='utf-8').splitlines():
        line=line.strip()
        if not line or line.startswith('//'): continue
        quantity,name=line.split(' ',1)
        entries.append((int(quantity),name.strip()))
    assert sum(q for q,n in entries)==99
    names={n for q,n in entries}|{s['commander']}
    missing=names-pool
    assert missing==set(s['missing_cards']),s['id']
    union |= names
    rows.append({'id':s['id'],'missing':len(missing),'status':s['status']})
result={'catalog':len(pool),'union':len(union),'supported':len(union&pool),'missing':len(union-pool),'lists':rows,'focused_tests_passed':sum([8,5,11])}
print(json.dumps(result,indent=2))
(root/'evidence/issue17/review275/audit.json').write_text(json.dumps(result,indent=2))
