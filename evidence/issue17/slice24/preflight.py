from pathlib import Path
import json, hashlib, shutil, urllib.request, datetime
root=Path.cwd(); out=root/'evidence/issue17/slice24'; out.mkdir(parents=True,exist_ok=True)
records=[]
for rel in ['evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl','evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl']:
 p=root/rel; assert p.is_file(); files=[p]+list(p.with_suffix('').rglob('*'))
 for f in files:
  if not f.is_file(): continue
  dest=root/'evidence/issue17/slice24-backup'/f.relative_to(root); dest.parent.mkdir(parents=True,exist_ok=True); shutil.copy2(f,dest)
  a=hashlib.sha256(f.read_bytes()).hexdigest(); assert a==hashlib.sha256(dest.read_bytes()).hexdigest(); records.append({'path':str(f.relative_to(root)),'sha256':a})
(out/'history-backup.json').write_text(json.dumps(records,indent=2))
c=next(c for c in json.loads((root/'evidence/issue17/slice17/remaining-ayula-sources.json').read_text())['cards'] if c['name']=='Surrak and Goreclaw')
req=urllib.request.Request(c['printing_url'],headers={'User-Agent':'TheAperture/issue17 (local card implementation sourcing)','Accept':'application/json'})
raw=urllib.request.urlopen(req,timeout=40).read(); (out/'source-raw.json').write_bytes(raw)
d=json.loads(raw); stamp=datetime.datetime.now(datetime.timezone.utc).isoformat(); (out/'source-retrieval.json').write_text(json.dumps({'url':c['printing_url'],'retrieved_at':stamp,'sha256':hashlib.sha256(raw).hexdigest()},indent=2))
print('backed up',len(records),'history files'); print(json.dumps({k:d.get(k) for k in ['id','name','set','collector_number','mana_cost','type_line','oracle_text','power','toughness']},indent=2)); print('retrieved',stamp)
print('bulk jobs registry:'); print(Path('C:/AI/mtg-slice/card-corpus-work/JOBS.json').read_text()[:15000])
