import json,re,subprocess,sys
from pathlib import Path
out=Path(__file__).parent; repo='wulfenbach-jpg/the-aperture'
def get(): return json.loads(subprocess.check_output(['gh','pr','view','46','--repo',repo,'--json','body,headRefOid,state,isDraft'],text=True))
r=get(); body=r['body'].replace('\r',''); body=re.sub(r'\n{3,}','\n\n',body)
if '--green' in sys.argv:
 receipt=json.loads((out/'exact-head-ci-summary-ea10693.json').read_text()); assert receipt['green'] and receipt['head']==r['headRefOid']
 body=body.replace('Exact-head CI still pending at this update, not inherited green.','Final exact-head CI: **10/10 completed SUCCESS**, every check keyed to `ea106932d649da6876004fdf5c1a2ce6e441d19c`; all strict required contexts pass with Actions app15368. No inherited-green claim.')
(out/'pr-body-final.md').write_text(body,encoding='utf-8',newline='\n')
subprocess.run(['gh','pr','edit','46','--repo',repo,'--body-file',str(out/'pr-body-final.md')],check=True)
a=get(); assert a['body'].replace('\r','')==body
assert a['headRefOid']=='ea106932d649da6876004fdf5c1a2ce6e441d19c'
(out/'pr-final-readback.json').write_text(json.dumps(a,indent=2)); print({k:v for k,v in a.items() if k!='body'})
