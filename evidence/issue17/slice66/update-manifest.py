"""Rewrite the Krenko manifest gap fields for slice 66, then recount.

Both declared fields are rewritten *whole* — the card-name list is recomputed
from the deck file against the supported catalog, and the interaction text is
rewritten to describe the batch that now exists — so the two fields cannot
drift apart from each other or from the pool.
"""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CATALOG = (ROOT / "crates/core/src/catalog.rs").read_text(encoding="utf-8")
MANIFEST_PATH = ROOT / "decks/starter-corpus/manifest.json"

names = set()
for macro in (
    "card",
    "card_artifact",
    "card_defender",
    "card_flash",
    "card_green",
    "card_islandwalk",
    "card_trample",
    "card_vigilance_trample",
    "card_flying_haste",
):
    for match in re.finditer(rf"\b{macro}!\(\s*\w+,\s*\"([^\"]+)\"", CATALOG):
        names.add(match.group(1).lower())
assert len(names) == 179, len(names)

manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
krenko = next(s for s in manifest["starters"] if s["id"] == "krenko-goblin-tokens")

deck = []
for line in (ROOT / krenko["deck_path"]).read_text(encoding="utf-8").splitlines():
    line = line.strip()
    if not line or line.startswith("#") or line.startswith("//"):
        continue
    match = re.match(r"^(\d+)\s+(.+)$", line)
    if match:
        deck.extend([match.group(2).strip()] * int(match.group(1)))
assert len(deck) == 99, len(deck)

deck_names = list(dict.fromkeys(deck + [krenko["commander"]]))
missing = sorted(
    {name for name in deck_names if name.lower() not in names}, key=str.lower
)
krenko["missing_cards"] = missing
krenko["missing_interactions"] = [
    f"remaining {len(missing)} desired-list cards are not in the supported pool"
    + (
        " (first unsupported names: " + ", ".join(missing[:6]) + ")"
        if missing
        else ""
    ),
    "goblin tribal payoffs still unsupported: the remaining sacrifice-a-Goblin"
    " costs (Skirk Prospector's nonland mana ability and Siege-Gang Lieutenant's"
    " Lieutenant combat trigger are the two remaining members of this family,"
    " so this entry lists only the cards still outside the pool), library search"
    " onto the battlefield (Moggcatcher), goblin-count-scaled combat payoffs"
    " (Grenzo, Howlsquad Heavy, General Kreat), and artifact/graveyard Goblin"
    " recursion (Goblin Engineer, Goblin Welder, Squee)",
    "tokens outside the typed 1/1 red Goblin package; the three-token enters"
    " package, the two-token sacrifice payoff and the artifact-destruction"
    " sacrifice of this family are now implemented, so what remains under this"
    " heading is the other token-making cards named above",
]
assert len(missing) == 36, len(missing)
MANIFEST_PATH.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
print(f"krenko missing_cards = {len(missing)}")
print(f"first missing        = {missing[:6]}")
print(f"interactions         = {len(krenko['missing_interactions'])}")
