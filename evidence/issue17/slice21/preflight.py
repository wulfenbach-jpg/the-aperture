from pathlib import Path
import json,shutil,hashlib,psutil,urllib.request,datetime
root=Path.cwd(); out=root/'evidence/issue17/slice21'; out.mkdir(exist_ok=True)
for p in psutil.process_iter(['pid','name','cmdline']):
 if p.info['name'] and any(s in p.info['name'].lower() for s in ['bun','cargo','rustc']): print('PROCESS',p.info)
files=[]
for name in ['sessions','review-sessions']:
 src=root/'evidence/issue17'/name; dst=root/'evidence/issue17/slice21-backup'/name
 shutil.copytree(src,dst,dirs_exist_ok=True)
 for p in src.rglob('*'):
  if p.is_file():
   q=dst/p.relative_to(src); h=hashlib.sha256(p.read_bytes()).hexdigest(); assert h==hashlib.sha256(q.read_bytes()).hexdigest(); files.append({'path':str(p.relative_to(root)),'sha256':h})
(out/'history-backup.json').write_text(json.dumps(files,indent=2))
url='https://api.scryfall.com/cards/named?exact=Dragon-Scarred%20Bear'
req=urllib.request.Request(url,headers={'User-Agent':'TheAperture/0.1 (Commander corpus conformance research)','Accept':'application/json'})
with urllib.request.urlopen(req,timeout=30) as r: raw=r.read()
c=json.loads(raw); (out/'source-raw.json').write_bytes(raw)
record={'retrieved_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'url':'https://api.scryfall.com/cards/'+c['id'],'source':c}
(out/'source-retrieval.json').write_text(json.dumps(record,indent=2))
print('BACKUP_FILES',len(files)); print('SOURCE',json.dumps({k:c.get(k) for k in ['id','oracle_id','set','collector_number','name','mana_cost','type_line','oracle_text','power','toughness']}))
u=json.loads((root/'evidence/issue17/usage-slice21.json').read_text()); print('GO_QUOTA',[(x['label'],x['amount'].get('remainingFraction')) for r in u['reports'] if r['provider']=='opencode-go' for x in r['limits']])
