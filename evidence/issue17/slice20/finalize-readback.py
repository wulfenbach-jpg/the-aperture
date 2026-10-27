from pathlib import Path
import subprocess,json,hashlib,psutil,datetime
r=Path('evidence/issue17/slice20')
head='cdfc32e1982758ce0ff5eb6d36f99769c39333ad'
a=Path('C:/Users/alex3/AppData/Local/hermes/kanban/boards/the-aperture/attachments/t_67778feb/SLICE20-final-exact-head-ci-cdfc32e.json')
ci=json.loads(a.read_text()); runs=ci['check_runs']
assert len(runs)==10 and all(v['head_sha']==head and v['status']=='completed' and v['conclusion']=='success' for v in runs)
summary=json.loads((r/'exact-head-ci-summary-cdfc32e.json').read_text())
assert {x['id'] for x in runs}=={x['id'] for x in summary['runs']}
req=json.loads((r/'required-checks-config.json').read_text())['checks']
for rule in req:
 matched=[v for v in runs if v['name']==rule['context'] and v['app']['id']==rule['app_id']]
 assert len(matched)==2 and all(v['conclusion']=='success' for v in matched)
prcmd=['gh','api','repos/wulfenbach-jpg/the-aperture/pulls/46']
d=json.loads(subprocess.check_output(prcmd,text=True));body=d['body'].replace('\r','')
old='Its own exact-head CI is being collected, no inherited-green claim.'
new='Finalcdfc32e has **10/10 completed SUCCESS exact-head CI checks**, all bound to that SHA; all four strict required contexts match their configured GitHub Actions app and pass. This is increment evidence, NOT eight-list acceptance.'
assert old in body
body=body.replace(old,new)
p=subprocess.run(['gh','api','--method','PATCH','repos/wulfenbach-jpg/the-aperture/pulls/46','--input','-'],input=json.dumps({'body':body}),capture_output=True,text=True);p.check_returncode()
z=json.loads(subprocess.check_output(prcmd,text=True));assert z['body'].replace('\r','')==body
assert z['head']['sha']==head and z['state']=='open' and z['draft']
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head
product_diff=subprocess.check_output(['git','diff','--name-only','--','crates','README.md','docs','decks','fixtures'],text=True)
assert not product_diff,product_diff
active=[]
for p in psutil.process_iter(['pid','name','cmdline']):
 try:
  name=(p.info['name'] or '').lower()
  if any(s in name for s in ['bun','cargo','rustc']) and str(Path.cwd()).lower() in p.cwd().lower(): active.append({'pid':p.pid,'name':p.info['name']})
 except (psutil.Error,OSError): pass
assert not active,active
out={'head':head,'reviewed_code':'41d55ee87bbd1de200b40df92ea06d80aecf9516','ci_total':10,'ci_success':10,'all_keyed_to_head':True,'required_pass':True,'ci_attachment_id':114,'checkpoint_attachment_id':112,'review_attachment_id':113,'attachments_readback_verified':True,'pr_state':z['state'],'pr_draft':z['draft'],'pr_mergeable':z['mergeable'],'pr_mergeable_state':z['mergeable_state'],'product_diff_empty':True,'native_omp_cargo_processes':active,'corpus':{'catalog':137,'union':459,'supported':125,'missing':334,'certified':1,'desired':8,'ayula_missing':28},'full_issue17_accepted':False,'checked_at':datetime.datetime.now(datetime.timezone.utc).isoformat()}
(r/'FINAL-READBACK.json').write_text(json.dumps(out,indent=2)+'\n',newline='\n')
print(json.dumps(out,indent=2))
