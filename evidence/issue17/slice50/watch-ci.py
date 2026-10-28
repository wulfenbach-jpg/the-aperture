import datetime
import json
from pathlib import Path
import subprocess
import sys
import time

head = sys.argv[1]
out = Path(__file__).resolve().parent
for attempt in range(48):
    raw = subprocess.check_output([
        'gh', 'api', f'repos/wulfenbach-jpg/the-aperture/commits/{head}/check-runs?per_page=100'
    ], text=True, encoding='utf-8')
    data = json.loads(raw)
    runs = data['check_runs']
    assert data['total_count'] == len(runs), 'pagination needed'
    assert all(r['head_sha'] == head for r in runs), 'wrong head'
    brief = [{k: r[k] for k in ('id', 'name', 'head_sha', 'status', 'conclusion', 'html_url')} for r in runs]
    result = {'head': head, 'retrieved_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
              'total': len(runs), 'checks': brief,
              'all_success': len(runs) >= 10 and all(r['status'] == 'completed' and r['conclusion'] == 'success' for r in runs)}
    (out / f'exact-head-ci-{head[:7]}.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(attempt, len(runs), [(r['name'], r['status'], r['conclusion']) for r in runs], flush=True)
    if result['all_success']:
        sys.exit(0)
    if any(r['status'] == 'completed' and r['conclusion'] not in ('success', 'skipped') for r in runs):
        sys.exit(1)
    time.sleep(25)
sys.exit(2)
