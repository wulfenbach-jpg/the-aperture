import json, re, subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[2]
text = (root / 'crates/core/src/catalog.rs').read_text(encoding='utf-8')
names = {n for _, n in re.findall(r'(\w+), "([^"]+)", cost:', text)}
m = json.loads((root / 'decks/starter-corpus/manifest.json').read_text(encoding='utf-8'))
u = set()
rows = []
unchanged = True
for s in m['starters']:
    p = Path(s['deck_path'])
    t = (root / p).read_text(encoding='utf-8')
    cs = [(int(a),b) for a,b in re.findall(r'(?m)^(\d+) (.+)$', t)]
    assert sum(n for n,_ in cs) == 99
    ds = {n for _, n in cs} | {s['commander']}
    u |= ds
    rows.append((s['id'],len(ds-names),set(s['missing_cards'])==ds-names))
    old = subprocess.check_output(['git','show','3987dd6:'+p.as_posix()],cwd=root).decode('utf-8')
    unchanged &= old.replace('\r\n','\n') == t.replace('\r\n','\n')
result = dict(catalog=len(names),union=len(u),supported=len(u&names),missing=len(u-names),lists=rows,desired_unchanged=unchanged)
replay_path=root/'evidence/issue17/slice7-native-journey.replay.json'
if replay_path.exists():
    replay=json.loads(replay_path.read_text(encoding='utf-8'))
    ids={name: ident for ident,name in re.findall(r'(\w+), "([^"]+)", cost:', text)}
    package=(root/'decks/package-slice6-dies.txt').read_text(encoding='utf-8')
    expected=[[ids[name],int(count)] for count,name in re.findall(r'(?m)^(\d+) (.+)$',package)]
    assert len(replay['seats'])==4
    assert all(s['cards']==expected for s in replay['seats'])
    result['journey_exact_four_seat_package_snapshots']=True
print(json.dumps(result,indent=2))
(root/'evidence/issue17/slice7-native-corpus.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
