import json, zipfile, shutil, hashlib, subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
r=root/'evidence/issue17'
for name in ['sessions','review-sessions']:
    shutil.copytree(r/name,r/'slice7-final-backup'/name,dirs_exist_ok=True)
files=[
 'SLICE7-HUMAN-CHECKPOINT.md','SLICE7-IMPLEMENT-HANDOFF.md','SLICE7-IMPLEMENT-1.md','SLICE7-REPAIR-2.md','SLICE7-PR-BODY.md',
 'slice7-implement-1.log','slice7-implement-1b.log','slice7-repair-2.log','slice7-token-ordering-red.log','slice7-trace-red.log',
 'slice7-native-tests.log','slice7-native-clippy.log','slice7-native-build.log','slice7-native-probe-green.log',
 'slice7-native-journey.json','slice7-native-journey.replay.json','slice7-native-journey.log','slice7-native-journey-replay.log',
 'slice7-native-audit.py','slice7-native-corpus.json','slice7-exact-head-ci.json','slice7-required-checks.json','slice7-pr-readback.json',
 'review224-probe/src/lib.rs','review224-probe/Cargo.toml','review224-probe/Cargo.lock','review224-probe/output.log',
]
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()=='7e1eb5f1b8218e24d7a03377c3cb3c5bb0981a4f'
assert not subprocess.check_output(['git','status','--porcelain','--untracked-files=no'],cwd=root,text=True).strip()
missing=[str(f) for f in files if not (r/f).exists()]
assert not missing, missing
z=r/'slice7-review-packet.zip'
with zipfile.ZipFile(z,'w',zipfile.ZIP_DEFLATED) as out:
    for f in files:
        out.write(r/f,f)
with zipfile.ZipFile(z) as check:
    assert check.testzip() is None
    assert len(check.namelist())==len(files)
print(json.dumps({'archive':str(z),'files':len(files),'bytes':z.stat().st_size,'sha256':hashlib.sha256(z.read_bytes()).hexdigest()},indent=2))
