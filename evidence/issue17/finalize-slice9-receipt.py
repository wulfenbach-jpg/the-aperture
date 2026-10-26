import json
import subprocess
from pathlib import Path
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[2]
REPO = 'wulfenbach-jpg/the-aperture'
OUT = ROOT / 'evidence/issue17'

def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True, encoding='utf-8')

def gh(*args):
    return json.loads(command('gh', *args))

head = command('git', 'rev-parse', 'HEAD').strip()
pr = gh('pr', 'view', '46', '--repo', REPO, '--json', 'headRefOid,state,isDraft,mergeable,mergeStateStatus')
remote = command('git', 'ls-remote', 'origin', 'refs/heads/aperture/issue-17').split()[0]
review = json.loads((OUT / 'slice9-review-2.json').read_text())
assert head == remote == pr['headRefOid'] == review['reviewed_head']
assert pr['state'] == 'OPEN' and pr['isDraft'] and review['review_outcome'] == 'APPROVED_INCREMENT'
checks = gh('api', f'repos/{REPO}/commits/{head}/check-runs')
(OUT / 'slice9-exact-head-ci.json').write_text(json.dumps(checks, indent=2) + '\n')
rows = checks['check_runs']
assert len(rows) >= 5 and all(r['head_sha'] == head for r in rows)
assert all(r['status'] == 'completed' and r['conclusion'] == 'success' for r in rows), [(r['name'], r['status'], r['conclusion']) for r in rows]
required = gh('pr', 'checks', '46', '--repo', REPO, '--required', '--json', 'name,state,bucket')
assert required and all(r['bucket'] == 'pass' for r in required), required
(OUT / 'slice9-required-checks.json').write_text(json.dumps(required, indent=2) + '\n')
receipt = {'timestamp': datetime.now(timezone.utc).isoformat(), 'head': head, 'remote_head': remote, 'pr': pr, 'review_outcome': review['review_outcome'], 'ci_success': len(rows), 'ci_failed': 0, 'required_checks_pass': True, 'catalog': 114, 'union': 459, 'supported': 102, 'missing': 357, 'implemented_lists': 1, 'certified_lists': 0, 'full_issue17_accepted': False, 'no_merge_or_closure': True}
(OUT / 'slice9-publication-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
body = f'''## Issue #17 — partial starter corpus, slices 8–9

**Draft; the original eight-list certification gate remains open.** Catalog114; exact eight-list union459 =102 catalog-supported names +357 missing. One exact100-card starter implemented; zero certified. Catalog membership is not whole-list playability. Arbitrary Archidekt compatibility remains a non-goal until proven.

### Newly implemented on this continuation
Soul's Attendant, Razorclaw Bear, Volley Veteran, Dramatic Reversal, Goblin War Strike, Brightstone Ritual, Mana Geyser and Rite of Flame. Live printing IDs, fields and UTC retrieval dates are retained in `fixtures/starter-corpus/slice8-sourced.json` and `slice9-sourced.json`. All eight desired deck contents are unchanged.

Mana pool/event quantities widened to u32, exact payments above255, bounded rendering for large quantities, and preserved old numeric replay behavior. u32MAX saturation remains an explicit simulation limit. Goblin War Strike's planeswalker target is not usable because no planeswalker permanents exist in the pool.

### Verification at the current candidate
- Exact local/remote/PR/review head `{head}`; same-history independent practical review2 **APPROVED_INCREMENT**, zero findings.
- {len(rows)}/{len(rows)} exact-head checks completed SUCCESS; configured required checks pass. This is not merge authorization.
- Slice8 affected review suites101 tests; final9-test slice8 suite after the unblocked control. Slice9 affected review suites47 tests; final native slice8+9 suites9+6. These are separate runs, not a unique combined test count. Round2 review reused-evidence text accidentally repeats101 for slice9; its actual focused log totals47.
- fmt, core Clippy warnings-denied and locked release CLI build independently reproduced. Empty quiet-command logs are not presented as evidence.
- Real exact four-seat40-life package seed801: terminal2377actions/68turns,6087events, separate replay matches `ffbeaee0db38...`. Razorclaw/Volley trigger events occur; other new effects are proven by focused tests, not merely card-name occurrences.
- Real exact four-seat40-life Krenko package seed901: terminal1339actions/32turns,4379events, separate replay matches `5d20bb4a35e2...`; old slice8 replay still matches. The heuristic casts none of the three new rituals, so focused conformance tests, not this trace, prove their mana behavior.
- Both journeys use explicit40-card PACKAGE snapshots, not substitutes for desired100-card starters.

### Preserved correctness and limitations
Prior slice7 exact-once simultaneous death dispatch and token destination-before-SBA integration remain approved; not re-audited here. Friendly Teddy and Aetherize remain unsupported due to multi-player draw/APNAP and attacker-list commander replacement continuations. Brightstone's sourced slice9 implementation supersedes its historical slice8 exclusion. Broader legend/equipment/auras, tribal commander, tutor/reanimation and hundreds of desired cards remain missing. Analyzer gameplay refusal is unchanged. Slice8's absent-catalogRED/implementation-first history and slice9's genuine pre-engineRED are recorded honestly.

No merge, master push, force push, issue closure or certified #26/#32 unlock. Continue on this SAME card/PR and preserved writer/reviewer histories. Full #17 remains unfinished.
'''
body_path = OUT / 'SLICE9-PR-BODY.md'
body_path.write_text(body)
subprocess.run(['gh', 'pr', 'edit', '46', '--repo', REPO, '--body-file', str(body_path)], cwd=ROOT, check=True)
readback = gh('pr', 'view', '46', '--repo', REPO, '--json', 'headRefOid,state,isDraft,mergeable,body')
assert readback['headRefOid'] == head and readback['body'].replace('\r\n', '\n') == body.replace('\r\n', '\n')
(OUT / 'slice9-pr-readback.json').write_text(json.dumps(readback, indent=2) + '\n')
print(json.dumps(receipt, indent=2))
