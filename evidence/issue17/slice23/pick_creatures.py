"""List pool creatures with power >= 4 and their printed costs (slice-23 test picking)."""
import re
from pathlib import Path

src = Path("C:/AI/aperture-worktrees/issue-17/crates/core/src/catalog.rs").read_text(encoding="utf-8")
pool = src.partition("pub static TOKENS")[0]
entries = re.findall(
    r"card!\(\s*[A-Za-z0-9_]+,\s*\"([^\"]+)\",\s*cost:\s*(None|Some\(\"([^\"]*)\"\)),\s*"
    r"kinds:\s*([A-Z_]+),\s*basic:\s*[A-Za-z:()\.\s]+?,\s*"
    r"type_line:\s*\"([^\"]*)\",\s*power:\s*(None|Some\((-?\d+)\))",
    pool,
    re.S,
)
rows = []
for name, _costfull, cost, kinds, tl, _pfull, power in entries:
    if "CREATURE" in kinds and power:
        rows.append((int(power), name, cost or "", tl))
for power, name, cost, tl in sorted(rows, reverse=True):
    print(f"{power:3d}  {name:34s} {cost:14s} {tl}")
