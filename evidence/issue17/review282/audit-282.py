"""Independent review-282 corpus audit.

Deliberately NOT the implementer's script: this one parses the catalog with a
different rule (any `name!(Id, "Name"` macro form) and cross-checks the parsed
catalog against the built CLI's own `pool` output, then recomputes the
eight-list union from the desired deck files. No hardcoded expectations: every
number printed is derived from the sources.
"""
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(r"C:\AI\aperture-worktrees\issue-17")
CATALOG = (ROOT / "crates/core/src/catalog.rs").read_text(encoding="utf-8")
MANIFEST = json.loads((ROOT / "decks/starter-corpus/manifest.json").read_text(encoding="utf-8"))

# Any macro invocation whose first argument is an identifier and second a name.
names = set()
for m in re.finditer(r"\b([a-z_]+)!\s*\(\s*[A-Za-z_][A-Za-z0-9_]*\s*,\s*\"([^\"]+)\"", CATALOG):
    names.add(m.group(2).lower())
print("catalog_parse_names =", len(names))

# Cross-check against product truth: the CLI's implemented list.
cli = ROOT / "target/release/aperture-cli.exe"
pool_out = subprocess.run([str(cli), "pool"], capture_output=True, text=True)
print("cli_pool_exit =", pool_out.returncode)
pool_lines = [l.strip() for l in pool_out.stdout.splitlines()]
pool_names = {l.lower() for l in pool_lines if l and not l[0].isdigit()}
print("cli_pool_lines =", len(pool_lines), "cli_pool_names =", len(pool_names))
print("cli_sample =", pool_lines[:4])

union = set()
report = []
for starter in MANIFEST["starters"]:
    path = ROOT / starter["deck_path"]
    names_in_deck = []
    for line in path.read_text(encoding="utf-8").splitlines():
        s = line.strip()
        if not s or s.startswith("#") or s.startswith("//"):
            continue
        m = re.match(r"^(\d+)\s+(.+?)\s*$", s)
        if m:
            names_in_deck.extend([m.group(2)] * int(m.group(1)))
    total = len(names_in_deck) + (1 if starter.get("commander") else 0)
    uniq = list(dict.fromkeys(names_in_deck + ([starter["commander"]] if starter.get("commander") else [])))
    union.update(n.lower() for n in uniq)
    missing = sorted({n for n in uniq if n.lower() not in names}, key=str.lower)
    declared = sorted(starter.get("missing_cards", []), key=str.lower)
    agree = missing == declared
    report.append((starter["id"], starter.get("status"), total, len(missing), agree))
    if not agree:
        print("  MISMATCH", starter["id"],
              "only_computed=", sorted(set(missing) - set(declared)),
              "only_manifest=", sorted(set(declared) - set(missing)))

union_supported = sum(1 for n in union if n in names)
print("union_names        =", len(union))
print("union_supported    =", union_supported)
print("union_missing      =", len(union) - union_supported)
for sid, status, total, missing, agree in report:
    print(f"  {sid:28s} status={str(status):10s} deck_cards={total:3d} missing={missing:3d} manifest_agrees={agree}")
print("per_list_missing   =", [r[3] for r in report])
print("certified_lists    =", sum(1 for r in report if r[1] == "certified"),
      [r[0] for r in report if r[1] == "certified"])
print("catalog_vs_cli_delta_only_in_parse =", sorted(names - pool_names)[:5], "count", len(names - pool_names))
print("catalog_vs_cli_delta_only_in_cli   =", sorted(pool_names - names)[:5], "count", len(pool_names - names))
