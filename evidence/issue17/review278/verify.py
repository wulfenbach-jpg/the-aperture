from pathlib import Path
import json,re,subprocess
root=Path(__file__).resolve().parents[3]
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()=='ea106932d649da6876004fdf5c1a2ce6e441d19c'
src=(root/'crates/core/src/catalog.rs').read_text(encoding='utf-8').partition('pub static TOKENS')[0]
pool=set(re.findall(r'card(?:_[a-z]+)?!\(\s*[A-Za-z0-9_]+,\s*"([^"]+)"',src,re.S))
m=json.loads((root/'decks/starter-corpus/manifest.json').read_text()); union=set();rows=[]
for s in m['starters']:
    entries=[line.strip().split(' ',1) for line in (root/s['deck_path']).read_text().splitlines() if line.strip() and not line.strip().startswith('//')]
    assert sum(int(q) for q,n in entries)==99
    required={n.strip() for q,n in entries}|{s['commander']};missing=required-pool
    assert missing==set(s['missing_cards']);union|=required
    rows.append((s['id'],len(missing),s['status']))
r={'catalog':len(pool),'union':len(union),'supported':len(union&pool),'missing':len(union-pool),'lists':rows,'focused_tests':sum([13,10,7,12])}
print(json.dumps(r,indent=2));(Path(__file__).parent/'corpus.json').write_text(json.dumps(r,indent=2))
# Own driver copy: same executable behavior, separate output paths, no shipped evidence overwritten.
a=root/'evidence/issue17/slice25-journey-driver';b=root/'evidence/issue17/review278-driver'
(b/'src').mkdir(parents=True,exist_ok=True)
(b/'Cargo.toml').write_text((a/'Cargo.toml').read_text().replace('issue17-slice25-journey','issue17-review278-journey'))
(b/'Cargo.lock').write_bytes((a/'Cargo.lock').read_bytes())
(b/'src/main.rs').write_text((a/'src/main.rs').read_text().replace('evidence/issue17/slice25','evidence/issue17/review278'))
