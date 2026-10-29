"""Check current gate receipts and unchanged journey bytes without rewriting history."""
import hashlib
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[3]
out = root / 'evidence/issue17/slice58'
log = (out / 'gates.log').read_text(encoding='utf-8')
results = [tuple(map(int, m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;', log)]
exits = dict(re.findall(r'^(\w+)_exit=(\d+)$', log, re.M))
assert len(results) == 7, results
assert set(exits) == {'fmt', 'focused', 'clippy', 'corpus', 'release'}, exits
assert all(v == '0' for v in exits.values()), exits
print('focused_suites=', len(results), 'passed=', sum(p for p, _ in results), 'failed=', sum(f for _, f in results))
print('gate_exits=', exits)
journey_log = (out / 'journey.log').read_text(encoding='utf-8')
assert 'driver_exit=0' in journey_log and 'separate_release_cli_exit=0' in journey_log
assert 'reproduces the save' in journey_log
for name in ['demo-save.json', 'demo-replay.json', 'demo-journey.json']:
    old = (root / 'evidence/issue17/slice57/native' / name).read_bytes()
    fresh = (out / 'native' / name).read_bytes()
    assert old == fresh, name
    print(name, 'byte_identical=true', 'sha256=' + hashlib.sha256(fresh).hexdigest())
journey = json.loads((out / 'native/demo-journey.json').read_text(encoding='utf-8'))
print(json.dumps(journey, indent=2))
