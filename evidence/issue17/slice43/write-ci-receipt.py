import json, subprocess, datetime

HEAD = "e3cfe4b7985742d471c9a216e65a1074b90161a3"
REPO = "wulfenbach-jpg/the-aperture"
raw = subprocess.run(
    ["gh", "api", f"repos/{REPO}/commits/{HEAD}/check-runs"],
    capture_output=True, text=True, check=True,
).stdout
data = json.loads(raw)
runs = []
for run in data["check_runs"]:
    runs.append({
        "id": run["id"],
        "name": run["name"],
        "status": run["status"],
        "conclusion": run["conclusion"],
        "head_sha": run["head_sha"],
    })
by_name = {}
for run in runs:
    by_name.setdefault(run["name"], []).append(run["conclusion"])
receipt = {
    "schema": "aperture.exact-head-ci.v1",
    "repo": REPO,
    "head": HEAD,
    "read_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "total": data["total_count"],
    "all_completed": all(r["status"] == "completed" for r in runs),
    "all_success": all(r["conclusion"] == "success" for r in runs),
    "all_keyed_to_head": all(r["head_sha"] == HEAD for r in runs),
    "worst_conclusion_by_name": by_name,
    "runs": runs,
    "note": (
        "The first attempt of one windows-latest job failed in the MSVC linker with "
        "'A device which does not exist was specified. (os error 433)' before any test "
        "executed; the failed job was re-run and every check on this head is now a "
        "completed success. No test or compile failure ever appeared in the logs."
    ),
}
path = 'evidence/issue17/slice43/exact-head-ci-e3cfe4b.json'
json.dump(receipt, open(path, 'w'), indent=2)
print(json.dumps({k: receipt[k] for k in ("head", "total", "all_completed",
                                          "all_success", "all_keyed_to_head")}, indent=1))
