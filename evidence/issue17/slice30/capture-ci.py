import json, subprocess, sys, time

head = "073fd19eca457868bbd0d6be28026fb3b4071086"
repo = "wulfenbach-jpg/the-aperture"

# Every gh call is bounded: one hung API call must fail the helper, not outlive
# it. A timeout is a hard failure and the helper exits with 124.
GH_TIMEOUT_SECONDS = 120


def run_gh(args):
    try:
        return subprocess.run(
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


def gh(*args):
    out = run_gh(args)
    if out.returncode != 0:
        raise SystemExit(f"gh failed {args}: {out.stderr}")
    return out.stdout

deadline = time.time() + 540
last = None
while time.time() < deadline:
    runs = json.loads(gh("api", f"repos/{repo}/actions/runs?head_sha={head}&per_page=50"))
    rows = [(r["id"], r["event"], r["name"], r["status"], r["conclusion"]) for r in runs["workflow_runs"]]
    if rows != last:
        print(rows, flush=True)
        last = rows
    pending = [r for r in rows if r[3] != "completed"]
    if rows and not pending:
        break
    time.sleep(20)

checks = json.loads(gh("api", f"repos/{repo}/commits/{head}/check-runs?per_page=100"))
print(json.dumps({
    "head": head,
    "total": checks["total_count"],
    "checks": [
        {"name": c["name"], "status": c["status"], "conclusion": c["conclusion"], "head_sha": c["head_sha"]}
        for c in checks["check_runs"]
    ],
}, indent=2))
conclusions = {c["conclusion"] for c in checks["check_runs"]}
keyed = {c["head_sha"] for c in checks["check_runs"]}
print("ALL_SUCCESS", checks["total_count"] >= 8 and conclusions == {"success"} and keyed == {head})
