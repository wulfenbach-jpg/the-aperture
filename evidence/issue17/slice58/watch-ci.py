"""Bounded exact-head CI wait. Never publish another head while this runs."""
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

root = Path(__file__).resolve().parents[3]
head = subprocess.check_output(['git', 'rev-parse', sys.argv[1]], cwd=root, text=True).strip()
out = root / 'evidence/issue17/slice58'
for attempt in range(90):
    result = subprocess.run(['gh', 'api', f'repos/wulfenbach-jpg/the-aperture/commits/{head}/check-runs', '-f', 'per_page=100', '--method', 'GET'], cwd=root, text=True, capture_output=True)
    if result.returncode:
        print(f'query_error={result.returncode}: {result.stderr}', flush=True)
        time.sleep(30)
        continue
    data = json.loads(result.stdout)
    runs = data['check_runs']
    keyed = all(run['head_sha'] == head for run in runs)
    success = sum(run['status'] == 'completed' and run['conclusion'] == 'success' for run in runs)
    print(f'{datetime.now(timezone.utc).isoformat()} head={head} runs={len(runs)} success={success} all_keyed={keyed}', flush=True)
    receipt = {'head': head, 'observed_at': datetime.now(timezone.utc).isoformat(), 'all_keyed': keyed, 'success': success, 'total': len(runs), 'check_runs': runs}
    (out / f'exact-head-ci-{head[:7]}.json').write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
    if len(runs) >= 10 and keyed and success == len(runs):
        print('EXACT_HEAD_CI_SUCCESS', flush=True)
        sys.exit(0)
    if runs and any(run['status'] == 'completed' and run['conclusion'] not in ('success', 'skipped', 'neutral') for run in runs):
        print('CI_FAILURE_OR_CANCELLATION', flush=True)
        sys.exit(1)
    time.sleep(30)
print('CI_WAIT_TIMEOUT', flush=True)
sys.exit(2)
