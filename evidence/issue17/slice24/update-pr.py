import json,subprocess
from pathlib import Path
out=Path(__file__).parent
old=json.loads((out/'pr-before.json').read_text())['body']
body=old.replace('Catalog **140 real cards**.','Catalog **141 real cards**.').replace('459 names = 128 supported + 331 missing','459 names = 129 supported + 330 missing').replace('55/0/51/48/25/52/52/53','55/0/51/48/24/52/52/53').replace('Nearest list, Ayula: **25 missing names**.','Nearest list, Ayula: **24 missing names**.')
latest='''### Latest increment: slice24 — Surrak and Goreclaw (candidate under independent review)

- Exact MOM337 printing `9c10934d-9016-43c4-a7ab-56cc7d8f671f`, retrieved `2026-10-27T08:44:35.433520+00:00`, fixture `fixtures/starter-corpus/slice24-sourced.json`. Full printed trample, static trample to other controlled creatures, and another own nontoken creature entry grants that entered permanent a +1/+1 counter and haste until end of turn. Non-target captured identity, not a target prompt.
- Reusable `OtherControlledCreaturesHaveTrample`, `ContinuousKind::GrantsHaste`, and serde-skipped `affected` identity on trigger/stack entries. Haste affects attack legality, tap-cost checks and the actual offered creature-mana-source list.
- First cut `c457e49` was NOT accepted: native actual REDs proved a hasty Werebear missing from legal offers, and departure silently deleting the non-target trigger. Repair `c2d9b35` fixes offers and invalidates the affected binding while retaining the mandatory trigger; uses captured card identity for source-left resolution. Failed evidence retained, assertions not weakened.
- Native focused checks: 93 passed (including slice24 10); writer's fmt, workspace warnings-denied Clippy and release CLI gates exit0. Native independently rebuilt/reran real four-seat/40-life/40-card exact package: terminal P3, 1602 actions, 4127 replay events, digest `a41bbfd2760034f791a3616bd5d6faa0fd79ad8e9cee300eb69cc38c5a1ec631`; 6 actual Surrak casts, 11 counter and haste events; separate CLI replay matches and fresh save roundtrips. Evidence-only masked demonstrator with Jasmine Boreal in four command zones, NOT product AI, strength evidence or a desired 100-card starter certificate.
- Native per-list audit: 141 catalog /459 union =129 supported+330 missing; Ayula24, still1/8 certified. Desired snapshots and certificate statuses unchanged. Midtrigger save test is sandbox/direct fixture, not a reachable action-history claim. Full original #17 remains unfinished. Candidate `3b06249` published; independent original-history review and exact-head CI are pending, not claimed green.

### Historical increment: slice23 — Goreclaw, Terror of Qal Sisma (cost reduction + granted trample)'''
body=body.replace('### Latest increment: slice23 — Goreclaw, Terror of Qal Sisma (cost reduction + granted trample)',latest)
(out/'pr-body.md').write_text(body,encoding='utf-8')
subprocess.run(['gh','pr','edit','46','--repo','wulfenbach-jpg/the-aperture','--body-file',str(out/'pr-body.md')],check=True)
read=json.loads(subprocess.check_output(['gh','pr','view','46','--repo','wulfenbach-jpg/the-aperture','--json','body,headRefOid,state,isDraft'],text=True))
assert read['body'].replace('\r\n','\n')==body.replace('\r\n','\n')
(out/'pr-readback.json').write_text(json.dumps(read,indent=2))
print({k:v for k,v in read.items() if k!='body'})
