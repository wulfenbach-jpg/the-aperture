#!/usr/bin/env python3
"""Capture the exact-head check runs for one commit, with hard timeouts.

Every network call is bounded (the slice-30/31 lesson: an untimed `gh` call in a
run path can wedge for hours). A timeout is a hard failure with its own exit code
(124), never a silent pass.
"""
import json
import subprocess
import sys
import pathlib

REPO = "wulfenbach-jpg/the-aperture"
TIMEOUT = 120


def gh(args):
    try:
        out = subprocess.run(
            ["gh"] + args, capture_output=True, text=True, timeout=TIMEOUT
        )
    except subprocess.TimeoutExpired:
        print(f"TIMEOUT after {TIMEOUT}s: gh {' '.join(args)}", file=sys.stderr)
        sys.exit(124)
    if out.returncode != 0:
        print(f"FAILED gh {' '.join(args)}: {out.stderr.strip()}", file=sys.stderr)
        sys.exit(1)
    return out.stdout


def main():
    if len(sys.argv) < 2:
        print("usage: capture-ci.py <sha> [out.json]", file=sys.stderr)
        sys.exit(2)
    # Resolve an abbreviated sha to the commit id the check runs are keyed to,
    # otherwise an abbreviated `head_sha` comparison can never match and a green
    # head would be reported as ALL_KEYED=False.
    sha = gh(["api", f"repos/{REPO}/commits/{sys.argv[1]}", "--jq", ".sha"]).strip()
    out_path = pathlib.Path(
        sys.argv[2] if len(sys.argv) > 2 else f"evidence/issue17/slice35/exact-head-ci-{sha[:7]}.json"
    )
    payload = json.loads(
        gh(["api", f"repos/{REPO}/commits/{sha}/check-runs", "--paginate"])
    )
    runs = payload["check_runs"]
    heads = {run["head_sha"] for run in runs}
    by_state = {}
    for run in runs:
        key = f"{run['status']}/{run.get('conclusion')}"
        by_state[key] = by_state.get(key, 0) + 1
    all_done_success = all(
        run["status"] == "completed" and run.get("conclusion") == "success"
        for run in runs
    )
    all_keyed = heads == {sha}
    receipt = {
        "sha": sha,
        "total": len(runs),
        "states": by_state,
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
    print(f"total {len(runs)}  states {by_state}")
    print(f"ALL_KEYED={all_keyed} ALL_SUCCESS={all_done_success}")
    for check in receipt["checks"]:
        print(f"  {check['status']:>10} {str(check['conclusion']):>9}  {check['name']}")
    if not (all_done_success and all_keyed):
        sys.exit(1)


if __name__ == "__main__":
    main()
