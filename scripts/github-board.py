#!/usr/bin/env python
"""Idempotent roadmap bootstrap and token-free public board snapshots.

Authentication stays in gh's credential store. Never serialize tokens.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs" / "roadmap.json"
STATUSES = ("backlog", "ready", "in-progress", "review", "blocked")


def gh(*args: str, payload: dict | None = None):
    command = ["gh", *args]
    if payload is not None:
        command += ["--input", "-"]
    result = subprocess.run(
        command,
        input=None if payload is None else json.dumps(payload),
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if result.returncode:
        raise RuntimeError(f"gh operation failed ({result.returncode}): {result.stderr.strip()}")
    return json.loads(result.stdout) if result.stdout.strip() else None


def pages(endpoint: str) -> list:
    batches = gh("api", endpoint, "--paginate", "--slurp")
    return [item for batch in batches for item in batch]


def all_issues(repo: str) -> list:
    return [i for i in pages(f"repos/{repo}/issues?state=all&per_page=100") if "pull_request" not in i]


def snapshot(repo: str) -> dict:
    if gh("api", f"repos/{repo}")["private"]:
        raise RuntimeError("Refusing to publish a public snapshot of a private repository")
    issues = all_issues(repo)
    public = {
        "repository": repo,
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "issues": [
            {
                "number": i["number"],
                "title": i["title"],
                "url": i["html_url"],
                "state": i["state"],
                "labels": [label["name"] for label in i["labels"]],
                "milestone": i["milestone"]["title"] if i["milestone"] else None,
            }
            for i in issues
        ],
    }
    (ROOT / "docs" / "issues.json").write_text(json.dumps(public, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return public


def bootstrap(repo: str, manifest: dict) -> dict:
    metadata = gh("api", f"repos/{repo}")
    if metadata["private"]:
        raise RuntimeError("Refusing to publish a public snapshot of a private repository")
    labels = {label["name"] for label in pages(f"repos/{repo}/labels?per_page=100")}
    desired = {f"status:{s}": "74613e" for s in STATUSES}
    desired.update({"priority:P0": "c86a5a", "priority:P1": "b08d57", "priority:P2": "9a958a", "scope:v1": "7fbf7f"})
    desired.update({f"area:{i['area']}": "50545e" for i in manifest["issues"]})
    for name, color in desired.items():
        if name not in labels:
            gh("api", f"repos/{repo}/labels", "--method", "POST", payload={"name": name, "color": color, "description": "THE APERTURE release planning"})
    milestones = {m["title"]: m for m in pages(f"repos/{repo}/milestones?state=all&per_page=100")}
    for milestone in manifest["milestones"]:
        if milestone["title"] not in milestones:
            created = gh("api", f"repos/{repo}/milestones", "--method", "POST", payload={**milestone, "state": "open"})
            milestones[milestone["title"]] = created
    existing = all_issues(repo)
    by_title = {i["title"]: i for i in existing}
    by_key = {}
    for issue in existing:
        match = re.search(r"<!-- aperture:roadmap:([a-z0-9-]+) -->", issue.get("body") or "")
        if match:
            if match[1] in by_key:
                raise RuntimeError(f"Duplicate roadmap key {match[1]}")
            by_key[match[1]] = issue
    created_keys = []
    for spec in manifest["issues"]:
        key = spec["key"]
        if key in by_key:
            continue
        if spec["title"] in by_title:
            raise RuntimeError(f"Matching title without ownership marker: {spec['title']}")
        missing = [dep for dep in spec["depends_on"] if dep not in by_key]
        if missing:
            raise RuntimeError(f"Dependencies must precede {key}: {missing}")
        dependencies = ", ".join(f"#{by_key[dep]['number']}" for dep in spec["depends_on"]) or "None"
        checks = "\n".join(f"- [ ] {item}" for item in spec["acceptance"])
        body = (
            f"<!-- aperture:roadmap:{key} -->\n## Outcome\n{spec['title']}\n\n"
            f"## Dependencies\n{dependencies}\n\n## Acceptance criteria\n{checks}\n\n"
            "## Evidence required before Done\n"
            "Link the reviewed commit/PR, exact commands and real results. Add regression tests before rule changes, "
            "cite the pinned rules where applicable, preserve hidden information and record remaining limitations. "
            "A worker launch, a screenshot alone, or an unexecuted plan does not satisfy acceptance.\n\n"
            "## Workflow\nUse exactly one `status:` label while open. Closed issues appear in Done. "
            "Change labels on GitHub or with `python scripts/github-board.py status NUMBER STATUS`.\n"
        )
        item = gh("api", f"repos/{repo}/issues", "--method", "POST", payload={
            "title": spec["title"], "body": body,
            "milestone": milestones[spec["milestone"]]["number"],
            "labels": [f"status:{spec['status']}", f"priority:{spec['priority']}", f"area:{spec['area']}", "scope:v1"],
            "assignees": [],
        })
        by_key[key] = item
        created_keys.append(key)
        print(json.dumps({"created_issue": item["number"], "key": key}), flush=True)
    # Read back the exact target resources after every batch of remote writes.
    # GitHub's issue-list index can lag a successful create. Retry read-back,
    # never infer that an acknowledged write is already visible in the list.
    import time

    expected_keys = {spec["key"] for spec in manifest["issues"]}
    for attempt in range(5):
        verified_keys = {}
        for item in all_issues(repo):
            match = re.search(r"<!-- aperture:roadmap:([a-z0-9-]+) -->", item.get("body") or "")
            if match:
                if match[1] in verified_keys:
                    raise RuntimeError(f"Duplicate remote roadmap key: {match[1]}")
                verified_keys[match[1]] = item
        if expected_keys <= verified_keys.keys():
            break
        if attempt < 4:
            time.sleep(1)
    else:
        raise RuntimeError(f"Remote issue index still missing: {sorted(expected_keys - verified_keys.keys())}")
    for spec in manifest["issues"]:
        item = verified_keys[spec["key"]]
        if item["title"] != spec["title"] or item["milestone"]["title"] != spec["milestone"]:
            raise RuntimeError(f"Remote title/milestone differs for {spec['key']}")
        if not all(f"- [ ] {check}" in item["body"] or f"- [x] {check}" in item["body"] for check in spec["acceptance"]):
            raise RuntimeError(f"Acceptance criteria missing for {spec['key']}")
        states = [label["name"] for label in item["labels"] if label["name"].startswith("status:")]
        if item["state"] == "open" and len(states) != 1:
            raise RuntimeError(f"Open issue has ambiguous status: {spec['key']}")
    verified_milestones = {m["title"] for m in pages(f"repos/{repo}/milestones?state=all&per_page=100")}
    if not all(m["title"] in verified_milestones for m in manifest["milestones"]):
        raise RuntimeError("Missing remote milestone")
    result = snapshot(repo)
    return {"verified_roadmap_issues": len(manifest["issues"]), "verified_milestones": len(manifest["milestones"]), "created_keys": created_keys, "snapshot_issues": len(result["issues"])}


def change_status(repo: str, number: int, status: str) -> dict:
    issue = gh("api", f"repos/{repo}/issues/{number}")
    if "pull_request" in issue or issue["state"] != "open":
        raise RuntimeError("Status transitions require an open issue, not a pull request")
    labels = [label["name"] for label in issue["labels"] if not label["name"].startswith("status:")]
    labels.append(f"status:{status}")
    gh("api", f"repos/{repo}/issues/{number}", "--method", "PATCH", payload={"labels": labels})
    verified = gh("api", f"repos/{repo}/issues/{number}")
    if {label["name"] for label in verified["labels"]} != set(labels):
        raise RuntimeError("Status read-back mismatch")
    return {"issue": number, "status": status, "verified": True}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("bootstrap", help="Create only missing labels, milestones and ownership-marked issues")
    sub.add_parser("snapshot", help="Refresh the fallback snapshot from public GitHub issues")
    status = sub.add_parser("status", help="Replace only the status label and read it back")
    status.add_argument("number", type=int)
    status.add_argument("status", choices=STATUSES)
    args = parser.parse_args()
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    repo = manifest["repository"]
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo):
        parser.error("Invalid repository identifier")
    specs = manifest["issues"]
    if len({i["key"] for i in specs}) != len(specs) or len({i["title"] for i in specs}) != len(specs):
        parser.error("Roadmap keys and titles must be unique")
    if args.command == "bootstrap":
        result = bootstrap(repo, manifest)
    elif args.command == "status":
        if args.number < 1:
            parser.error("Issue number must be positive")
        result = change_status(repo, args.number, args.status)
    else:
        result = {"snapshot_issues": len(snapshot(repo)["issues"])}
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
