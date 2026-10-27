from pathlib import Path
import shutil,hashlib,json,datetime,psutil
root=Path.cwd(); manifest=[]
for p in psutil.process_iter(['pid','name','cmdline']):
 if p.info['name'] and any(x in p.info['name'].lower() for x in ['bun','cargo','rustc']): print('LIVE_PROCESS',p.info)
for name in ['sessions','review-sessions']:
 src=root/'evidence/issue17'/name; dst=root/'evidence/issue17/slice21-final-backup'/name
 shutil.copytree(src,dst,dirs_exist_ok=True)
 for p in src.rglob('*'):
  if p.is_file():
   q=dst/p.relative_to(src); h=hashlib.sha256(p.read_bytes()).hexdigest(); assert h==hashlib.sha256(q.read_bytes()).hexdigest()
   manifest.append({'path':str(p.relative_to(root)),'sha256':h,'bytes':p.stat().st_size})
out={'verified_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'backup_files':len(manifest),'files':manifest}
(root/'evidence/issue17/slice21/final-backup-manifest.json').write_text(json.dumps(out,indent=2)+'\n'); print('BYTE_VERIFIED_FILES',len(manifest))
