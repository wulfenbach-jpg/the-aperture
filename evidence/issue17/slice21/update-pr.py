from pathlib import Path
import json,subprocess
r=Path('evidence/issue17/slice21'); repo='wulfenbach-jpg/the-aperture'
def get(): return json.loads(subprocess.check_output(['gh','pr','view','46','--repo',repo,'--json','body,headRefOid,state,isDraft,mergeable,mergeStateStatus'],text=True))
d=get(); (r/'pr-before.json').write_text(json.dumps(d,indent=2)); b=d['body']; assert d['headRefOid']=='d23da60a3e90e09206d3da9dabefc1537a010fb1'
b=b.replace('Catalog **137 real cards**.','Catalog **138 real cards**.',1).replace('459 names = 125 supported + 334 missing','459 names = 126 supported + 333 missing',1).replace('55/0/51/48/28/52/52/53','55/0/51/48/27/52/52/53',1).replace('Ayula: **28 missing names**','Ayula: **27 missing names**',1)
b=b.replace('### Latest increment: slice20','### Historical increment: slice20',1)
b=b.replace('Ring temptation, regeneration, tutors','Ring temptation, tutors',1)
new='''### Latest increment: slice21 — Dragon-Scarred Bear / Formidable regeneration

Full sourced DTK183 printing `689a6390-894a-4d31-87c2-f9cec3b187a7`, retrieved `2026-10-27T04:44:50.249896+00:00`, fixture `fixtures/starter-corpus/slice21-sourced.json`. Printed {1}{G} repeatable/no-tap activation checks current controlled-creature power >=8 only at activation. Typed one-shot regeneration replaces destruction, removes damage, taps and removes from combat; multiple shields/cleanup expiry/accepted-state save reload. Exile/bounce/sacrifice/zero toughness are not destruction.

- Same original-history review **CHANGES_REQUESTED at d9d2c94**: ordinary-card ObjectId persists through bounce/recast, so a shield transferred. The initial test inserted a fresh object instead of recasting the same card. Real reviewer RED retained; canonical battlefield departure now invalidates shields. Real offered same-card recast then Shock dies. **APPROVED_INCREMENT repaired f59d29b**, no blocking findings, failed verdict preserved (`slice21/REVIEW21.md`, `REVIEW21-REPAIR.md`). Dedicated queued-source-left test is not claimed; code-inspected only.
- Native own68focused tests/gates pass; repaired8new-card+1cleanup tests pass; fmt/Clippy-Dwarnings/locked release pass. Native rebuilt/reran exact4seat40life40card package: terminalP3,1906actions/45turns/4909events,digest0d20c7597183; actual25activations/25shield creations/1regeneration,Bear cast7; separate release replay+fresh save match. Prior slice20 replay still72cd69fb1143.
- Evidence-only offered-action demonstrator with public battlefield/shield instrumentation, NOT productAI/strength; JasmineBoreal injected4seats. NOT another100card certificate. AI mechanical field initialization only (`search.rs:727`, `tests/common.rs:83`), no policy change.
- Native fixed wrong Fuse citation and preferred-branch activation undercount before approval. Initial registration RED precedes source; later behavior tests mostly GREEN strengthening, not blanket strictTDD.
- Final d23da60 differs from approved f59d29b in exactly5evidence files, no product source. Its own exact-head CI is being collected, no inherited-green claim. Checkpoint `evidence/issue17/SLICE21-FINAL-HUMAN-CHECKPOINT.md`.
- CURRENT:138catalog/459union=126supported+333missing,Ayula27,**still1of8certified**. Full original acceptance open; no merge/closure/certified-downstream unlock. Desired snapshots unchanged; bulk candidates unintegrated,0release-enabled/0certified.

'''
b=b.replace('### Historical increment: slice20',new+'### Historical increment: slice20',1)
(r/'pr-body.md').write_text(b)
subprocess.run(['gh','pr','edit','46','--repo',repo,'--body-file',str(r/'pr-body.md')],check=True)
z=get(); (r/'pr-readback.json').write_text(json.dumps(z,indent=2)+'\n'); assert z['body'].replace('\r\n','\n').rstrip()==b.replace('\r\n','\n').rstrip(); assert z['state']=='OPEN' and z['isDraft'] and z['headRefOid']=='d23da60a3e90e09206d3da9dabefc1537a010fb1'
print('PR_EXACT_READBACK',z['headRefOid'],z['state'],z['isDraft'],z['mergeable'],z['mergeStateStatus'])
