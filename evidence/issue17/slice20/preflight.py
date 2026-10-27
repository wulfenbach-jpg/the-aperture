from pathlib import Path
import json, psutil, shutil, hashlib
root=Path.cwd()
d=json.loads(Path('evidence/issue17/slice17/remaining-ayula-sources.json').read_text())
for c in d['cards']:
    if c['name'] in ['Stuffed Bear', "Vivien's Grizzly"]: print(json.dumps(c))
for p in psutil.process_iter(['pid','name','cmdline']):
    if p.info['name'] and any(x in p.info['name'].lower() for x in ['cargo','rustc','bun']):
        try: print('LIVE',p.info,'CWD',p.cwd())
        except psutil.Error: pass
for p in [Path('evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl'),Path('evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl')]:
    assert p.is_file()
    dest=Path('evidence/issue17/slice20-backup')/p.parent.name
    shutil.copytree(p.parent,dest,dirs_exist_ok=True)
    assert hashlib.sha256(p.read_bytes()).digest()==hashlib.sha256((dest/p.name).read_bytes()).digest()
    print('HISTORY',p,p.stat().st_size)
print('CONFIG MODEL ROLES')
p=Path.home()/'.omp/agent/config.yml'
s=p.read_text()
for i,l in enumerate(s.splitlines()):
    if 'modelRoles' in l: print('\n'.join(s.splitlines()[i:i+15]))
