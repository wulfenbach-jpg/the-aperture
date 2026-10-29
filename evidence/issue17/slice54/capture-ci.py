"""Capture actual GitHub checks for one fixed full SHA; empty is never green."""
import datetime
import json
from pathlib import Path
import subprocess
import sys
import time
head = sys.argv[1]
assert len(head) == 40
out = Path(__file__).resolve().parent / ("exact-head-ci-" + head[:7] + ".json")
for attempt in range(51):
    response = subprocess.run(["gh", "api", "repos/wulfenbach-jpg/the-aperture/commits/" + head + "/check-runs?per_page=100"], check=True, capture_output=True, text=True, encoding="utf-8")
    checks = json.loads(response.stdout)["check_runs"]
    keyed = all(c["head_sha"] == head for c in checks)
    finished = len(checks) >= 10 and keyed and all(c["status"] == "completed" for c in checks)
    receipt = {"head": head, "retrieved_at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "total": len(checks), "all_keyed": keyed, "finished": finished, "all_success": finished and all(c["conclusion"] == "success" for c in checks), "checks": checks}
    out.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(receipt["retrieved_at"], "total", len(checks), "completed", sum(c["status"] == "completed" for c in checks), "all_success", receipt["all_success"], flush=True)
    if finished:
        sys.exit(0 if receipt["all_success"] else 1)
    if attempt < 50:
        time.sleep(30)
sys.exit(2)
