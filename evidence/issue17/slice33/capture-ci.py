"""Capture the exact-head CI check runs for one commit.

Every gh call is bounded (GH_TIMEOUT_SECONDS) and a timeout is a hard failure
with exit code 124, so a hung API call can never outlive this helper.
"""
import json
import subprocess
import sys
import time

HEAD = sys.argv[1]
OUT = sys.argv[2]
REPO = "wulfenbach-jpg/the-aperture"
DEADLINE = time.time() + 900
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
        print(
            f"gh {args} exceeded {GH_TIMEOUT_SECONDS}s; failing hard (exit 124)",
            file=sys.stderr,
            flush=True,
        )
        raise SystemExit(124)
    if r.returncode != 0:
        raise SystemExit(f"gh failed {args}: {r.stderr}")
    return r.stdout


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
    if not pending:
        break
    if time.time() > DEADLINE:
        break
    time.sleep(20)

receipt = {"head": HEAD, "total": len(rows), "checks": rows}
with open(OUT, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)

green = all(r["conclusion"] == "success" for r in rows)
keyed = {r["head_sha"] for r in rows} == {HEAD}
print(f"TOTAL={len(rows)} ALL_SUCCESS={green} ALL_KEYED={keyed} PENDING={len(pending)}")
sys.exit(0 if (green and keyed and len(rows) >= 8) else 1)
