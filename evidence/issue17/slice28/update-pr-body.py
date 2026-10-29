#!/usr/bin/env python3
"""Insert the slice-28 section into PR46's body and refresh the state bullets.

Deterministic string surgery on the PR body that was read back into
pr-body-before.md: the "Current state" bullets are replaced and one section is
inserted at a known anchor. Everything else is left byte-identical, which
pr-body-readback-diff.txt proves.
"""
import pathlib
import subprocess

HERE = pathlib.Path(__file__).resolve().parent
BEFORE = (HERE / "pr-body-before.md").read_text(encoding="utf-8")

OLD_STATE = """- Catalog **143 real cards**.

- Desired eight-list union **459 names = 131 supported + 328 missing** (per-list missing `55/0/51/48/22/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **22 missing names**.

- Last **behavior** change: slice 26, `Ursine Champion` (code head `9cd0b68`). Latest head: slice 27 record repair (`27d8f48`), comment-only source edits, no behavior change.
"""

NEW_STATE = """- Catalog **144 real cards**.

- Desired eight-list union **459 names = 132 supported + 327 missing** (per-list missing `55/0/51/48/21/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **21 missing names**.

- Last **behavior** change: slice 28, `Circle of the Moon Druid` (code head `38dc519`, reviewed and approved). Latest head: `6f58f75`, an evidence/doc-only follow-up that regenerates the slice-28 mutation logs at the reviewed bytes and discloses one ordering limit.
"""

SLICE28 = """### Latest increment: slice 28 — Circle of the Moon Druid (first turn-conditioned printed ability)

`Circle of the Moon Druid` (`{2}{G}`, printed 2/4 Creature — Human Elf Druid; Scryfall **AFR 177**, immutable `700c9ed8-6bfe-4f1e-855a-5f31e0d2fc9b`, retrieved **2026-10-27T12:07:11Z**, both printings AFR 177 / HBG 204 identical) prints "Bear Form — During your turn, this creature is a Bear with base power and toughness 4/2. (It loses all other creature types.)" It is the pool's first printed continuous ability conditioned on whose turn it is (CR 611.3a), and one printed ability spans two layers: **layer 4** (CR 613.1d) replaces the printed subtype line, and **layer 7b** (CR 613.4b) sets base power/toughness 4/2. Neither half is a stored continuous effect — the condition is answered when the characteristic is read, so nothing expires in cleanup and no effect object can be orphaned by a bounce or a reload. `Game::has_subtype` now answers the printed line before effects, so Little Bear's Bear branch, Ayula's Bear offer lists and Exuberant Wolfbear's "target Human you control" all read the effective subtype. `enter_battlefield` recomputes characteristics when the entering card prints a layer 4/7b static; additive printed 7c statics keep their existing recompute points (disclosed debt, not changed here).

Verified: 5 focused slice-28 tests plus 172 focused tests across 18 suites, workspace **114 suites / 803 passed / 0 failed**, fmt / `clippy -D warnings` / locked release build clean, three discriminating mutation REDs with `engine.rs` restored byte-for-byte, a real four-seat 40-life package journey (`decks/package-slice28.txt`, terminal `Win{P3}`, 2065 actions, 50 turns, digest `48bf3ae5…`, separate CLI replay matches) whose driver asserts the printed 4/2-or-2/4 invariant after every accepted action, and an independent read-only review (**APPROVED**, `evidence/issue17/review283/REVIEW283.md`). CI on `6f58f75`: **10/10 check runs SUCCESS keyed to that head**, including the pull_request-triggered run (the earlier `crates/ai/tests/review.rs` merge-ref failure was master drift and is fixed on master by `73820d7`). Checkpoint `evidence/issue17/SLICE28-HUMAN-CHECKPOINT.md`.

"""

if OLD_STATE not in BEFORE:
    raise SystemExit("current-state block not found; refusing to edit the PR body")
body = BEFORE.replace(OLD_STATE, NEW_STATE)

anchor = "### Latest increment: slice 27"
if body.count(anchor) != 1:
    raise SystemExit("slice-27 anchor not unique")
body = body.replace(anchor, SLICE28 + anchor)

path = HERE / "pr-body-after.md"
path.write_text(body, encoding="utf-8")
subprocess.run(["gh", "pr", "edit", "46", "--body-file", str(path)], check=True)
readback = subprocess.run(
    ["gh", "pr", "view", "46", "--json", "body", "--jq", ".body"],
    capture_output=True,
    text=True,
    check=True,
).stdout
(HERE / "pr-body-readback.md").write_text(readback, encoding="utf-8")
print("body updated; readback equal:", readback == body)
