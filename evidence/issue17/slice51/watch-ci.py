#!/usr/bin/env python3
"""Poll GitHub check runs for one exact commit and write the receipt JSON.

Usage: watch-ci.py <sha> <out.json>
Writes only completed results; never marks a pending check as success.
"""
import json
import subprocess
import sys
import time

REPO = "wulfenbach-jpg/the-aperture"
sha = sys.argv[1]
out = sys.argv[2]
deadline = time.time() + 2400

while True:
    raw = subprocess.run(
        ["gh", "api", f"repos/{REPO}/commits/{sha}/check-runs?per_page=100"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    data = json.loads(raw)
    runs = data["check_runs"]
    done = [r for r in runs if r["status"] == "completed"]
    bad = [r for r in done if r["conclusion"] not in ("success", "neutral", "skipped")]
    summary = {
        "head": sha,
        "total": len(runs),
        "completed": len(done),
        "success": len([r for r in done if r["conclusion"] == "success"]),
        "failed_or_other": [{"name": r["name"], "conclusion": r["conclusion"]} for r in bad],
        "pending": [r["name"] for r in runs if r["status"] != "completed"],
        "all_keyed": all(r["head_sha"] == sha for r in runs),
        "runs": [
            {
                "name": r["name"],
                "status": r["status"],
                "conclusion": r["conclusion"],
                "head_sha": r["head_sha"],
                "started_at": r["started_at"],
                "completed_at": r["completed_at"],
                "details_url": r["html_url"],
            }
            for r in runs
        ],
        "read_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    with open(out, "w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2)
        handle.write("\n")
    print(
        f"{summary['completed']}/{summary['total']} completed, "
        f"{summary['success']} success, {len(bad)} not-success, "
        f"{len(summary['pending'])} pending",
        flush=True,
    )
    if len(done) == len(runs) and runs:
        print("ALL COMPLETE", flush=True)
        break
    if time.time() > deadline:
        print("TIMED OUT", flush=True)
        break
    time.sleep(30)
