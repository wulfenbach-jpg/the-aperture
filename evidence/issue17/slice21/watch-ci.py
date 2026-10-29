import subprocess,json,time,sys,datetime
from pathlib import Path
head=sys.argv[1]; root=Path('evidence/issue17/slice21'); repo='wulfenbach-jpg/the-aperture'
required=json.loads((root/'required-checks-config.json').read_text())['checks']
for attempt in range(50):
 p=subprocess.run(['gh','api',f'repos/{repo}/commits/{head}/check-runs'],capture_output=True,text=True,timeout=45)
 if p.returncode: print('API_ERROR',p.stderr,flush=True);time.sleep(30);continue
 data=json.loads(p.stdout); (root/f'exact-head-ci-{head[:7]}.json').write_text(json.dumps(data,indent=2)+'\n'); runs=data['check_runs']; assert all(r['head_sha']==head for r in runs)
 print(datetime.datetime.now(datetime.timezone.utc).isoformat(),[(r['name'],r['status'],r['conclusion']) for r in runs],flush=True)
 if len(runs)>=10 and all(r['status']=='completed' for r in runs):
  req=[]
  for rule in required:
   matches=[r for r in runs if r['name']==rule['context'] and r['app']['id']==rule['app_id']]
   req.append({'context':rule['context'],'app_id':rule['app_id'],'matched':len(matches),'pass':bool(matches) and all(r['conclusion']=='success' for r in matches)})
  out={'head':head,'captured_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'total':len(runs),'all_completed':True,'all_success':all(r['conclusion']=='success' for r in runs),'all_keyed_to_head':True,'required_checks':req,'required_pass':all(r['pass'] for r in req),'runs':[{'id':r['id'],'name':r['name'],'head_sha':r['head_sha'],'status':r['status'],'conclusion':r['conclusion'],'url':r['html_url']} for r in runs]}
  (root/f'exact-head-ci-summary-{head[:7]}.json').write_text(json.dumps(out,indent=2)+'\n'); print('FINAL',json.dumps(out),flush=True); sys.exit(0 if out['all_success'] and out['required_pass'] else 1)
 time.sleep(30)
sys.exit('CI timeout')
