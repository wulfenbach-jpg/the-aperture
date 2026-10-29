import pathlib
import subprocess

before = pathlib.Path("evidence/issue17/slice31/pr-body-before.md").read_text(encoding="utf-8")
s = before


def sub(old, new, expect=1):
    global s
    n = s.count(old)
    if n != expect:
        raise SystemExit(f"expected {expect} of {old[:70]!r}, found {n}")
    s = s.replace(old, new)


sub("- Catalog **146 real cards**.", "- Catalog **147 real cards**.")
sub(
    "- Desired eight-list union **459 names = 134 supported + 325 missing** (per-list missing `55/0/51/48/19/52/52/53`).",
    "- Desired eight-list union **459 names = 135 supported + 324 missing** (per-list missing `55/0/51/48/18/52/52/53`).",
)
sub("- Nearest list, Ayula: **19 missing names**.", "- Nearest list, Ayula: **18 missing names**.")
sub(
    "- Last **behavior** change: slice 30, `Drover Grizzly` (code head `dea6fab`). `Stuffed Bear` (slice 29) remains the previous behavior increment; the two follow-up commits are docs/evidence only.",
    "- Last **behavior** change: slice 31, `Beorn the Fierce` (code head `e53b987`). `Drover Grizzly` (slice 30) remains the previous behavior increment.",
)

SLICE31 = """### Latest increment: slice 31 — Beorn the Fierce (HOB 119)

`Beorn the Fierce` (`{3}{G}{G}`, 6/6 Legendary Creature — Bear Shapeshifter Warrior; Scryfall **HOB 119**, immutable `367d5f8b-77ee-47f7-bc71-972d62c280a9`, oracle `30ca8e92-5955-46a1-86c1-094a873f518f`, retrieved **2026-10-27T15:33:31Z**) prints "Trample / Other Bears you control get +2/+2. / At the beginning of combat on your turn, put a trample counter on up to one target creature you control. It becomes a Bear in addition to its other types. Then if you control three or more Bears, draw two cards." (fixture `fixtures/starter-corpus/slice31-sourced.json`, enforced by the provenance guard in `crates/core/tests/card_printings.rs`).

Reusable machinery, not a one-card patch:

* `TriggerEvent::BeginningOfCombat` (CR 507.1) is dispatched once when the beginning-of-combat step starts, only for the active player's permanents, before any player gets priority; nothing is stored, so it cannot double-fire.
* `TargetSpec::UpToOneCreatureYouControl` + `Action::DeclineTriggerTarget` (CR 115.6) make "up to one target" an honest choice: the engine asks whenever a legal target exists — including exactly one — and always offers "choose no target". Declining still resolves the ability, so the conditional draw is unaffected; decline stays illegal for mandatory-target triggers (Ayula, Little Bear, Wolfbear).
* `CounterKind::Trample` is the pool's first keyword counter (CR 122.1b). `Game::has_trample` reads it, cleanup never removes it, and the permanent keeps trample after the turn ends — the subtype does not.
* `Effect::AddTargetCreatureType` starts a layer 4 *additive* subtype effect on the chosen target (CR 613.1d); because the ability prints no duration it lasts until end of turn (CR 611.2a) and cleanup ends it.
* `StaticAbility::OtherBearsYouControlGet` is a layer 7c subtype anthem (CR 613.4c) whose recipients are computed from the **effective** subtype — a creature a layer 4 effect just made a Bear receives it — and which never pumps its own source.
* `Effect::DrawIfControlsBears` evaluates the Bear count after the earlier instructions of the same resolution (CR 608.2c), so the creature this ability just turned into a Bear counts toward the three.

Verification: 8/8 focused slice-31 tests; four discriminating mutation REDs (trample-counter read dropped, Bear-count gate ignored, optional target made mandatory, subtype anthem disabled) with `engine.rs` restored byte-for-byte; workspace **117 suites / 827 passed / 0 failed**; fmt, `clippy -D warnings` and locked release build clean; a real four-seat/40-life exact-snapshot journey (`decks/package-slice31.txt`, seed 3131, terminal `Win{P2}`, 1917 actions, 46 turns, 4930 replay events, digest `36e18185…`, 13 triggers, 13 trample counters, 13 Bear additions) with a separate release CLI replay matching; and 10/10 exact-head CI SUCCESS all keyed to `e53b987`. Independent review: **APPROVED_INCREMENT**, no functional defect (`evidence/issue17/review287/REVIEW287.md`).

Disclosed limits: the journey is a 40-card package, not a 100-card starter certificate, and the driver is an evidence-only masked demonstrator, not product AI or strength evidence. Newly discovered and **not** fixed here: `Surrak and Goreclaw` prints its own Trample but its catalog row is a plain `card!` (trample `false`) and its static grants trample only to other creatures, so Surrak itself does not trample; repairing an approved card's behavior would also move the digests of the journeys that used it, so it is recorded for a separately reviewed increment. This slice certifies no starter list: Ayula still has **18** missing names and the eight-list gate remains **1 of 8**.

### Previous increment: slice 30 — Drover Grizzly and the pool's first saddle
"""

sub(
    "### Latest increment: slice 30 — Drover Grizzly and the pool's first saddle\n",
    SLICE31,
)

pathlib.Path("evidence/issue17/slice31/pr-body-after.md").write_text(s, encoding="utf-8")
result = subprocess.run(
    [
        "gh",
        "pr",
        "edit",
        "46",
        "--body-file",
        "evidence/issue17/slice31/pr-body-after.md",
    ],
    capture_output=True,
    text=True,
    encoding="utf-8",
    timeout=120,
)
print("edit exit", result.returncode, result.stdout.strip(), result.stderr.strip())
if result.returncode != 0:
    raise SystemExit(result.returncode)
