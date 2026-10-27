from pathlib import Path
import hashlib,json,shutil,psutil
root=Path.cwd(); dest=root/'evidence/issue17/slice25-final-backup'; rows=[]
for rel in ['evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl','evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl']:
 p=root/rel; assert p.is_file()
 for f in [p]+list(p.with_suffix('').rglob('*')):
  if not f.is_file(): continue
  to=dest/f.relative_to(root); to.parent.mkdir(parents=True,exist_ok=True); shutil.copy2(f,to)
  digest=hashlib.sha256(f.read_bytes()).hexdigest(); assert digest==hashlib.sha256(to.read_bytes()).hexdigest(); rows.append({'path':str(f.relative_to(root)),'sha256':digest})
workers=[]
for p in psutil.process_iter(['name','cmdline']):
 try:
  cmd=p.info['cmdline'] or []
  if p.info['name'].lower() in ('bun.exe','node.exe') and any('ASSIGN-IMPLEMENT24' in s or 'ASSIGN-REPAIR24' in s or 'ASSIGN-REVIEW24' in s or 'ASSIGN-IMPLEMENT25' in s or 'ASSIGN-REVIEW25' in s for s in cmd): workers.append({'pid':p.pid,'cmdline':cmd})
 except (psutil.NoSuchProcess,psutil.AccessDenied): pass
assert not workers,workers
out=root/'evidence/issue17/slice25/history-backup.json'; out.write_text(json.dumps({'files':rows,'count':len(rows),'workers':workers,'directory':str(dest)},indent=2))
print('SHA-verified history files',len(rows),'all assigned OMP processes exited')
