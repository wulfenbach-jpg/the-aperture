import json,subprocess,pathlib
h='20c0d814b9f59a9bce51742f6ff3482a1da59659'; v='a89dbecca28ed7db030a8937a5bbfd61d4b076b5'
d=json.loads(subprocess.check_output(['gh','pr','view','46','--repo','wulfenbach-jpg/the-aperture','--json','headRefOid,state,isDraft,mergeable,mergeStateStatus'],text=True))
assert d['headRefOid']==h and d['state']=='OPEN' and d['isDraft']
changed=subprocess.check_output(['git','diff','--name-only',v,h],text=True).split()
assert changed and all(c.startswith('evidence/') for c in changed), changed
pathlib.Path('evidence/issue17/slice21/ci-final-state.json').write_text(json.dumps({'final_head':h,'last_ci_verified_head':v,'ci_summary':json.loads(pathlib.Path('evidence/issue17/slice21/exact-head-ci-summary-a89dbec.json').read_text()),'delta_files':changed,'pr':d},indent=2)+'\n')
print('FINAL',h,'LAST_CI_GREEN',v,'DELTA',changed)
