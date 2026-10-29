from pathlib import Path
import json

p = Path('evidence/issue17/slice59/final/FINAL-RESULT.json')
d = json.loads(p.read_text(encoding='utf-8'))
d['tests']['ring_repairs'] = '10 passed'
d['master_reconciliation'] = {
    'merge_commit': '2f8cbd8',
    'merged_base': 'origin/master 73820d7',
    'defect': 'published merge ref lacked fn sample_discard after master #34 removed it, while the branch still called it; the Ring discard also offered one hard-coded sample while check_selection accepted any legal card',
    'fix': 'Ring discard now uses master hand_choice_offers(player, 1, HandChoice::Discard); regression asserts one named offer per card in hand and every offer validates',
}
p.write_text(json.dumps(d, indent=2) + '\n', encoding='utf-8', newline='\n')
print('FINAL-RESULT updated')
