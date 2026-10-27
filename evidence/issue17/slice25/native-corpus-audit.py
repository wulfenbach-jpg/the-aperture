"""Independent each-list audit of current slice25, preserving older audit artifacts."""
from pathlib import Path
import json,re,subprocess,hashlib
root=Path(__file__).resolve().parents[3]
src=(root/'crates/core/src/catalog.rs').read_text(encoding='utf-8').partition('pub static TOKENS')[0]
names=re.findall(r'card(?:_[a-z]+)?!\(\s*[A-Za-z0-9_]+,\s*"([^"]+)"',src,re.S)
assert len(names)==len(set(names)), 'duplicate catalog names'
pool=set(names); m=json.loads((root/'decks/starter-corpus/manifest.json').read_text()); union=set(); rows=[]
for s in m['starters']:
 raw=(root/s['deck_path']).read_bytes(); entries=[]
 for line in raw.decode().splitlines():
  line=line.strip()
  if not line or line.startswith('//'): continue
  qty,name=line.split(' ',1); entries.append((int(qty),name.strip()))
 assert sum(q for q,n in entries)==99,(s['id'],'library length')
 required={n for _,n in entries}|{s['commander']}; missing=required-pool
 assert missing==set(s['missing_cards']),(s['id'],sorted(missing^set(s['missing_cards'])))
 union|=required; rows.append({'id':s['id'],'cards_with_commander':100,'missing':len(missing),'status':s['status'],'snapshot_sha256':hashlib.sha256(raw).hexdigest()})
r={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'catalog':len(pool),'union':len(union),'supported':len(union&pool),'missing':len(union-pool),'certified':sum(s['status']=='certified' for s in rows),'desired':len(rows),'lists':rows}
assert len(rows)==8
print(json.dumps(r,indent=2)); (Path(__file__).parent/'native-corpus-audit.json').write_text(json.dumps(r,indent=2)+'\n')
