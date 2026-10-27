from pathlib import Path
import json,subprocess
out=Path(__file__).parent; repo='wulfenbach-jpg/the-aperture'
def gh(*args): return subprocess.check_output(['gh',*args],text=True)
r=json.loads(gh('pr','view','46','--repo',repo,'--json','body,headRefOid,state,isDraft'))
assert r['headRefOid']=='ea106932d649da6876004fdf5c1a2ce6e441d19c'
(out/'pr-before.json').write_text(json.dumps(r,indent=2))
body=r['body'].replace('\r\n','\n').replace('\r','').replace('Catalog **141 real cards**.','Catalog **142 real cards**.').replace('459 names = 129 supported + 330 missing','459 names = 130 supported + 329 missing').replace('55/0/51/48/24/52/52/53','55/0/51/48/23/52/52/53').replace('Nearest list, Ayula: **24 missing names**.','Nearest list, Ayula: **23 missing names**.')
section='''### Latest increment: slice25 — Exuberant Wolfbear (independently approved increment)

- Exact IKO151 printing `8c8ff0da-fbe3-4c95-bc39-c975c55b6aea`, oracle `40f5f8b9-5c2c-41d4-8b83-55a2e51ced64`, retrieved `2026-10-27T09:19:21Z`; exact response and stamp retained in `slice25/source-raw.json` / `source-retrieval.json`, printing fixture `fixtures/starter-corpus/slice25-sourced.json`. Full optional attack trigger: target Human you control has its base power/toughness set to the Wolfbear's effective P/T until end of turn.
- Reusable layer7b `SetBasePowerToughness`, applied in timestamp order BEFORE counters and layer7c modifiers. Source is read at resolution; a first-departure `source_pt_lki` snapshot preserves actual last-known effective P/T when it leaves, without later same-ID incarnations overwriting it. New serialized fields skip None; no new pending kind or AI policy change.
- Native added three GREEN-strengthening controls after writer code823680d: post-trigger real Giant Growth responses distinguish current-vs-trigger-time P/T; source7/7 plus target's own counter+pump yields11/11 and cleanup restores3/2; pumped source then real Unsummon uses7/7LKI; actual multiple-Human target offers reject nonHuman/opponent/wrongseat submissions without state/digest changes. Earlier writer test name overclaimed a forged-target assertion; native renamed it and supplied the actual test. Initial missing-card RED preceded registration; no blanket strictTDD claim.
- Native100focusedtests pass (7slice25,10slice24 plus affected suites), fmt/workspaceClippy-Dwarnings/releaseCLI all exit0. Native rebuilt/reran exact4seat40life40cardpackage: terminalP3,1760actions/41turns/4471events,8Wolfbearcasts/9attacktriggers/9base-PT-setevents; digest `4d6ea58ee735c74a89e142eca90656dbf22faeaf49c2de0fb33c0ec9952c5d52`, separateCLIreplay matches, freshsave roundtrips, exact snapshot SHA256 `1bd229439656be174278cd958eb7083ec495c43be3eefa8a17d4ee6fff881a0d`. Evidence-only masked offered-action demonstrator, NOT productAI/strength or a desired100cardcertificate. Slice24 replay still a41bbfd2.
- SAME original-history independent reviewer **APPROVED_INCREMENT at ea10693**, independently ran7focusedtests and release replay, inspected actual delta and native evidence. Report `evidence/issue17/slice25/REVIEW25-ORIGINAL-HISTORY.md`. Native authored test/docs-only follow-up ea10693; writer823680d contains engine changes. No blocking finding. Exact-head CI still pending at this update, not inherited green.
- Current142catalog/459union=130supported+329missing; per-list55/0/51/48/23/52/52/53. Only1of8desired100cardstarterscertified, Ayula23missing. Full#17UNFINISHED. No reachable mid-optional save/reload, dedicated target departure/recast or source-recast-overrides-LKI test is claimed; those paths are code-inspected. No full workspace rerun in this increment; exact-headCI owns that gate.

### Historical increment: slice24 — Surrak and Goreclaw (approved3b06249, 10/10exact-headCI PASS)

The publication-era pending statements below are superseded by the exact3b06249 original-history approval and10/10CI receipt in `evidence/issue17/SLICE24-FINAL-HUMAN-CHECKPOINT.md`.
'''
needle='### Latest increment: slice24 — Surrak and Goreclaw (candidate under independent review)'
assert needle in body
body=body.replace(needle,section)
(out/'pr-body.md').write_text(body)
subprocess.run(['gh','pr','edit','46','--repo',repo,'--body-file',str(out/'pr-body.md')],check=True)
after=json.loads(gh('pr','view','46','--repo',repo,'--json','body,headRefOid,state,isDraft'))
assert after['body'].replace('\r\n','\n')==body.replace('\r\n','\n')
(out/'pr-readback.json').write_text(json.dumps(after,indent=2)); print({k:v for k,v in after.items() if k!='body'})
