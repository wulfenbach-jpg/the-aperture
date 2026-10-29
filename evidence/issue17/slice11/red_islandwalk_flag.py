"""Genuine RED for the `islandwalk` CardDef flag.

Flips `islandwalk: true` to false in the dedicated macro only, so the pool guard
must reject the two cards as "prints text but has no implementation" while
everything else stays untouched.
"""

import io
import shutil
import subprocess
import sys

path = "crates/core/src/catalog.rs"
backup = path + ".red-backup"
shutil.copyfile(path, backup)

raw = io.open(path, encoding="utf-8", newline="").read()
assert raw.count("islandwalk: true,") == 1
io.open(path, "w", encoding="utf-8", newline="").write(
    raw.replace("islandwalk: true,", "islandwalk: false,")
)

result = subprocess.run(
    ["cargo", "test", "-p", "aperture-core", "--test", "catalog_pool", "--test", "issue17_slice11", "--locked"],
    capture_output=True,
    text=True,
)
print(result.stdout[-3000:])

shutil.copyfile(backup, path)
import os

os.remove(backup)
io.open("evidence/issue17/slice11/red-before-islandwalk-flag.log", "w", encoding="utf-8", newline="\n").write(
    result.stdout + result.stderr
)
print("RED exit code:", result.returncode)
sys.exit(0)
