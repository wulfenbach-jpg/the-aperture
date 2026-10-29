from pathlib import Path
import json,base64,hashlib
r=Path('evidence/issue17/slice20')
d=json.loads((r/'exact-head-ci-summary-cdfc32e.json').read_text())
assert d['all_success'] and d['required_pass'] and d['head']=='cdfc32e1982758ce0ff5eb6d36f99769c39333ad'
small={k:d[k] for k in ['head','captured_at','total','all_completed','all_success','all_keyed_to_head','required_checks','required_pass']}
small['check_runs']=[{'id':v['id'],'name':v['name'],'conclusion':v['conclusion']} for v in d['runs']]
small['raw_capture_sha256']=hashlib.sha256((r/'exact-head-ci-cdfc32e.json').read_bytes()).hexdigest()
small['reviewed_code']='41d55ee87bbd1de200b40df92ea06d80aecf9516'
small['full_issue17_accepted']=False
payload=json.dumps(small,separators=(',',':')).encode()
(r/'SLICE20-final-exact-head-ci-compact.json').write_bytes(payload)
print(base64.b64encode(payload).decode())
