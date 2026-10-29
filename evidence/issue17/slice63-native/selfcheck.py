"""Native coauthor self-check; not independent review."""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
WRITER = ROOT / 'evidence/issue17/slice63/repair-final-native'
FILES = ['demo-journey.json', 'demo-replay.json', 'demo-save.json']
before = {name: (WRITER / name).read_bytes() for name in FILES}
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
results = {}
def run(name, args):
    result = subprocess.run(args, cwd=ROOT, capture_output=True, timeout=600)
    (OUT / (name + '.log')).write_bytes(result.stdout + result.stderr + f'\n{name}_exit={result.returncode}\n'.encode())
    print(name, 'exit', result.returncode, flush=True)
    results[name] = {'command': args, 'exit': result.returncode}
    if result.returncode:
        print((result.stdout + result.stderr).decode(errors='replace')[-6000:])
        sys.exit(result.returncode)
run('corpus-audit', ['python', 'evidence/issue17/slice63/corpus-audit.py'])
run('journey', ['cargo', 'run', '--release', '--locked', '--manifest-path', 'evidence/issue17/slice63-journey-driver/Cargo.toml'])
identical = {}
for name in FILES:
    data = (WRITER / name).read_bytes()
    identical[name] = data == before[name]
    (OUT / name).write_bytes(data)
assert all(identical.values()), identical
run('separate-cli-replay', ['target/release/aperture-cli.exe', 'replay', '--file', str(OUT / 'demo-replay.json')])
run('fmt', ['cargo', 'fmt', '--all', '--check'])
run('driver-fmt', ['cargo', 'fmt', '--manifest-path', 'evidence/issue17/slice63-journey-driver/Cargo.toml', '--check'])
focused = (ROOT/'evidence/issue17/slice63/repair-final-native/gates.log').read_text()
lines = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', focused)
report = json.loads((OUT/'demo-journey.json').read_text())
result = {'self_check': True, 'independent_review': False, 'head': head,
    'commands': results, 'three_artifacts_byte_identical_to_writer': identical,
    'writer_focused_result_lines': len(lines), 'writer_focused_passed': sum(int(x[0]) for x in lines),
    'writer_focused_failed': sum(int(x[1]) for x in lines),
    'native_affected': 'issue17_slice62 7/0 + issue17_slice63 10/0; overlap writer, not additive',
    'journey': report, 'full_issue17_accepted': False, 'certified': 2, 'desired': 8}
(OUT/'prepublication-result.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
