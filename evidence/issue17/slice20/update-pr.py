from pathlib import Path
import json,subprocess
root=Path('evidence/issue17/slice20')
d=json.loads((root/'pr-preupdate.json').read_text())
b=d['body']
b=b.replace('- Catalog **135 real cards**.','- Catalog **137 real cards**.')
b=b.replace('**459 names = 123 supported + 336 missing** (per-list missing `55/0/51/48/30/52/52/53`)','**459 names = 125 supported + 334 missing** (per-list missing `55/0/51/48/28/52/52/53`)')
b=b.replace('Nearest list, Ayula: **30 missing names**.','Nearest list, Ayula: **28 missing names**.')
latest='''### Latest increment: slice20 — Vivien's Grizzly / private top-card look

Full sourced `{3}{G}` activated ability, not an approximation: privately look at the top card; optionally reveal/take a creature or planeswalker, otherwise put it on the bottom. Eligible cards offer take/bottom; ineligible cards still show their identity only to the owner and require a mandatory bottom acknowledgement. Empty library is not a draw/loss. Anchored `Pending::TopCardChoice` preserves mid-resolution save/load; spell-container validation permits multiple abilities sharing one source while retaining malformed-container refusal.

- Printing WAR182, immutable id `37f2571b-6756-462c-9e71-3121fa458160`, retrieved `2026-10-27T03:25:27.464708+00:00`, `fixtures/starter-corpus/slice20-sourced.json`; native live readback matches.
- Native125focused tests pass (nine new-card tests plus independent two-activation probe and affected existing suites), fmt/Clippy-Dwarnings/locked release CLI pass. Native rebuilt/reran the exact4seat/40life/40card package: terminalP3,2064actions/46turns/5281events, digest `72cd69fb1143...`, Vivien cast6/activated22/take5/voluntarybottom5/mandatorybottom16; separate releaseCLI replay and fresh-save roundtrip match.
- Evidence-only masked demonstrator, NOT product AI/strength, Jasmine Boreal injected eachseat; package is NOT another100-card certificate. Planeswalker branch code-inspected only (none inpool).
- Firstcut `cb7a18a` NOT accepted: native proved legal double-activation save refusal (retained RED101), and found controller's ineligible-card look lost. Writer repaired but omitted the engine hunk from its repair commit; native caught and committed that exact tested hunk before publication. Existing tests strengthened rather than waived.
- ONE true-original-history independent review: **APPROVED_INCREMENT at f4a02a2**, `evidence/issue17/slice20/REVIEW20.md`, no blocking findings. Final78dea97 differs only in evidence. Final exact-head CI is being collected; no inherited-green claim.
- Scope-only AI compatibility: slice20 `crates/ai/tests/common.rs` pending exhaustiveness arm; slice19 `crates/ai/src/search.rs:724` clone / `crates/ai/tests/common.rs:80` Default initialization; **no policy change**.
- Authoritative checkpoint `evidence/issue17/SLICE20-FINAL-HUMAN-CHECKPOINT.md`. Original eight-list acceptance remains UNFINISHED:1of8certified,334missing union names,Ayula28. No merge/closure/certified-downstream unlock. Three bulk lanes remain processed/candidate evidence only, zero release-enabled/certified cards from that fanout.

### Historical increment: slice18 — Ayula, Queen Among Bears'''
b=b.replace('### Latest increment: slice18 — Ayula, Queen Among Bears',latest)
b=b.replace('## Slice 19 — Spectral Bears and the pool\'s first attack trigger','## Historical slice19 — Spectral Bears and the pool\'s first attack trigger')
(root/'pr-body.md').write_text(b)
p=subprocess.run(['gh','pr','edit','46','--repo','wulfenbach-jpg/the-aperture','--body-file',str(root/'pr-body.md')],text=True,capture_output=True)
print('EDIT',p.returncode,p.stdout,p.stderr);p.check_returncode()
r=subprocess.check_output(['gh','pr','view','46','--repo','wulfenbach-jpg/the-aperture','--json','body,headRefOid,isDraft,state,mergeable,mergeStateStatus,url'],text=True)
(root/'pr-readback.json').write_text(r)
r=json.loads(r)
assert r['body'].replace('\r\n','\n').rstrip()==b.replace('\r\n','\n').rstrip()
assert r['headRefOid']=='78dea9784a0676ca30042a3eba3e634504972f1d'
assert r['isDraft'] and r['state']=='OPEN'
print('READBACK EXACT',r['headRefOid'],r['state'],r['isDraft'],r['mergeable'],r['mergeStateStatus'])
