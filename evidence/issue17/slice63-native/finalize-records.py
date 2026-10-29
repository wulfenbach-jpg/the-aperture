"""Publishable local records only after both exact-head workflows finish."""
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
HEAD = '150435b94f2e5282eaf6c5dbc782ffe1c148680e'
BASE = 'a4ac11ca9813d6ba8327c658e8f793673755dc27'
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == HEAD
ci_path = OUT / 'exact-head-ci-150435b.json'
ci = json.loads(ci_path.read_text())
assert ci['head'] == HEAD and ci['all_keyed'] and ci['both_workflows_complete']
assert ci['total'] >= 10 and ci['success'] == ci['total']
assert all(c['head_sha'] == HEAD and c['status'] == 'completed' and c['conclusion'] == 'success' for c in ci['check_runs'])
quality = [r for r in ci['workflow_runs'] if r['head_sha'] == HEAD and r['name'] == 'Rust quality' and r['event'] in ('push', 'pull_request')]
assert {r['event'] for r in quality} == {'push', 'pull_request'}
assert all(r['status'] == 'completed' and r['conclusion'] == 'success' for r in quality)
runs = {r['event']: {'id': r['id'], 'attempt': r['run_attempt'], 'updated_at': r['updated_at']} for r in quality}
selfcheck = json.loads((OUT/'prepublication-result.json').read_text())
offer = json.loads((OUT/'offer-audit.json').read_text())
assert selfcheck['head'] == HEAD and selfcheck['writer_focused_failed'] == 0
assert all(x['exit'] == 0 for x in selfcheck['commands'].values())
assert all(selfcheck['three_artifacts_byte_identical_to_writer'].values())
assert offer['all_actions_exactly_offered'] and offer['accepted_actions_checked'] == 954
assert offer['final_digest'] == selfcheck['journey']['digest']
manifest = json.loads((ROOT/'decks/starter-corpus/manifest.json').read_text())
starters = manifest['starters']
certified = [s['id'] for s in starters if s['status'] == 'certified']
gaps = [len(s['missing_cards']) for s in starters]
assert len(certified) == 2 and gaps == [55,0,51,43,0,52,52,53]
audit = (OUT/'corpus-audit.log').read_text()
assert 'AUDIT OK' in audit
source = [json.loads(p.read_text()) for p in sorted((ROOT/'evidence/issue17/slice63/sources').glob('*-source.json'))]
result = {
    'slice': 63, 'status': 'candidate for independent practical increment review',
    'native_coauthored': True, 'independent_review': False,
    'base': BASE, 'code_and_driver_head': HEAD,
    'product_commits': ['c58213a', '77865fb'], 'standalone_driver_format_commit': '150435b',
    'full_issue17_accepted': False,
    'corpus': {'catalog': 172, 'union': 459, 'supported': 158, 'missing': 301,
        'per_list': gaps, 'certified': len(certified), 'certified_lists': certified, 'desired': 8},
    'demanded_enabled': ['Goblin Warchief','Goblin King',"Dragonlord's Servant"],
    'out_of_list_positive_control': 'Volcanic Dragon',
    'gates': {'writer_focused_result_lines': selfcheck['writer_focused_result_lines'],
        'writer_focused_passed': selfcheck['writer_focused_passed'], 'writer_focused_failed': 0,
        'native_affected_passed': 17, 'native_affected_failed': 0, 'overlaps_writer': True,
        'new_test_delta': 11, 'new_tests': '10 slice63 + 1 printing conformance',
        'writer_fmt_clippy_locked_workspace_release': 'all exit 0, aggregate exit 0; repair-final-native/gates.log',
        'native_fmt_and_driver_fmt': 'exit 0', 'no_fresh_local_workspace_test_total_claimed': True},
    'native_journey': selfcheck['journey'], 'native_offer_and_payment_audit': offer,
    'three_artifacts_byte_identical_to_writer': selfcheck['three_artifacts_byte_identical_to_writer'],
    'separate_release_cli_replay': 'matches recorded, 2455 events, exit 0',
    'sources': source,
    'ci': {'head': HEAD, 'total': ci['total'], 'success': ci['success'], 'all_keyed': True,
        'both_workflows_complete': True, 'runs': runs,
        'evidence_pushed_only_after_both_complete': True,
        'receipt': 'evidence/issue17/slice63-native/exact-head-ci-150435b.json',
        'evidence_tip_claimed_no_own_ci': True},
    'children_inspected': ['t_26a7a4ff','t_3c2fe51a'],
    'children_disposition': 'Scheduled certified-input evaluation wait and unassigned soak backlog left gated; not increment-review children',
    'disclosures': [
        'Native is a coauthor/self-checker, NOT independent reviewer.',
        'Local initial c58213a broke existing Chieftain opponent/no-abilities control; native retained assertion RED and same writer repaired BEFORE any push.',
        'Volcanic Dragon is authorized out-of-list positive control, not desired-union progress.',
        'Krenko commander-tax test constructs an explicit taxed command-zone fixture; not a whole-game commander recast journey.',
        '954-step journey is a projected-offer 40-card package using Jasmine setup; not exact100-card Krenko certification, product AI, strength or GUI evidence.',
        'Native posthoc trace audit proves all 954 accepted actions were exactly offered and actual Dragon cast paid 4 mana with two Servants (printed 6).',
        'Initial standalone driver fmt check failed, formatting only was fixed in 150435b; subsequent native driver/workspace fmt pass.',
        'Initial pre-repair receipts retained; TDD.md records initial 8-test phase, not retroactive TDD for the later 2 tests.',
        'No new serialized state fields, AI/search/UI/import code or global config changed.',
        'Full original #17 remains unfinished: 2 of 8 certified, six lists planned, 301 union names missing.'
    ],
    'next_action': 'One independent practical delta review of a4ac11c..150435b plus evidence-only successor. Then SAME writer/history/card/branch/draftPR46 continues grouped sourced Krenko demanded mechanics; 3/8 only after actual exact-list certificate.',
    'no_merge_master_push_closure_unlock_newteam_globalconfig': True,
    'created_at': datetime.now(timezone.utc).isoformat()
}
(OUT/'RESULT.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
source_lines = '\n'.join(f"- **{s['name']}** ({s['set'].upper()} {s['collector_number']}, printing `{s['scryfall_id']}`, retrieved `{s['retrieved_at_utc']}`)." for s in source)
body = f'''# Slice63 — three demanded Krenko cards enabled; independent review requested

Same task/writer history/branch `aperture/issue-17`/draft PR46. Base `{BASE[:7]}`; **tested published code+driver head `{HEAD}`**. The successor publishing these records is evidence-only and claims no own CI. Original #17 remains **2 of8 certified**, not accepted. No merge/master push/closure/dependent unlock.

## Product and complete sourced text
- **Goblin Warchief**: Goblin spells cost one less generic; controlled Goblins have haste INCLUDING itself.
- **Goblin King**: OTHER Goblins across ALL controllers get +1/+1 and mountainwalk; a second King affects the first. Defending player's Mountain controls the restriction.
- **Dragonlord's Servant**: Dragon spells cost one less generic, not Goblin spells.
- **Volcanic Dragon**: sourced full Flying/haste companion, expressly authorized as an out-of-list real positive control. Does not increase demanded-union support.
- Shared subtype reduction/haste/anthem/landwalk machinery and Ruxa no-abilities recipient semantics. Both Krenko gap fields updated together: **46→43**. Fresh native audit **172catalog/459union=158supported+301missing**, per-list `{gaps}`, exactly2certified.

{source_lines}

Full provider responses and text: `fixtures/starter-corpus/slice63-sourced.json` and `slice63/sources/`. No card implemented from a paraphrase.

## Honest prepublication defects and authorship
Native is a coauthor, NOT the independent reviewer. Native affected check caught an actual regression in local initial c58213a: removing an outer controller restriction for global King's grant accidentally let an opposing Chieftain count as granting haste for Ruxa. Existing slice62 assertion failed6/1, retained in `slice63-native/affected-precheck.log`; same writer repaired the per-ability controller branch at77865fb. Assertion unchanged. No bad candidate was pushed.

Native also required a real castable Dragon, rather than accepting an unreachable positive cost path. New tests prove one/two Servants, colored-pip refusal, opposing-Servant control, resolved flying/haste, and Warchief reducing Krenko's generic commander tax. That last test is an explicit taxed command-zone fixture, not a full-game recast journey. The initial claim 'commander tax inapplicable' is superseded.

Standalone-driver fmt initially failed; native applied formatting-only150435b, with no product-source change. Final native workspace/driver fmt0. Initial receipts remain historical; TDD.md's8 refers to the initial phase, final slice63 suite10. No retroactive test-first claim for follow-up controls.

## Executed gates and actual journey
- Writer focused gates **139passed/0failed across12 result lines**: corelib14; slice62=7; slice63=10; Ruxa15; reduction11; printings22; catalog5; commander43; slice15=2; replay6; import3+1. fmt, locked workspace Clippy-Dwarnings and locked workspace release all0, aggregate0 (`slice63/repair-final-native/gates.log`). No fresh local whole-workspace test total claimed.
- New test delta **11** (10 new core tests +1 printing test), counted base→head, separate from suite totals. Native affected suites **17/0**, overlap writer, NOT additive.
- Native rebuilt/reran actual four-seat40-life exact40-card package journey, seed2323, terminalP3, **954actions/23turns/2455events**, digest`{selfcheck['journey']['digest']}`, snapshot`{selfcheck['journey']['snapshot_sha256']}`. Warchief6casts/King6/Servant3/Dragon1;14attacks/44combat-damage events. All3artifacts byte-identical to repaired writer's.
- Genuine non-sandbox save/load and **separate shipped release CLI replay**, matches recorded/2455events/exit0. Native trace audit additionally verifies **every954accepted action exactly belonged to that seat's projected offers** and actual Dragon payment was **4mana with2Servants vs printed6** (step864, P2); `slice63-native/offer-audit.json` + executable source.
- This is a projected-offer demonstrator with Jasmine setup, NOT100-card Krenko certification/productAI/strength/GUI evidence. No new canonical state fields or AI/UI/import code.

## Exact-head CI — waited before evidence push
`slice63-native/exact-head-ci-150435b.json`: **{ci['success']}/{ci['total']}completed success**, all keyed`{HEAD}`, both workflows completed: push**{runs['push']['id']}** attempt{runs['push']['attempt']}, PR merge-ref**{runs['pull_request']['id']}** attempt{runs['pull_request']['attempt']}. Both finished BEFORE the evidence publication. Evidence tip claims no ownCI.

## Remaining gate
Independent concise PRACTICAL DELTA review of a4ac11c..150435b plus evidence successor, reusing approved1–62. Focus global-vs-controlled grants/Ruxa, effective subtype and defending-land restriction, generic-only reductions/colored pips/Krenko tax, four complete printings, actual offered-action/payment/replay evidence. Then SAME writer/history/card/branch/PR46 continues demanded Krenko packages; **3/8 only on a genuine exact100-card next-list certificate**. Six lists remain planned,301union names missing. Inspected scheduled evaluation t_26a7a4ff and unassigned soak t_3c2fe51a; both left gated, neither an increment-review child. No completion/merge/masterpush/closure/unlock/newteam/globalconfig.
'''
(ROOT/'evidence/issue17/SLICE63-FINAL-HUMAN-CHECKPOINT.md').write_text(body,encoding='utf-8')
pr = f'''## Slice63 — three demanded Krenko cards + real Dragon control; original #17 remains2of8

Candidate for independent practical increment review, not self-approved or original-issue acceptance. Same draft PR/branch; approved slices1–62 preserved.

### Product
Goblin Warchief (DOM130): generic Goblin-spell reduction and self-inclusive controlled haste. Goblin King (10E207): other-Goblin +1/+1 and mountainwalk across ALL controllers. Dragonlord's Servant (TDC210): generic Dragon-spell reduction. Native-authorized out-of-list Volcanic Dragon (CMR207, full Flying/haste) makes that positive payment path genuinely reachable/tested. Full current source texts/printings/retrieval dates retained.

Both Krenko gap fields46→43. **172catalog/459union=158supported+301missing**, gaps`{gaps}`, **2of8certified**. No3of8 claim; Volcanic Dragon not demanded-union progress.

### Verification
Writer focused139/0 across12 result lines, fmt/lockedworkspaceClippy-Dwarnings/release0. Native affected17/0 overlaps. New tests11 (10core+1printing), not suite totals. Native actual4seat40life journey954actions/23turns/2455events, terminalP3/digest43bbcce4…8848; Warchief6casts/King6/Servant3/Dragon1,14attacks/44combatdamage. Non-sandbox save/load, separate release CLI replay match,3artifacts byte-identical. Native trace audit all954actions exactlyoffered; Dragon actually paid4mana with2Servants,printed6. Forty-card projected-offer Jasmine demonstrator, NOT exact100-card certification/productAI/strength/GUI evidence.

**{ci['success']}/{ci['total']}exact-head CI all keyed`{HEAD}`**, push{runs['push']['id']}/PR{runs['pull_request']['id']}, both complete BEFORE evidence push. Evidence successor claims no ownCI.

### Disclosures / next gate
Native coauthored, not independent reviewer. Native caught/fixed real prepublication Chieftain/Ruxa opposing-controller regression; existing assertion unchanged. Added real Dragon control and Krenko-tax fixture rather than accepting 'inapplicable' gaps. Standalone driver formatting fixed; historical receipts retained, no retroactiveTDD. No new serialized state fields/AI/UI/import code/globalconfig changes.

Read `evidence/issue17/SLICE63-FINAL-HUMAN-CHECKPOINT.md` and `evidence/issue17/slice63-native/RESULT.json`. Next ONE independent practical delta review, then same task/history/branch/PR continues grouped sourced Krenko demanded mechanics. Six lists planned. Original#17/fullCommander/arbitraryArchidekt/strength/v1 unaccepted. No completion/merge/masterpush/closure/dependentunlock/newteam.
'''
(OUT/'pr-body.md').write_text(pr,encoding='utf-8')
print('LOCAL_RECORDS_READY', HEAD, ci['success'], ci['total'], runs)
