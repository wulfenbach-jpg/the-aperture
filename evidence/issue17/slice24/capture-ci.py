"""Capture exact SHA checks and repository required contexts; exit2 if pending."""
import json,subprocess,sys,datetime
from pathlib import Path
from collections import Counter
head=sys.argv[1]; assert len(head)==40
out=Path(sys.argv[2]); repo='wulfenbach-jpg/the-aperture'
def gh(*args):
 return json.loads(subprocess.check_output(['gh',*args],text=True))
raw=gh('api',f'repos/{repo}/commits/{head}/check-runs?per_page=100')
required=gh('api',f'repos/{repo}/branches/master/protection/required_status_checks')
runs=raw['check_runs']; assert raw['total_count']==len(runs)
wrong={r['head_sha'] for r in runs}-{head}
checks=required.get('checks',[])
required_pass=bool(checks) and all(any(r['name']==c['context'] and (c.get('app_id') is None or r['app']['id']==c['app_id']) and r['head_sha']==head and r['status']=='completed' and r['conclusion']=='success' for r in runs) for c in checks)
green=len(runs)==10 and not wrong and all(r['status']=='completed' and r['conclusion']=='success' for r in runs) and required_pass
summary={'head':head,'retrieved_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'total':len(runs),'states':dict(Counter(f"{r['status']}:{r['conclusion']}" for r in runs)),'all_keyed_to_head':not wrong,'required_checks':checks,'required_pass':required_pass,'green':green,'checks':[{'name':r['name'],'status':r['status'],'conclusion':r['conclusion'],'head_sha':r['head_sha'],'app_id':r['app']['id']} for r in runs]}
(out/f'exact-head-ci-{head[:7]}.json').write_text(json.dumps(raw,indent=2))
(out/f'exact-head-ci-summary-{head[:7]}.json').write_text(json.dumps(summary,indent=2))
print(json.dumps({k:v for k,v in summary.items() if k!='checks'},indent=2)); sys.exit(0 if green else 2)
