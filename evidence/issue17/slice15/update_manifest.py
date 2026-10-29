"""Remove the three supported slice-15 names from the Ayula missing list.

The manifest keeps one name per line; each of these names appears exactly once
in the whole file (they are only in Ayula's list), so a line-level removal keeps
every other byte — and therefore every other list — untouched. The companion
guard test `issue17_slice15_corpus.rs` recomputes the whole field from the
compiled pool afterwards, so this script cannot leave a stale entry behind.
"""

import io

PATH = "decks/starter-corpus/manifest.json"
NAMES = ["Ayula's Influence", "Bearscape", "Mother Bear"]

text = io.open(PATH, newline="").read()
for name in NAMES:
    needle = f'"{name}"'
    found = text.count(needle)
    if found != 1:
        raise SystemExit(f"{name}: expected exactly one occurrence, found {found}")
    lines = text.split("\n")
    kept = [line for line in lines if needle not in line]
    if len(kept) != len(lines) - 1:
        raise SystemExit(f"{name}: line removal was not exactly one line")
    text = "\n".join(kept)

with io.open(PATH, "w", newline="") as fh:
    fh.write(text)
print("removed", len(NAMES), "names from the Ayula missing list")
