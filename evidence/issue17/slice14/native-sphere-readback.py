import datetime
import json
import pathlib
import subprocess
source=json.loads(pathlib.Path('evidence/issue17/slice13/candidate-source.json').read_text())
x=next(x for x in source['cards'] if x['name']=="Commander's Sphere")
live=json.loads(subprocess.check_output(['curl','-fsS','--max-time','30','-H','User-Agent: ApertureCorpus/0.1 (local development)','-H','Accept: application/json',x['uri']]))
fields=['name','mana_cost','type_line','oracle_text','power','toughness','colors','color_identity']
record={'retrieved_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'uri':x['uri'],'fields':{k:live.get(k) for k in fields},'matches':all(x.get(k)==live.get(k) for k in fields)}
pathlib.Path('evidence/issue17/slice14/native-sphere-readback.json').write_text(json.dumps(record,indent=2))
print(json.dumps(record,indent=2))
