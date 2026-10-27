from pathlib import Path
import json,subprocess,hashlib,datetime
r=Path('evidence/issue17/slice21'); head='d23da60a3e90e09206d3da9dabefc1537a010fb1'; repo='wulfenbach-jpg/the-aperture'
ci=json.loads((r/'exact-head-ci-summary-d23da60.json').read_text()); assert ci['head']==head and ci['all_success'] and ci['required_pass'] and ci['total']==10
attachments=Path('C:/Users/alex3/AppData/Local/hermes/kanban/boards/the-aperture/attachments/t_67778feb')
a=json.loads((attachments/'SLICE21-final-exact-head-ci-d23da60.json').read_text()); assert len(a['check_runs'])==10 and all(x['head_sha']==head and x['status']=='completed' and x['conclusion']=='success' for x in a['check_runs'])
def get(): return json.loads(subprocess.check_output(['gh','pr','view','46','--repo',repo,'--json','body,headRefOid,state,isDraft,mergeable,mergeStateStatus'],text=True))
d=get(); assert d['headRefOid']==head
old='Its own exact-head CI is being collected, no inherited-green claim.'
new='Its own **10/10 exact-head CI checks SUCCESS**, every check keyed to d23da60 and all strict required contexts matching the configured Actions app pass. No inherited-green claim.'
import re
assert old in d['body'] or new in d['body']
body=d['body'].replace(old,new,1).replace('\r\n','\n').replace('\r','\n')
body=re.sub(r'\n{3,}', '\n\n', body)
(r/'pr-body-final.md').write_text(body, newline='\n')
subprocess.run(['gh','pr','edit','46','--repo',repo,'--body-file',str(r/'pr-body-final.md')],check=True)
z=get(); assert z['body'].replace('\r\n','\n').rstrip()==body.replace('\r\n','\n').rstrip(); assert z['headRefOid']==head and z['state']=='OPEN' and z['isDraft']
local=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(); remote=subprocess.check_output(['git','ls-remote','origin','refs/heads/aperture/issue-17'],text=True).split()[0]; assert local==remote==head
j=json.loads((r/'demo-journey.json').read_text()); assert hashlib.sha256(Path('decks/package-slice21.txt').read_bytes()).hexdigest()==j['snapshot_sha256']
m={'published_pr':'https://github.com/wulfenbach-jpg/the-aperture/pull/46','final_head':head,'reviewed_head':'f59d29be91b77a9f234d268d438f42039915213d','review_outcome':'approved_increment_after_identity_repair','full_issue17_accepted':False,'catalog_cards':138,'union_names':459,'supported_union_names':126,'missing_union_names':333,'ayula_missing':27,'certified_lists':1,'desired_lists':8,'ci_success':10,'ci_all_keyed_to_head':True,'required_checks_pass':True,'checkpoint_attachment_id':115,'review_attachment_id':116,'ci_attachment_id':117,'native_initial_focused_passed':68,'native_repair_focused_passed':9,'journey':j,'ai_compatibility':'search.rs727/testscommon83 new shield-field init only; no policy change','backup_files_verified':59,'all_omp_workers_stopped':True,'next_action':'same_card_sourced_implementation_continuation','no_merge_or_closure':True,'no_certified_downstream_unlock':True}
(r/'FINAL-READBACK.json').write_text(json.dumps({'pr':z,'metadata':m,'verified_at':datetime.datetime.now(datetime.timezone.utc).isoformat()},indent=2)+'\n'); (r/'handoff-metadata.json').write_text(json.dumps(m,indent=2)+'\n')
print('FINAL_VERIFIED',head,'10/10CI','requiredPASS','attachments115/116/117'); print('SCHEDULED_UNTIL='+ (datetime.datetime.now(datetime.timezone.utc)+datetime.timedelta(minutes=1)).isoformat())
