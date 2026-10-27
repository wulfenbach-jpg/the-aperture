from pathlib import Path
import shutil,hashlib,json,subprocess,datetime
root=Path.cwd()
manifest=[]
for name in ['sessions','review-sessions']:
    src=root/'evidence/issue17'/name
    dst=root/'evidence/issue17/slice20-final-backup'/name
    shutil.copytree(src,dst,dirs_exist_ok=True)
    for p in src.rglob('*'):
        if p.is_file():
            q=dst/p.relative_to(src)
            a=hashlib.sha256(p.read_bytes()).hexdigest()
            assert a==hashlib.sha256(q.read_bytes()).hexdigest(),str(p)
            manifest.append({'path':str(p.relative_to(root)),'sha256':a,'bytes':p.stat().st_size})
print(json.dumps({'verified_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'backup_files':len(manifest),'files':manifest},indent=2))
