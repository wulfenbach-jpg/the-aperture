"""Capture actual GitHub check runs for one fixed, full commit SHA."""
import datetime
import json
from pathlib import Path
import subprocess
import sys
import time

HEAD = sys.argv[1]
OUT = Path(__file__).resolve().parent / ("exact-head-ci-" + HEAD[:7] + ".json")
assert len(HEAD) == 40
for attempt in range(41):
    result = subprocess.run(
        ["gh", "api", "repos/wulfenbach-jpg/the-aperture/commits/" + HEAD + "/check-runs?per_page=100"],
        check=True, capture_output=True, text=True, encoding="utf-8",
    )
    data = json.loads(result.stdout)
    checks = data["check_runs"]
    all_keyed = all(c["head_sha"] == HEAD for c in checks)
    finished = len(checks) >= 10 and all_keyed and all(c["status"] == "completed" for c in checks)
    receipt = {
        "head": HEAD,
        "retrieved_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "total": len(checks),
        "all_keyed": all_keyed,
        "finished": finished,
        "all_success": finished and all(c["conclusion"] == "success" for c in checks),
        "checks": checks,
    }
    OUT.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(receipt["retrieved_at"], "total", len(checks), "completed", sum(c["status"] == "completed" for c in checks), "all_success", receipt["all_success"], flush=True)
    if finished:
        sys.exit(0 if receipt["all_success"] else 1)
    if attempt < 40:
        time.sleep(30)
sys.exit(2)
