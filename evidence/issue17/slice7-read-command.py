import json
from pathlib import Path
rows = [json.loads(s) for s in next(Path('evidence/issue17/sessions').glob('*.jsonl')).read_text(encoding='utf-8').splitlines()]
for row in rows:
    m=row.get('message',{})
    if m.get('role')!='assistant':
        continue
    for c in m.get('content',[]):
        if c.get('type')=='toolCall' and 'slice7-journey' in str(c):
            print(json.dumps(c,ensure_ascii=False))
