"""Capture the exact-head CI receipt for slice 22.

Writes the raw check-runs payload plus a compact summary, asserting that every
check is keyed to the head and completed SUCCESS. Exits non-zero when the head is
not fully green so a caller can never mistake "still running" for a pass.
"""
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path

HEAD = "89247d66dd0de8a4923aa6bf3537326ac22d2509"
REPO = "wulfenbach-jpg/the-aperture"
out = Path("C:/AI/aperture-worktrees/issue-17/evidence/issue17/slice22")


def gh(*args: str) -> str:
    return subprocess.run(
        ["gh", *args], capture_output=True, text=True, check=True
    ).stdout


raw = json.loads(gh("api", f"repos/{REPO}/commits/{HEAD}/check-runs?per_page=100"))
(out / f"exact-head-ci-{HEAD[:7]}.json").write_text(
    json.dumps(raw, indent=1), encoding="utf-8"
)
runs = raw["check_runs"]
states = Counter(f"{run['status']}:{run['conclusion']}" for run in runs)
wrong_head = sorted({run["head_sha"] for run in runs} - {HEAD})
summary = {
    "head": HEAD,
    "total": len(runs),
    "states": dict(states),
    "all_keyed_to_head": not wrong_head,
    "wrong_heads": wrong_head,
    "checks": [
        {
            "name": run["name"],
            "status": run["status"],
            "conclusion": run["conclusion"],
            "head_sha": run["head_sha"],
            "app": (run.get("app") or {}).get("id"),
            "required": run.get("is_required"),
        }
        for run in runs
    ],
}
(out / f"exact-head-ci-summary-{HEAD[:7]}.json").write_text(
    json.dumps(summary, indent=1), encoding="utf-8"
)
print(json.dumps({k: summary[k] for k in ("head", "total", "states", "all_keyed_to_head", "wrong_heads")}, indent=1))
green = len(runs) == 10 and all(
    run["status"] == "completed" and run["conclusion"] == "success" for run in runs
)
print("ALL_GREEN" if green else "NOT_YET_GREEN")
sys.exit(0 if green else 2)
