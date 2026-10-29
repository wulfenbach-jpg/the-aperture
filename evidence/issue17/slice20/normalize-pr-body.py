import json,subprocess
from pathlib import Path
cmd=['gh','api','repos/wulfenbach-jpg/the-aperture/pulls/46']
d=json.loads(subprocess.check_output(cmd,text=True))
body=d['body'].replace('\r','')
p=subprocess.run(['gh','api','--method','PATCH','repos/wulfenbach-jpg/the-aperture/pulls/46','--input','-'],input=json.dumps({'body':body}),text=True,capture_output=True)
p.check_returncode()
a=json.loads(subprocess.check_output(cmd,text=True))
assert a['body'].replace('\r','')==body
assert 'APPROVED_INCREMENT at41d55ee' in a['body']
assert a['head']['sha']=='cdfc32e1982758ce0ff5eb6d36f99769c39333ad'
assert a['state']=='open' and a['draft']
Path('evidence/issue17/slice20/pr-final-readback.json').write_text(json.dumps({'body':a['body'],'head':a['head']['sha'],'state':a['state'],'draft':a['draft'],'mergeable':a['mergeable'],'mergeable_state':a['mergeable_state']},indent=2)+'\n',newline='\n')
print('PR_BODY_EXACT_READBACK',a['head']['sha'],'draft',a['draft'],'CR',a['body'].count('\r'))
