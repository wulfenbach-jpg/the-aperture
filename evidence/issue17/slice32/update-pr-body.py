"""Refresh the PR46 body for slice 32 from the live body, then verify by readback."""
import json
import pathlib
import subprocess

OUT = pathlib.Path("evidence/issue17/slice32")
before = (OUT / "pr-body-before.md").read_text(encoding="utf-8")
s = before


def sub(old, new, expect=1):
    global s
    n = s.count(old)
    if n != expect:
        raise SystemExit(f"expected {expect} of {old[:70]!r}, found {n}")
    s = s.replace(old, new)


sub("- Catalog **147 real cards**.", "- Catalog **148 real cards**.")
sub(
    "- Desired eight-list union **459 names = 135 supported + 324 missing** (per-list missing `55/0/51/48/18/52/52/53`). The union includes the five basic land types.",
    "- Desired eight-list union **459 names = 136 supported + 323 missing** (per-list missing `55/0/51/48/17/52/52/53`). The union includes the five basic land types.",
)
sub("- Nearest list, Ayula: **18 missing names**.", "- Nearest list, Ayula: **17 missing names**.")
sub(
    "- Last **behavior** change: slice 31, `Beorn the Fierce` (code head `e53b987`). `Drover Grizzly` (slice 30) remains the previous behavior increment.",
    "- Last **behavior** change: slice 32, `Kamahl's Summons` plus `Surrak and Goreclaw`'s printed trample repair (code head `40a87aa`). `Beorn the Fierce` (slice 31) remains the previous behavior increment.",
)

sub(
    "### Latest increment: slice 31 — Beorn the Fierce (HOB 119)",
    """### Latest increment: slice 32 — Kamahl's Summons (ONS 269) and Surrak's printed Trample

`Kamahl's Summons` (`{3}{G}` Sorcery; Scryfall **ONS 269**, immutable `0edc37c6-b6a8-424f-95dd-928d03c28542`, oracle `8b6ef0a2-cd90-44c8-8f11-e1983f2b1dfd`) prints "Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green Bear creature token for each card they revealed this way." (fixture `fixtures/starter-corpus/slice32-sourced.json`, enforced by `crates/core/tests/card_printings.rs`).

Reusable machinery, not a one-card patch:

* `Pending::ApnapReveal` is the pool's first **per-seat arbitrary hand subset**: every live seat answers in APNAP order from the active player (CR 101.4, 608.2f), and `Action::RevealCreatureCards` names any subset of that seat's own creature cards (CR 701.9a). An empty list is a real answer, because the card prints "may reveal **any number**".
* Revealing never moves a card: the revealed cards stay in their owner's hand, so the only thing a reveal buys is tokens. Validation is typed — a card not in that seat's hand, a non-creature card, a duplicate, or a reveal when nothing is pending is refused with `EngineError::Illegal` and leaves the pending choice untouched.
* **Nothing is applied per answer.** Every seat's count is fixed by its own reveal and the Bears are created together, in APNAP order, only after the last seat has answered, so no seat's result depends on the answering order.
* The pending reveal is projected only to the seat that owes it (like the APNAP sacrifice queue) because the offer names that seat's hand; the answers are public and logged as `Event::CardsRevealed`.
* `Surrak and Goreclaw` now prints its **own** Trample: the row moved from `card!` (trample false) to `card_trample!`, so `Game::has_trample` answers it exactly like every other trampling card. Its MOM 337 printing (`keywords: ["Trample"]`) is recorded in the same fixture.

Verification: 4/4 focused slice-32 tests (three-seat APNAP reveal with counts 1/2/0 and cards still in hand, every refusal, mandatory-target decline neither offered nor legal, Surrak's real trample assignment) + 2 provenance tests; four discriminating mutation REDs with every touched file restored byte-for-byte; workspace **118 suites / 833 passed / 0 failed**; fmt, `clippy -D warnings` and locked release clean; a real four-seat/40-life exact-snapshot journey (`decks/package-slice32.txt`, seed 3232, terminal `Win{P3}`, 1362 actions, 32 turns, 3660 replay events, digest `f6e65209…`, 12 APNAP reveal answers, 23 revealed cards -> 23 Bear tokens, 10 Surrak combat-damage events) with a byte-identical rerun and a separate release CLI replay matching. The Surrak repair moves exactly one previously recorded journey: slice 24 `a41bbfd2…`/4127 events -> `697a846d…`/4130 events (same 1602 actions and decisions); every other committed journey digest is unchanged (`evidence/issue17/slice32/replay-drift.log`).

### Previous increment: slice 31 — Beorn the Fierce (HOB 119)""",
)

(OUT / "pr-body-after.md").write_text(s, encoding="utf-8")
subprocess.run(
    ["gh", "pr", "edit", "46", "--body-file", str(OUT / "pr-body-after.md")],
    check=True,
    timeout=120,
)
live = json.loads(
    subprocess.run(
        ["gh", "pr", "view", "46", "--json", "body"], check=True, capture_output=True, text=True, timeout=120
    ).stdout
)["body"]
(OUT / "pr-body-readback.md").write_text(live, encoding="utf-8")


def normalized(text):
    # GitHub stores the body with CRLF line endings and pads blank runs; compare
    # the actual content, not the transport's newline normalisation.
    return [line.rstrip() for line in text.replace("\r\n", "\n").split("\n") if line.strip()]


if normalized(live) != normalized(s):
    raise SystemExit("readback does not match the body that was sent")
print("PR body updated and verified by readback:", len(live), "bytes")
