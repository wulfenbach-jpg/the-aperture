#!/usr/bin/env python3
"""Insert the slice-29 section into PR46's body and refresh the state bullets.

Deterministic string surgery on the PR body that was read back into
pr-body-before.md: the "Current state" bullets are replaced and one section is
inserted at a known anchor. Everything else is left byte-identical.
"""
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
# The head token the body quotes is the commit the section was written for; an
# explicit argument lets the script be re-run later and still reproduce the
# recorded body byte-for-byte.
HEAD = (
    sys.argv[1]
    if len(sys.argv) > 1
    else subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout.strip()
)
BEFORE = (HERE / "pr-body-before.md").read_text(encoding="utf-8")

OLD_STATE = """- Catalog **144 real cards**.

- Desired eight-list union **459 names = 132 supported + 327 missing** (per-list missing `55/0/51/48/21/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **21 missing names**.

- Last **behavior** change: slice 28, `Circle of the Moon Druid` (code head `38dc519`, reviewed and approved). Latest head: `6f58f75`, an evidence/doc-only follow-up that regenerates the slice-28 mutation logs at the reviewed bytes and discloses one ordering limit.
"""

NEW_STATE = """- Catalog **145 real cards**.

- Desired eight-list union **459 names = 133 supported + 326 missing** (per-list missing `55/0/51/48/20/52/52/53`). The union includes the five basic land types.

- **1 certified / 7 planned**; desired snapshots unchanged, no bundled replacements.

- Nearest list, Ayula: **20 missing names**.

- Last **behavior** change: slice 29, `Stuffed Bear` (code head `{head}`). Slice 28's `Circle of the Moon Druid` remains the previous increment.
""".format(head=HEAD[:7])

SLICE29 = """### Latest increment: slice 29 — Stuffed Bear (first animation of a noncreature permanent)

`Stuffed Bear` (`{2}` Artifact; Scryfall **MID 259**, immutable `4cd40276-016b-475f-906f-74d31dceaf70`, oracle `1402c116-194e-49ca-a8c1-09ca659c220c`, retrieved **2026-10-27T13:14:48Z**; both printings MID 259 / DBL 259 carry identical wording) prints "{2}: This artifact becomes a 4/4 green Bear artifact creature until end of turn." It is the pool's first permanent that is *not* a creature until an ability animates it, and one printed ability spans three layers: **layer 4** (CR 613.1d) *adds* the creature card type and the Bear subtype to the printed Artifact type, **layer 5** (CR 613.1e) sets the colour to green, and **layer 7b** (CR 613.4b) sets base power/toughness 4/4. All three are ordinary until-end-of-turn `ContinuousEffect`s, so cleanup ends them (CR 514.2) and departure prunes them (CR 400.7) through the existing pipeline.

The rules layer now asks `Game::is_creature` everywhere instead of reading the printed card type: an animated artifact can be targeted as a creature, attack, block, deal and receive damage, and die as a *creature* death (the death record captures the creature answer at the moment it leaves, so dies-watchers drain correctly). `Game::has_color` is the layered colour query with the printed colour as its baseline, and `CardDef::is_color`/`is_black` are its printed predicates. A permanent that prints an animation records CR 302.6 summoning sickness when it *arrives*, so an artifact animated on the turn it entered cannot attack, while one animated on a later turn can.

Verified: 9 focused slice-29 tests plus __FOCUSED__ focused tests across 22 suites, workspace **__SUITES__ suites / __PASSED__ passed / 0 failed**, fmt / `clippy -D warnings` / locked release build clean, four discriminating mutation REDs (layer 4 addition ignored, layer 5 ignored, layer 7b ignored, entry sickness dropped) each with `engine.rs` restored byte-for-byte, and a real four-seat 40-life package journey (`decks/package-slice29.txt`, terminal `Win{P3}`, __STEPS__ actions, __TURNS__ turns, digest `__DIGEST__`, separate CLI replay matches) whose driver asserts the animated-or-plain invariant after every accepted action (__ANIMATED__ animated and __PLAIN__ plain sightings, __ACTIVATIONS__ activations). This slice certifies no starter list: Ayula still has **20** missing names and the eight-list gate remains **1 of 8**.
"""


def main() -> int:
    assert BEFORE.count(OLD_STATE) == 1, "the current-state block is not at the expected head"
    text = BEFORE.replace(OLD_STATE, NEW_STATE)
    anchor = "### Latest increment: slice 28"
    assert text.count(anchor) == 1, "the slice-28 anchor is not unique"
    body = SLICE29
    for name, value in {
        "FOCUSED": "197",
        "SUITES": "115",
        "PASSED": "812",
        "STEPS": "1912",
        "TURNS": "44",
        "DIGEST": "ac16e510ca4691a69981beafcb577f3aa8f01bd6973710f86b89275899030024",
        "ANIMATED": "1301",
        "PLAIN": "3741",
        "ACTIVATIONS": "119",
    }.items():
        body = body.replace("__" + name + "__", value)
    text = text.replace(anchor, body + "\n" + anchor)
    (HERE / "pr-body-after.md").write_text(text, encoding="utf-8")
    print(f"wrote pr-body-after.md ({len(text)} bytes) for head {HEAD}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
