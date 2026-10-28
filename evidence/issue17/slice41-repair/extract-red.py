"""Extract the writer's actual pre-fix failing test tool result, not a new run."""
import json
from pathlib import Path
session = Path('evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl')
rows = [json.loads(line) for line in session.read_text(encoding='utf-8').splitlines()]
for row in rows:
    msg = row.get('message', {})
    text = '\n'.join(x.get('text', '') for x in msg.get('content', []) if isinstance(x, dict))
    if msg.get('role') == 'toolResult' and 'midsearch_after_one_pick_save_reload_continues_identically' in text and 'test result: FAILED' in text and 'library search chose #4 twice' in text:
        dest = Path('evidence/issue17/slice41-repair/writer-red-extracted.log')
        dest.write_text('SOURCE: ' + str(session) + '\nTOOL_CALL: ' + str(msg.get('toolCallId')) + '\nTIMESTAMP: ' + str(msg.get('timestamp')) + '\n' + text, encoding='utf-8')
        print(dest)
        break
else:
    raise SystemExit('Actual failing test result not found')
