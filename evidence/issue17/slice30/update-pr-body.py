import pathlib
import subprocess

before = pathlib.Path("evidence/issue17/slice30/pr-body-before.md").read_text(encoding="utf-8")
s = before

def sub(old, new, expect=1):
    global s
    n = s.count(old)
    if n != expect:
        raise SystemExit(f"expected {expect} of {old[:70]!r}, found {n}")
    s = s.replace(old, new)

sub("- Catalog **145 real cards**.", "- Catalog **146 real cards**.")
sub(
    "- Desired eight-list union **459 names = 133 supported + 326 missing** (per-list missing `55/0/51/48/20/52/52/53`).",
    "- Desired eight-list union **459 names = 134 supported + 325 missing** (per-list missing `55/0/51/48/19/52/52/53`).",
)
sub("- Nearest list, Ayula: **20 missing names**.", "- Nearest list, Ayula: **19 missing names**.")
sub(
    "- Last **behavior** change: slice 29, `Stuffed Bear` (code head `99785d7`). Slice 28's `Circle of the Moon Druid` remains the previous increment.",
    "- Last **behavior** change: slice 30, `Drover Grizzly` (code head `dea6fab`). `Stuffed Bear` (slice 29) remains the previous behavior increment; the two follow-up commits are docs/evidence only.",
)

SLICE30 = """### Latest increment: slice 30 — Drover Grizzly and the pool's first saddle

`Drover Grizzly` (`{2}{G}`, 4/2 Creature — Bear Mount; Scryfall **OTJ 161**, immutable `560062cd-34f8-4d30-9e25-099b03961724`, retrieved **2026-10-27T14:40:04Z**) prints "Whenever this creature attacks while saddled, creatures you control gain trample until end of turn." and "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)" (CR 702.171a-c; pinned rules text `evidence/issue17/slice2-rules-source.txt` lines 5280-5286).

This is reusable machinery rather than a one-card patch: `Action::SaddleMount { source, creatures }` types the cost as *other untapped creatures you control* (summoning-sick helpers legal, source excluded, duplicates/opponents/tapped/insufficient power refused, helpers tap on payment), the ability resolves through the ordinary stack into a `Saddled` designation effect that cleanup ends and battlefield departure prunes, `TriggerCondition::SourceIsSaddled` types the printed intervening condition so an unsaddled attack produces no trigger at all, and resolution grants trample to every creature controlled *then* through the existing layer-6 grants with captured source identity. The saddle stack item carries `affected: Some(source)`, so a source that leaves resolves with no effect and no transfer to a later same-ID incarnation. All new rules sites ask `Game::is_creature`, so an animated Stuffed Bear can saddle and receive trample while a plain artifact cannot.

Native inspection of the first cut found two real defects (printed-only creature queries in the new rules; unbound saddle stack item) and returned them to the same writer, repair `dea6fab` — disclosed as GREEN-strengthening test additions, not strict TDD. Verification: 61 focused tests / 0 failed with a committed `run-native.sh` that prints every exit code (fmt, workspace Clippy `-D warnings`, locked release build, journey, separate replay, audit all `=0`), a real four-seat/40-life package journey on exact snapshots (`decks/package-slice30.txt`, terminal `Win{P1}`, 2653 actions, 55 turns, 6968 replay events, digest `b711d966…5677`, 9 Drover casts, 116 offered saddle activations, 22 trample grants) with a separate CLI replay matching, and an independent review **APPROVED_INCREMENT** in the same original reviewer history (`evidence/issue17/slice30/REVIEW30-ORIGINAL-HISTORY.md`). Disclosed limit: saddle *offers* enumerate at most 10 eligible helpers, so above that cap no saddle action is offered although direct validation accepts a legal set — a bounded-practice limitation, not whole-card conformance. This slice certifies no starter list: Ayula still has **19** missing names and the eight-list gate remains **1 of 8**.

### Previous increment: slice 29 — Stuffed Bear (first animation of a noncreature permanent)
"""

sub("### Latest increment: slice 29 — Stuffed Bear (first animation of a noncreature permanent)\n", SLICE30)

pathlib.Path("evidence/issue17/slice30/pr-body-after.md").write_text(s, encoding="utf-8")
subprocess.run(
    ["gh", "pr", "edit", "46", "--body-file", "evidence/issue17/slice30/pr-body-after.md"],
    check=True,
)
readback = subprocess.run(
    ["gh", "pr", "view", "46", "--json", "body", "--jq", ".body"],
    capture_output=True, text=True, encoding="utf-8", check=True,
).stdout
pathlib.Path("evidence/issue17/slice30/pr-body-readback.md").write_text(readback, encoding="utf-8")
norm = lambda t: t.replace("\r\n", "\n").strip()
print("READBACK_EQUALS_WRITTEN", norm(readback) == norm(s))
print("BYTES", len(s), len(readback))
