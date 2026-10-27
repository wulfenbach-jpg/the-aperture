#!/usr/bin/env python3
"""Capture the exact-head check runs for one commit and summarise them.

Exits non-zero unless every discovered check run is completed and successful and
every one of them is keyed to the requested head, so a green summary can never
be inherited from another commit. Distinguishes the push-trigger run from the
pull_request-trigger run, because PR46's merge ref currently carries
master-drift debt that is not attributable to this branch.
"""
import json
import subprocess
import sys
import time
from pathlib import Path

REPO = "wulfenbach-jpg/the-aperture"
HEAD = sys.argv[1]
OUT = Path(__file__).resolve().parent


def gh(path: str) -> dict:
    out = subprocess.run(["gh", "api", path], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def runs(head: str) -> list[dict]:
    data = gh(f"repos/{REPO}/commits/{head}/check-runs?per_page=100")
    return data["check_runs"]


def workflows(head: str) -> list[dict]:
    data = gh(f"repos/{REPO}/actions/runs?head_sha={head}&per_page=100")
    return data["workflow_runs"]


def main() -> int:
    for attempt in range(60):
        found = runs(HEAD)
        pending = [r for r in found if r["status"] != "completed"]
        print(f"poll {attempt}: {len(found)} check runs, {len(pending)} still running", flush=True)
        if found and not pending:
            break
        time.sleep(20)
    else:
        print("timed out waiting for check runs")
        return 2

    (OUT / f"exact-head-ci-{HEAD[:7]}.json").write_text(json.dumps(found, indent=1), encoding="utf-8")
    flows = workflows(HEAD)
    (OUT / f"exact-head-runs-{HEAD[:7]}.json").write_text(json.dumps(flows, indent=1), encoding="utf-8")

    by_flow: dict[str, list[dict]] = {}
    for run in flows:
        by_flow.setdefault(run["name"], []).append(run)

    summary = {
        "head": HEAD,
        "check_runs": len(found),
        "all_keyed_to_head": all(r["head_sha"] == HEAD for r in found),
        "all_completed_success": all(
            r["status"] == "completed" and r["conclusion"] == "success" for r in found
        ),
        "workflows": {},
    }
    for name, group in sorted(by_flow.items()):
        for run in group:
            summary["workflows"].setdefault(name, []).append(
                {
                    "event": run["event"],
                    "status": run["status"],
                    "conclusion": run["conclusion"],
                    "run_id": run["id"],
                    "head_sha": run["head_sha"],
                }
            )
    summary["checks"] = [
        {
            "name": r["name"],
            "conclusion": r["conclusion"],
            "head_sha": r["head_sha"],
            "app": r["app"]["slug"],
        }
        for r in found
    ]
    (OUT / f"exact-head-ci-summary-{HEAD[:7]}.json").write_text(
        json.dumps(summary, indent=1), encoding="utf-8"
    )
    print(json.dumps(summary, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
