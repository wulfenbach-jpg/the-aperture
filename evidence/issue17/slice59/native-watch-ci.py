"""Wait for checks AND both workflows before publishing another head."""
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
HEAD = subprocess.check_output(['git', 'rev-parse', sys.argv[1]], cwd=ROOT, text=True).strip()

def api(endpoint):
    result = subprocess.run(['gh', 'api', endpoint], cwd=ROOT, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr)
    return json.loads(result.stdout)

for _ in range(70):
    try:
        checks = api(f'repos/wulfenbach-jpg/the-aperture/commits/{HEAD}/check-runs?per_page=100')['check_runs']
        runs = api(f'repos/wulfenbach-jpg/the-aperture/actions/runs?head_sha={HEAD}&per_page=100')['workflow_runs']
        keyed = bool(checks) and all(c['head_sha'] == HEAD for c in checks)
        succeeded = sum(c['status'] == 'completed' and c['conclusion'] == 'success' for c in checks)
        quality = [r for r in runs if r['head_sha'] == HEAD and r['event'] in ('push', 'pull_request') and r['name'] == 'Rust quality']
        events = {r['event'] for r in quality}
        workflows_complete = events == {'push', 'pull_request'} and all(r['status'] == 'completed' and r['conclusion'] == 'success' for r in quality)
        receipt = {'head': HEAD, 'observed_at': datetime.now(timezone.utc).isoformat(), 'all_keyed': keyed, 'success': succeeded, 'total': len(checks), 'both_workflows_complete': workflows_complete, 'check_runs': checks, 'workflow_runs': runs}
        (OUT / f'exact-head-ci-{HEAD[:7]}.json').write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
        print(f"{receipt['observed_at']} checks={succeeded}/{len(checks)} keyed={keyed} both_workflows={workflows_complete} workflow_names={[(r['name'],r['event'],r['status'],r['conclusion']) for r in runs]}", flush=True)
        if len(checks) >= 10 and keyed and succeeded == len(checks) and workflows_complete:
            print('EXACT_HEAD_CI_AND_BOTH_WORKFLOWS_SUCCESS', flush=True)
            sys.exit(0)
        if any(c['status'] == 'completed' and c['conclusion'] not in ('success', 'neutral', 'skipped') for c in checks):
            sys.exit('CI_FAILURE_OR_CANCELLATION')
    except Exception as exc:
        print(f'query_error={exc}', flush=True)
    time.sleep(25)
print('CI_WAIT_TIMEOUT', flush=True)
sys.exit(2)
