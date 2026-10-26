#!/usr/bin/env python
"""Bounded credential/secret scan for THE APERTURE (#31 security lane).

Scans the tracked working tree and the reachable git history of THIS
repository only, for high-signal secret patterns. Findings are REDACTED:
at most a 3-character prefix and the matched length are printed, never a
full credential value.

Usage:
  python scripts/security/scan_credentials.py [--history] [--json PATH]

Exit codes: 0 = no findings, 1 = review-tier findings only, 2 = secret-tier
findings. Triage happens in the audit report; this tool only evidences.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

# Secret-tier: a match is almost certainly a live credential.
SECRET_PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    ("pem-private-key", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY( HEADER)?-----")),
    ("github-token", re.compile(r"\bgh[pousr]_[A-Za-z0-9]{16,}\b")),
    ("github-pat", re.compile(r"\bgithub_pat_[A-Za-z0-9_]{20,}\b")),
    ("aws-access-key", re.compile(r"\bAKIA[0-9A-Z]{16}\b")),
    ("aws-secret-shape", re.compile(r"(?i)aws(.{0,20})?secret.access.key\W+[A-Za-z0-9/+=]{30,}")),
    ("slack-token", re.compile(r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b")),
    ("openai-key", re.compile(r"\bsk-[A-Za-z0-9_-]{20,}\b")),
    ("anthropic-key", re.compile(r"\bsk-ant-[A-Za-z0-9_-]{20,}\b")),
    ("google-api-key", re.compile(r"\bAIza[0-9A-Za-z_\-]{35}\b")),
    ("private-key-file-marker", re.compile(r"(?i)^\[rsa|ssh-rsa\s+AAAA[A-Za-z0-9+/=]{100,}")),
]

# Review-tier: needs human triage (docs examples, prose, placeholders).
REVIEW_PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    ("bearer-string", re.compile(r"(?i)\bbearer\s+[A-Za-z0-9._\-]{20,}")),
    (
        "credential-assignment",
        re.compile(r"(?i)\b(password|passwd|api[_-]?key|secret|token|credential)\b\s*[:=]\s*[\"']?[^\s\"']{12,}"),
    ),
    ("authorization-header", re.compile(r"(?i)authorization:\s*\S{8,}")),
]

# Files whose very presence in history is reportable regardless of content.
SENSITIVE_FILENAMES = re.compile(
    r"(?i)(^|/)(\.env(\..*)?|id_rsa(_[A-Za-z0-9]+)?|.*\.pem|.*\.pfx|.*\.p12|.*\.keystore|credentials\.json|secrets?\.(json|ya?ml|txt))$"
)

SKIP_DIRS = {".git", "target", "node_modules", ".hermes"}
MAX_FILE_BYTES = 1_000_000


def redact(value: str) -> str:
    value = value.strip()
    if len(value) <= 3:
        return f"<redacted len={len(value)}>"
    return f"{value[:3]}...<redacted len={len(value)}>"


def is_probably_binary(data: bytes) -> bool:
    return b"\x00" in data[:8192]


def scan_text(where: str, text: str, findings: list[dict]) -> None:
    for lineno, line in enumerate(text.splitlines(), start=1):
        for name, pattern in SECRET_PATTERNS:
            m = pattern.search(line)
            if m:
                findings.append(
                    {"tier": "secret", "pattern": name, "where": where, "line": lineno,
                     "evidence": redact(m.group(0))}
                )
        for name, pattern in REVIEW_PATTERNS:
            m = pattern.search(line)
            if m:
                findings.append(
                    {"tier": "review", "pattern": name, "where": where, "line": lineno,
                     "evidence": redact(m.group(0))}
                )


def tracked_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z"], capture_output=True, check=True
    ).stdout
    return [f for f in out.decode("utf-8", "replace").split("\0") if f]


def scan_worktree(findings: list[dict]) -> None:
    for path in tracked_files():
        p = Path(path)
        if SENSITIVE_FILENAMES.search(path):
            findings.append({"tier": "secret", "pattern": "sensitive-filename",
                             "where": path, "line": 0, "evidence": "<name>"})
        if any(part in SKIP_DIRS for part in p.parts):
            continue
        try:
            raw = p.read_bytes()
        except OSError:
            continue
        if len(raw) > MAX_FILE_BYTES or is_probably_binary(raw):
            continue
        scan_text(f"worktree:{path}", raw.decode("utf-8", "replace"), findings)


def scan_history(findings: list[dict]) -> None:
    """Scan every object in the repository (reachable or not) exactly once,
    with content/length bounds and full redaction. Path attribution uses the
    first path each blob was seen at."""
    listed = subprocess.run(
        ["git", "rev-list", "--objects", "--all"], capture_output=True, check=True
    ).stdout.decode("utf-8", "replace").splitlines()
    blob_paths: dict[str, str] = {}
    for line in listed:
        parts = line.split(" ", 1)
        if len(parts) == 2 and parts[0]:
            blob_paths.setdefault(parts[0], parts[1])

    # Stream every object in the repository without an stdin pipe: a piped
    # request list deadlocks once the request text outgrows the pipe buffer.
    # --batch-all-objects also covers unreachable blobs (deleted secrets).
    proc = subprocess.Popen(
        ["git", "cat-file", "--batch-all-objects", "--batch"],
        stdout=subprocess.PIPE,
    )
    out = proc.stdout
    assert out is not None
    while True:
        header = out.readline().decode("utf-8", "replace").strip()
        if not header:
            break  # end of stream
        fields = header.split()
        if len(fields) != 3:
            break  # unexpected framing
        size = int(fields[2])
        if size > MAX_FILE_BYTES:
            out.read(size + 1)
            continue
        data = out.read(size)
        out.read(1)  # trailing newline
        oid = fields[0]
        if fields[1] != "blob":
            continue
        if is_probably_binary(data):
            continue
        path = blob_paths.get(oid, "<unreachable-or-ambiguous>")
        if SENSITIVE_FILENAMES.search(path):
            findings.append({"tier": "secret", "pattern": "sensitive-filename-in-history",
                             "where": f"blob {oid[:12]}:{path}", "line": 0,
                             "evidence": "<name>"})
            continue
        scan_text(f"history:{oid[:12]}:{path}", data.decode("utf-8", "replace"), findings)
    proc.wait()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--history", action="store_true", help="also scan all reachable blobs")
    ap.add_argument("--json", help="write the raw findings JSON here")
    args = ap.parse_args()

    findings: list[dict] = []
    scan_worktree(findings)
    if args.history:
        scan_history(findings)

    if args.json:
        Path(args.json).write_text(json.dumps(findings, indent=2), encoding="utf-8")

    secret = [f for f in findings if f["tier"] == "secret"]
    review = [f for f in findings if f["tier"] == "review"]
    print(f"findings: {len(secret)} secret-tier, {len(review)} review-tier")
    for f in findings[:200]:
        print(f"[{f['tier']:6}] {f['pattern']}: {f['where']}:{f['line']} {f['evidence']}")
    if secret:
        return 2
    if review:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
