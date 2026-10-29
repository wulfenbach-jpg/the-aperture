from pathlib import Path
import json,subprocess
r=Path('evidence/issue17/slice20')
def get():
 return json.loads(subprocess.check_output(['gh','pr','view','46','--repo','wulfenbach-jpg/the-aperture','--json','body,headRefOid,state,isDraft,mergeable,mergeStateStatus'],text=True))
d=get();b=d['body']
old='ONE true-original-history independent review: **APPROVED_INCREMENT at f4a02a2**, `evidence/issue17/slice20/REVIEW20.md`, no blocking findings. Final78dea97 differs only in evidence. Final exact-head CI is being collected; no inherited-green claim.'
new='Initial original-history approval at f4a02a2 is **SUPERSEDED**: native full-projection noninterference RED proved the public pending wording leaked private top-card eligibility (creature vs land), despite hiding card identities. Repair41d55ee filters TopCardChoice details to its owner; permanent unequal-hidden-state test compares entire opposing PlayerViews and masked digests. SAME original reviewer re-adjudicated the repaired delta: **APPROVED_INCREMENT at41d55ee**,14focused tests pass, unchanged release replay. Report `evidence/issue17/slice20/REVIEW20-PRIVACY.md`; no blocking finding remains. Native independently reran10changed tests/fmt/Clippy/locked release/replay. Finalcdfc32e differs from repaired approval only in3evidence paths. Its own exact-head CI is being collected, no inherited-green claim.'
assert old in b
b=b.replace(old,new)
(r/'pr-body-postprivacy.md').write_text(b)
p=subprocess.run(['gh','pr','edit','46','--repo','wulfenbach-jpg/the-aperture','--body-file',str(r/'pr-body-postprivacy.md')],capture_output=True,text=True);p.check_returncode()
z=get();(r/'pr-readback-postprivacy.json').write_text(json.dumps(z,indent=2)+'\n')
assert z['body'].replace('\r\n','\n').rstrip()==b.replace('\r\n','\n').rstrip()
assert z['headRefOid']=='cdfc32e1982758ce0ff5eb6d36f99769c39333ad' and z['state']=='OPEN' and z['isDraft']
print('EXACT_PR_READBACK',z['headRefOid'],z['state'],z['isDraft'],z['mergeable'],z['mergeStateStatus'])
