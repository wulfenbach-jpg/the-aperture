from pathlib import Path
import json, subprocess
p=Path('evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl')
rows=[json.loads(l) for l in p.read_text().splitlines()]
for r in rows[-45:]:
    m=r.get('message',{})
    if m.get('role')=='assistant':
        for c in m.get('content',[]):
            if c.get('type')=='toolCall': print(r.get('timestamp'),c['name'],str(c.get('arguments'))[:400])
    if m.get('role')=='toolResult' and m.get('isError'): print('ERROR',str(m)[:1200])
print(subprocess.run(['git','diff','--stat'],capture_output=True,text=True).stdout)
print(Path('evidence/issue17/slice20/worker.log').read_text()[-3500:])
