"""Update only PR46 body, then read back the exact external target."""
import json
import subprocess
from pathlib import Path
ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
body = (OUT/'pr-body.md').read_text(encoding='utf-8')
payload = OUT/'pr-body-payload.json'
payload.write_text(json.dumps({'body': body}, ensure_ascii=False), encoding='utf-8')
subprocess.run(['gh', 'api', '--method', 'PATCH', 'repos/wulfenbach-jpg/the-aperture/pulls/46', '--input', str(payload)], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
readback = json.loads(subprocess.check_output(['gh', 'api', 'repos/wulfenbach-jpg/the-aperture/pulls/46'], cwd=ROOT))
assert readback['body'].rstrip('\r\n') == body.rstrip('\r\n')
assert readback['state'] == 'open' and readback['draft']
assert readback['head']['ref'] == 'aperture/issue-17'
summary = {'body_exact_except_trailing_newlines': True, 'head': readback['head']['sha'], 'state': readback['state'], 'draft': readback['draft'], 'body': readback['body']}
(OUT/'pr-body-readback.json').write_text(json.dumps(summary,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print('PR_BODY_VERIFIED', summary['head'], summary['state'], summary['draft'])
