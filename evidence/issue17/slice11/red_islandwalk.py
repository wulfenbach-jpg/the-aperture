"""Genuine RED for the islandwalk enforcement, then restore.

Removes only the CR 702.14b branch from `tokens::can_block` (the catalog entries
and the keyword table stay), so any failure is caused by the missing enforcement
and not by a missing card.
"""

import io
import shutil
import subprocess
import sys

path = "crates/core/src/tokens.rs"
backup = path + ".red-backup"
shutil.copyfile(path, backup)

raw = io.open(path, encoding="utf-8", newline="").read()
needle = (
    "    if attacker.contains(&Keyword::Islandwalk) && defender_controls_island {\r\n"
    "        return false;\r\n"
    "    }\r\n"
    "    if !attacker.contains(&Keyword::Flying) {\r\n"
)
replacement = "    if !attacker.contains(&Keyword::Flying) {\r\n"
assert raw.count(needle) == 1, "the branch to remove must appear exactly once"
io.open(path, "w", encoding="utf-8", newline="").write(raw.replace(needle, replacement))

result = subprocess.run(
    ["cargo", "test", "-p", "aperture-core", "--test", "issue17_slice11", "--locked"],
    capture_output=True,
    text=True,
)
print(result.stdout[-4000:])
print(result.stderr[-2000:])

shutil.copyfile(backup, path)
import os

os.remove(backup)
io.open("evidence/issue17/slice11/red-before-islandwalk-enforcement.log", "w", encoding="utf-8", newline="\n").write(
    result.stdout + result.stderr
)
print("RED exit code:", result.returncode)
sys.exit(0)
