"""Update PR46 body: current-state block + slice 23 section, preserving history."""
import json
import pathlib
import subprocess

root = pathlib.Path("C:/AI/aperture-worktrees/issue-17")
body = subprocess.run(
    ["gh", "pr", "view", "46", "--json", "body", "--jq", ".body"],
    cwd=root,
    capture_output=True,
    text=True,
    check=True,
).stdout.replace("\r\n", "\n")

old_state = """- Catalog **139 real cards**.

- Desired eight-list union **459 names = 127 supported + 332 missing** (per-list missing `55/0/51/48/26/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **26 missing names**."""
new_state = """- Catalog **140 real cards**.

- Desired eight-list union **459 names = 128 supported + 331 missing** (per-list missing `55/0/51/48/25/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **25 missing names**."""
assert old_state in body, "current-state block not found verbatim"
body = body.replace(old_state, new_state)

slice23 = """
### Latest increment: slice23 — Goreclaw, Terror of Qal Sisma (cost reduction + granted trample)

- Card: **Goreclaw, Terror of Qal Sisma**, `{3}{G}` Legendary Creature — Bear 4/3. Scryfall **CMM 293**, immutable `ee47f23a-3ba9-4615-b170-c89d8ab99d78`, oracle `befb211f-37ca-4083-98d4-9ff1f28be3f2`, retrieved `2026-10-27T07:38:22Z`; raw collection bytes `evidence/issue17/slice23/raw/cards-collection.json`, normalized record `fixtures/starter-corpus/slice23-sourced.json`, catalog text byte-equal to that printing.
- Printed behavior: "Creature spells you cast with power 4 or greater cost {2} less to cast." / "Whenever Goreclaw attacks, each creature you control with power 4 or greater gets +1/+1 and gains trample until end of turn."
- New reusable surface: `StaticAbility::CreatureSpellsYouCastWithPowerCostLess` applied inside `effective_cost` after commander tax — **generic part only, never below zero** (CR 601.2f, 118.7a), controller's own creature spells only; `Layer::Ability` (CR 613.4d) with `ContinuousKind::GrantsTrample`, read by combat damage assignment through `Game::has_trample`; `Effect::BoostAndTrampleEachCreatureYouControlWithPowerAtLeast`, whose power filter reads **current** power at resolution. Both grants are ordinary `ContinuousEffect` rows, so cleanup expiry and battlefield-departure invalidation come from the existing pipeline (the slice-21 stale-shield defect is not reproduced).
- Three discriminating **mutation REDs**, each failing exactly its target assertions and the engine restored byte-for-byte (sha256 before == after): cost reduction disabled → exactly the 2 cost tests fail; printed-power threshold ignored → exactly the qualifying-creature test fails; grant lookup dropped from `has_trample` → exactly the trample combat test fails (`evidence/issue17/slice23/red-mutation-*.log`).
- Executed: `issue17_slice23` **11/11**, plus `card_printings` 8, `catalog_pool` 5, `issue13_commander_combat` 43, `issue17_slice15_corpus` 2, `abilities` 13, `issue17_slice20` 9, `issue17_slice21` 8, `issue17_slice22` 9, `projection` 4, `timing_mana` 12, `m1_multiplayer` 28; `cargo test --workspace --all-targets --locked --no-fail-fast` exit 0 (108 suites); `fmt --check`, workspace `clippy -D warnings`, locked release build exit 0 (`evidence/issue17/slice23/native-gates-at-632c5e7.log`, `workspace-tests.log`).
- One real **4-seat / 40-life / 40-card package** journey on the exact `decks/package-slice23.txt` snapshot (`sha256 5f933f49…e71fb7`), seed 2323: terminal **Win{P3}**, **1322 actions / 32 turns / 3495 replay events**, digest `e2ccc113bceb…`; separate release-CLI replay reproduces it and a fresh save roundtrips. Goreclaw cast 5×, its attack trigger staged 12×, 45 trample grants and 45 pumps started. Evidence-only masked demonstrator, **not product AI or strength evidence**; Jasmine Boreal injected into all four command zones; **not** a 100-card certificate.
- Legacy digests unchanged: slice 17–22 demo replays still verify (`85fabffc`, `09e6bb25`, `5fd841b2`, `72cd69fb`, `0d20c759`, `17f03348`).
- Correction disclosed: my first test cut asserted Craw Wurm as a 6/6 and predicted 35 life; the card is a printed **6/4** and **both** attackers trample (3 + 5 = 8), so 32 is correct. The engine was right and the tests were corrected — no engine change was made for that.
- Not claimed: any second certified starter, playable Commander, or AI strength. The cost reduction reads printed power (no supported effect changes a card's power outside the battlefield), and `view` does not display granted keywords. **Full original #17 acceptance stays open: 1 of 8 certified, Ayula 25 names short.**
"""

marker = "\n### Latest increment: slice21"
if marker in body:
    body = body.replace(marker, "\n### Historical increment: slice21", 1)
anchor = "\n### Historical increment: slice21"
assert anchor in body
body = body.replace(anchor, slice23 + anchor, 1)

out = root / "evidence/issue17/slice23/pr-body.md"
out.write_text(body, encoding="utf-8", newline="")
print(len(body.splitlines()), "lines written")
