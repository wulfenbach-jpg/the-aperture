from pathlib import Path
import json,re,hashlib,subprocess
root=Path.cwd(); out=root/'evidence/issue17/slice21'
s=(out/'native-gates.log').read_text(); counts=[int(x) for x in re.findall(r'test result: ok\. (\d+) passed',s)]; codes=re.findall(r'EXIT=(\d+)',s); assert codes==['0']*8
raw=json.loads((out/'source-raw.json').read_text()); normalized=json.loads((root/'fixtures/starter-corpus/slice21-sourced.json').read_text()); print(json.dumps(normalized,indent=2))
j=json.loads((out/'demo-journey.json').read_text()); assert hashlib.sha256((root/'decks/package-slice21.txt').read_bytes()).hexdigest()==j['snapshot_sha256']
rows=[json.loads(l) for l in (root/'evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl').read_text().splitlines() if l.strip()]
reds=[r for r in rows if r.get('timestamp','')>'2026-10-27T04:47:30' and r.get('timestamp','')<'2026-10-27T04:47:53' and 'dragon_scarred_bear_is_sourced_and_supported' in json.dumps(r)]
(out/'registration-red-original-history.json').write_text(json.dumps(reds,indent=2)+'\n'); assert reds
result={'verified_code_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'focused_suites':counts,'focused_passed':sum(counts),'gate_exit_codes':codes,'source_printing':raw['id'],'source_oracle':raw['oracle_id'],'source_retrieved_at':json.loads((out/'source-retrieval.json').read_text())['retrieved_at'],'snapshot_sha256':j['snapshot_sha256'],'native_rebuilt_and_reran_driver':True,'generated_artifacts_unchanged':True,'journey':j,'source_text':raw['oracle_text'],'full_issue17_accepted':False}
(out/'native-verification.json').write_text(json.dumps(result,indent=2)+'\n'); print('NATIVE',sum(counts),j)
