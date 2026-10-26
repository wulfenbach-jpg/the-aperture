from pathlib import Path
import json
import os
home = Path(os.environ['HERMES_HOME'])
attached = home / 'kanban/boards/the-aperture/attachments/t_67778feb'
local = Path('evidence/issue17')
assert (attached / 'HUMAN-CHECKPOINT.md').read_bytes() == (local / 'HUMAN-CHECKPOINT.md').read_bytes()
assert (attached / 'jasmine-vanilla-stompy.txt').read_bytes() == Path('decks/starter-corpus/jasmine-vanilla-stompy.txt').read_bytes()
checks = json.loads((attached / 'PR46-final-ci-check-runs.json').read_text())
assert checks['total_count'] == len(checks['check_runs']) == 10
assert all(check['head_sha'] == '86db7d316edd6200f7f598849018e428d11b69eb' and check['status'] == 'completed' and check['conclusion'] == 'success' for check in checks['check_runs'])
review = json.loads((attached / 'PR46-review-approval-receipt.json').read_text())
assert review['id'] == 6395664107
assert 'approve_increment' in review['body'] and 'full_issue17_accepted: false' in review['body']
phase = json.loads((home / 'kanban/boards/the-aperture/continuity/active-phase.json').read_text())
jobs = json.loads((home / 'cron/jobs.json').read_text())['jobs']
supervisor = next(job for job in jobs if job['id'] == 'a60a00413961')
print(json.dumps({
    'checkpoint_attachment_byte_match': True,
    'exact_deck_attachment_byte_match': True,
    'attached_exact_head_ci_success': 10,
    'review_receipt_verified': True,
    'reporter_current_phase': phase.get('phase'),
    'reporter_current_issue': phase.get('issue'),
    'existing_supervisor_enabled': supervisor.get('enabled'),
    'existing_supervisor_state': supervisor.get('state'),
    'existing_supervisor_interval': supervisor.get('schedule'),
}, indent=2))
