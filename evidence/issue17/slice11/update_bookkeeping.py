"""Slice-11 bookkeeping: drop the two now-supported cards from the manifest's
Ayula gap list and re-state the corpus totals in the docs.

Both edits are derived from the native audit (`corpus_audit.py`), never typed by
hand: the names removed are exactly the ones the audit now reports supported.
"""

import io
import json
import subprocess
from pathlib import Path

audit = json.loads(Path("evidence/issue17/slice11/corpus-audit.json").read_text(encoding="utf-8"))
newly_supported = ["Pale Bears", "River Bear"]

manifest_path = Path("decks/starter-corpus/manifest.json")
raw = io.open(manifest_path, encoding="utf-8", newline="").read()
manifest = json.loads(raw)
entry = next(s for s in manifest["starters"] if s["id"] == "ayula-bear-counters")
before = len(entry["missing_cards"])
entry["missing_cards"] = [name for name in entry["missing_cards"] if name not in newly_supported]
after = len(entry["missing_cards"])
assert after == before - 2, (before, after)
io.open(manifest_path, "w", encoding="utf-8", newline="\n").write(
    json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"
)
print(f"manifest ayula missing_cards: {before} -> {after}")

docs = Path("docs/issue17-starter-corpus.md")
text = io.open(docs, encoding="utf-8", newline="").read()
replacements = [
    (
        "* **102** are in the supported 114-card catalog and **357** are missing;",
        "* **104** are in the supported 116-card catalog and **355** are missing;",
    ),
    (
        "114-card pool. Isamaru itself is supported, and the slice-4 package implemented",
        "116-card pool. Isamaru itself is supported, and the slice-4 package implemented",
    ),
    (
        "1. **Card-support surface.** The implemented pool is 114 cards. Non-vanilla",
        "1. **Card-support surface.** The implemented pool is 116 cards. Non-vanilla",
    ),
]
for old, new in replacements:
    assert text.count(old) == 1, old
    text = text.replace(old, new)

# Record slice 11 next to the slices 8-9 paragraph so the doc cannot drift from
# the audit it cites.
anchor = "Slices 8–9 currently support eight sourced cards:"
assert text.count(anchor) == 1
insert = (
    "Slice 11 adds the two islandwalk bears from the Ayula list, `Pale Bears` and\n"
    "`River Bear`, and models CR 702.14b from the defending player's board. That\n"
    "leaves `ayula-bear-counters` with "
    + str(after)
    + " of its names missing.\n\n"
)
text = text.replace(anchor, insert + anchor)
io.open(docs, "w", encoding="utf-8", newline="\n").write(text)
print("docs updated")
