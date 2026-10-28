"""Bounded read-only CI wait; requires >=10 runs, exact SHA, all successful."""
import datetime,json,pathlib,subprocess,sys,time
sha=sys.argv[1]
assert len(sha)==40 and all(c in '0123456789abcdef' for c in sha)
out=pathlib.Path(sys.argv[2]); start=time.monotonic()
while True:
    raw=subprocess.check_output(['gh','api',f'repos/wulfenbach-jpg/the-aperture/commits/{sha}/check-runs?per_page=100'],text=True,timeout=90)
    payload=json.loads(raw); runs=payload['check_runs']
    keyed=bool(runs) and all(r['head_sha']==sha for r in runs)
    done=len(runs)>=10 and all(r['status']=='completed' for r in runs)
    success=done and keyed and all(r['conclusion']=='success' for r in runs)
    print(int(time.monotonic()-start),len(runs),[(r['name'],r['status'],r['conclusion']) for r in runs],flush=True)
    if done or time.monotonic()-start>1800:
        payload.update({'verified_sha':sha,'retrieved_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'all_keyed':keyed,'all_completed_success':success})
        out.write_text(json.dumps(payload,indent=2),encoding='utf8')
        sys.exit(0 if success else 1)
    time.sleep(30)
