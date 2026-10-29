#!/usr/bin/env python3
"""Wait for the exact-head check runs of one commit, then commit the receipt.

Every `gh` call is bounded (timeout=120, exit 124 on timeout) — the slice-30
lesson. Polls until every run is `completed` (or the deadline passes), then
writes the same receipt shape as capture-ci.py and exits non-zero unless all
runs completed successfully and every head_sha is the commit.
"""
import json
import pathlib
import subprocess
import sys
import time

REPO = "wulfenbach-jpg/the-aperture"
TIMEOUT = 120
DEADLINE_S = 1500


def gh(args):
    try:
        out = subprocess.run(["gh"] + args, capture_output=True, text=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        print(f"TIMEOUT after {TIMEOUT}s: gh {' '.join(args)}", file=sys.stderr)
        sys.exit(124)
    if out.returncode != 0:
        print(f"FAILED gh {' '.join(args)}: {out.stderr.strip()}", file=sys.stderr)
        sys.exit(1)
    return out.stdout


def main():
    sha = gh(["api", f"repos/{REPO}/commits/{sys.argv[1]}", "--jq", ".sha"]).strip()
    out_path = pathlib.Path(sys.argv[2])
    start = time.time()
    while True:
        payload = json.loads(gh(["api", f"repos/{REPO}/commits/{sha}/check-runs", "--paginate"]))
        runs = payload["check_runs"]
        states = {}
        for run in runs:
            key = f"{run['status']}/{run.get('conclusion')}"
            states[key] = states.get(key, 0) + 1
        pending = [r for r in runs if r["status"] != "completed"]
        print(f"[{int(time.time() - start)}s] runs={len(runs)} states={states} pending={len(pending)}")
        # `runs == 0` means the workflow has not been created yet, not that it is
        # green: keep polling until at least one run exists.
        if (runs and not pending) or time.time() - start > DEADLINE_S:
            break
        time.sleep(30)

    heads = {run["head_sha"] for run in runs}
    all_done_success = all(
        run["status"] == "completed" and run.get("conclusion") == "success" for run in runs
    )
    all_keyed = heads == {sha}
    receipt = {
        "sha": sha,
        "total": len(runs),
        "states": states,
        "head_shas": sorted(heads),
        "all_keyed": all_keyed,
        "all_completed_success": all_done_success,
        "checks": sorted(
            (
                {
                    "name": run["name"],
                    "status": run["status"],
                    "conclusion": run.get("conclusion"),
                    "head_sha": run["head_sha"],
                }
                for run in runs
            ),
            key=lambda c: c["name"],
        ),
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(receipt, indent=1), encoding="utf-8")
    print(f"head {sha}")
    print(f"total {len(runs)}  states {states}")
    print(f"ALL_KEYED={all_keyed} ALL_SUCCESS={all_done_success}")
    for check in receipt["checks"]:
        print(f"  {check['status']:>10} {str(check['conclusion']):>9}  {check['name']}")
    if not (all_done_success and all_keyed):
        sys.exit(1)


if __name__ == "__main__":
    main()
