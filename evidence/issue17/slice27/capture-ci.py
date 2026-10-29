import json
import subprocess

HEAD = "27d8f48"
PUSH_RUN = "40493923124"
PR_RUN = "40493931766"


def run(*args):
    out = subprocess.run(args, capture_output=True, text=True, check=True).stdout
    return json.loads(out) if out.strip().startswith(("{", "[")) else out


def dump_run(run_id):
    data = run("gh", "run", "view", run_id, "--json",
               "databaseId,event,status,conclusion,headSha,jobs,name")
    return {
        "run": data["databaseId"],
        "event": data["event"],
        "name": data["name"],
        "status": data["status"],
        "conclusion": data["conclusion"],
        "head_sha": data["headSha"],
        "jobs": [
            {"name": j["name"], "status": j["status"], "conclusion": j["conclusion"],
             "job_id": j["databaseId"], "started_at": j["startedAt"],
             "completed_at": j["completedAt"]}
            for j in data["jobs"]
        ],
    }


push = dump_run(PUSH_RUN)
pr = dump_run(PR_RUN)

checks = run("gh", "api", f"repos/wulfenbach-jpg/the-aperture/commits/{HEAD}/check-runs")
by_id = {c["id"]: c for c in checks["check_runs"]}
for entry in push["jobs"] + pr["jobs"]:
    c = by_id.get(int(entry["job_id"]))
    entry["app"] = c["app"]["slug"] if c else None
    entry["app_id"] = c["app"]["id"] if c else None
    entry["head_sha"] = c["head_sha"] if c else None

report = {
    "head": HEAD,
    "branch_head_ci": push,
    "merge_ref_ci": pr,
    "note": (
        "push run is keyed to the branch head 27d8f48 and is fully green; the "
        "pull_request run compiles the synthetic merge of origin/master 51dffda "
        "(#28 move review, PR56) with this branch and fails in master's own new "
        "crates/ai/tests/review.rs:60, which passes a u8 to this branch's "
        "u32 mana::add (crates/core/src/mana.rs:86). That widening is branch-only "
        "commit e300bdd (approved slice 9 work), so the merge-ref failure is a "
        "cross-lane API conflict owned by the integration supervisor, not a "
        "defect introduced by the slice-27 record repair."
    ),
}
open("evidence/issue17/slice27/ci-27d8f48.json", "w", encoding="utf-8").write(
    json.dumps(report, indent=2))
print(json.dumps({
    "push": (push["event"], push["conclusion"],
             [(j["name"], j["conclusion"], j["head_sha"]) for j in push["jobs"]]),
    "pr": (pr["event"], pr["conclusion"],
           [(j["name"], j["conclusion"], j["head_sha"]) for j in pr["jobs"]]),
}, indent=2))
