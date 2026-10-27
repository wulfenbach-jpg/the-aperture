from pathlib import Path
import json, urllib.request, hashlib, datetime, re, subprocess
root=Path.cwd()
fixture=json.loads(Path('fixtures/starter-corpus/slice20-sourced.json').read_text())['cards'][0]
req=urllib.request.Request(fixture['printing_url'],headers={'User-Agent':'TheAperture-Issue17-NativeVerify/1.0','Accept':'application/json'})
with urllib.request.urlopen(req,timeout=25) as r: live=json.load(r)
checks={k:live[k]==fixture[k] for k in ['name','oracle_id','set','collector_number','layout','mana_cost','type_line','power','toughness','oracle_text','color_identity','keywords']}
checks['printing_id']=live['id']==fixture['scryfall_id']
checks['commander_legality']=live['legalities']['commander']==fixture['legalities_commander']
assert all(checks.values()),checks
journey=json.loads(Path('evidence/issue17/slice20/demo-journey.json').read_text())
hash_=hashlib.sha256(Path(journey['deck']).read_bytes()).hexdigest()
assert hash_==journey['snapshot_sha256']
manifest=json.loads(Path('decks/starter-corpus/manifest.json').read_text())
assert len(manifest['starters'])==8
assert sum(s['status']=='certified' for s in manifest['starters'])==1
changed=subprocess.check_output(['git','diff','--name-only','7dc933a..HEAD','--','decks/starter-corpus'],text=True).splitlines()
assert changed==['decks/starter-corpus/manifest.json'],changed
s=Path('evidence/issue17/slice20/native-focused.log').read_text()
counts=[int(x) for x in re.findall(r'test result: ok\. (\d+) passed',s)]
out={'head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'verified_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_checks':checks,'printing_url':fixture['printing_url'],'snapshot_sha256':hash_,'desired_snapshot_diff':changed,'certified':1,'desired':8,'native_focused_tests':sum(counts),'native_focused_suites':len(counts),'journey':journey,'full_issue17_accepted':False}
print(json.dumps(out,indent=2))
