from pathlib import Path
import json,sys
p=Path('evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl')
rows=[json.loads(x) for x in p.read_text(encoding='utf-8').splitlines() if x.strip()]
for r in rows[-int(sys.argv[1] if len(sys.argv)>1 else 8):]:
 print(json.dumps(r,ensure_ascii=True)[:1800])
