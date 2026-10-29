"""Wait for a head's check runs to appear, then require all SUCCESS keyed to it.

Same bounded gh policy as capture-ci.py: every gh call has a timeout and a
timeout is a hard failure (exit 124). Additionally the first stage tolerates an
empty check list (a just-pushed head whose workflow runs have not registered
yet) for up to FIRST_CHECK_DEADLINE seconds.
"""
import json
import subprocess
import sys
import time

HEAD = sys.argv[1]
OUT = sys.argv[2]
REPO = "wulfenbach-jpg/the-aperture"
DEADLINE = time.time() + 900
FIRST_CHECK_DEADLINE = time.time() + 240
GH_TIMEOUT_SECONDS = 120


def gh(*args):
    try:
        r = subprocess.run(
            ["gh", *args],
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=GH_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        print(f"gh {args} exceeded {GH_TIMEOUT_SECONDS}s; failing hard (exit 124)", file=sys.stderr, flush=True)
        raise SystemExit(124)
    if r.returncode != 0:
        raise SystemExit(f"gh failed {args}: {r.stderr}")
    return r.stdout


rows = []
while True:
    payload = json.loads(gh("api", f"repos/{REPO}/commits/{HEAD}/check-runs?per_page=100"))
    rows = [
        {
            "name": c["name"],
            "status": c["status"],
            "conclusion": c["conclusion"],
            "head_sha": c["head_sha"],
            "app": c.get("app", {}).get("slug"),
        }
        for c in payload["check_runs"]
    ]
    pending = [r for r in rows if r["status"] != "completed"]
    print(f"{len(rows)} checks, {len(pending)} pending", flush=True)
    if rows and not pending:
        break
    if time.time() > DEADLINE:
        break
    if not rows and time.time() > FIRST_CHECK_DEADLINE:
        break
    time.sleep(15)

receipt = {"head": HEAD, "total": len(rows), "checks": rows}
with open(OUT, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)

green = bool(rows) and all(r["conclusion"] == "success" for r in rows)
keyed = {r["head_sha"] for r in rows} == {HEAD}
print(f"TOTAL={len(rows)} ALL_SUCCESS={green} ALL_KEYED={keyed} PENDING={len(pending)}")
sys.exit(0 if (green and keyed and len(rows) >= 8) else 1)
