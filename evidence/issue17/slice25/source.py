from pathlib import Path
import json,urllib.request,datetime,hashlib
out=Path(__file__).parent; out.mkdir(parents=True,exist_ok=True)
c=next(c for c in json.loads(Path('evidence/issue17/slice17/remaining-ayula-sources.json').read_text())['cards'] if c['name']=='Exuberant Wolfbear')
req=urllib.request.Request(c['printing_url'],headers={'User-Agent':'TheAperture/issue17 (local implementation sourcing)','Accept':'application/json'})
raw=urllib.request.urlopen(req,timeout=40).read(); (out/'source-raw.json').write_bytes(raw)
(out/'source-retrieval.json').write_text(json.dumps({'url':c['printing_url'],'retrieved_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'sha256':hashlib.sha256(raw).hexdigest()},indent=2))
d=json.loads(raw); print(json.dumps({k:d.get(k) for k in ['id','oracle_id','name','set','collector_number','mana_cost','type_line','oracle_text','power','toughness']},indent=2))
