import json,pathlib,collections
p=pathlib.Path('evidence/issue17')
res={}
for kind,path in [('writer',p/'sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl'),('reviewer',p/'review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl')]:
    rows=[json.loads(l) for l in path.read_text(encoding='utf8').splitlines() if l.strip()]
    c=collections.Counter()
    for r in rows:
        m=r.get('message') or {}
        if m.get('role')=='assistant':
            c[(m.get('provider'),m.get('model'))]+=1
    res[kind]={'assistant_turns':sum(c.values()),'models':{f'{k[0]}/{k[1]}':v for k,v in c.items()}}
print(json.dumps(res,indent=2))
pathlib.Path('evidence/issue17/slice3-history-readback.json').write_text(json.dumps(res,indent=2)+'\n')
