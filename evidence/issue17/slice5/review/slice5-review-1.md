# Slice 5 review 1 — independent execution review

Candidate: `eb0a6f5` (`aperture/issue-17`, draft PR46). Reviewer ran everything below on
the worktree `C:\AI\aperture-worktrees\issue-17`; nothing outside
`evidence/issue17/slice5/review/` was written and no repository state was changed.

## Verdict

**approve_increment.** All ten assignment claims survived independent execution
(engine probes, live Scryfall, gates, CLI journey, replay and save round-trips).
Two low-severity and one informational finding, none of them a rules defect:
a mislabelled package-deck size, one non-probative focused test, and a
log-ordering nit shared with pre-existing code.

## What was run

| gate | command | result |
|---|---|---|
| focused | `cargo test -p aperture-core --test issue17_slice5` | 10 passed / 0 failed |
| workspace | `cargo test --workspace --locked` | 526 passed / 0 failed, 82 suite result lines (78 test binaries + 4 doc-test bins) |
| named suites | `replay_digest, ordering_persistence, projection, scry_surveil, counterspell, timing_mana, catalog_pool, card_printings, starter_corpus` | 6 / 2 / 4 / 5 / 4 / 12 / 5 / 6 / 8 — all pass |
| fmt | `cargo fmt --all --check` | exit 0, no diff |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0, no warnings |
| release | `cargo build --workspace --release --locked` | exit 0 |
| journey | `selfplay --seed 501`, four `package-slice5-mill` decks, four `Jasmine Boreal` commanders | terminal, winner P2, 2494 steps, 60 turns, digest `104adbf674ec216ea329adb8562e650b0fcdc210b7bd97b190d0a4b991c3e537`, replay verified; a second run of the same seed reproduced the identical digest |
| replay | `aperture-cli replay --file journey.replay.json` | 2494 actions, 6227 events, replayed digest `104adbf674ec` matches recorded |
| Scryfall | `review/scryfall_check.py` | 6/6 cards match on oracle text, mana cost, type line and P/T |
| corpus numbers | `review/corpus_check.py` | union 459, catalog 103, supported 91, missing 368, elas unsupported nonland 57 — all match README/docs and all eight manifest `missing_cards` lists |
| own probes | `review/probe_slice5.rs` (rustc against the built rlib) | 14/14 pass |

Artifacts: `focused.log`, `workspace-tests.log`, `named-suites.log`, `fmt.log`,
`clippy.log`, `release-build.log`, `journey.log`, `journey.json`,
`journey.replay.json`, `journey-replay.log`, `journey-verbose.log`,
`scryfall_check.py`, `scryfall-check.log`, `scryfall-live.json`,
`corpus_check.py`, `corpus-check.log`, `probe_slice5.rs`, `probe-build.log`,
`probe.log`, `legacy-replay.log`.

## Claim-by-claim

1. **Mill is a real zone move** — confirmed. `short_library_mill` (library of
   one, mill 3) mills exactly one card, stops, and keeps the game running;
   `mill_fires_no_battlefield_triggers` mills the top three in library order,
   emits one `Milled` per card and no cascade (the milled Stitcher's Supplier
   card does not trigger itself).
2. **Empty library** — confirmed. `empty_library_mill` mills nothing, life stays
   20, outcome stays `None`, `drew_from_empty` stays false. Milling an opponent
   that has already been milled out is equally harmless.
3. **Stitcher's Supplier mills on ETB and on death** — confirmed.
   `supplier_mills_again_when_it_dies` produces `[Bolt, Supplier, Island,
   Forest, Mountain]`, three `Milled` events and one battlefield departure.
4. **Ruin Crab landfall scope** — confirmed. An opponent's land does not
   trigger it; the controller's land mills each live opponent three, skips an
   eliminated seat, never mills the controller; two crabs stack to six.
5. **Negate** — confirmed on both paths. `legal_targets(NoncreatureSpell)` and
   `validate_cast` are the same code path (`engine.rs` 1743–1877), so the legal
   listing and the validation cannot diverge. A forged creature-spell target
   returns `BadTarget` with the hand, the pool and the stack unchanged; an
   ability on the stack is not a legal target either (CR 115.1); a countered
   Bolt deals no damage.
6. **AddMana** — confirmed. Pool is 3 after paying 2, `ManaAdded { amount: 3,
   pool_total: 3 }` is truthful, the mana survives priority passing inside the
   step, pays for a real `Lightning Bolt`, and empties with the step
   (`ManaPoolEmptied { amount: 2 }`, CR 500.4).
7. **Preordain order** — confirmed by an order-sensitive probe (see F-2 for the
   test's weakness): `Scried → OrderedCard(false) → OrderedCard(true) → Draw
   Forest`, hand `[Forest]`, library `[Mountain, Island]`.
8. **Provenance** — 6/6 match, re-fetched live during this review.
9. **Pool statements** — 103 cards, six names importable, `Hedron Crab` still
   refused by name.
10. **Replay/save compatibility** — workspace green; a pre-slice-5 slice-4 replay
    (`5e905fdc0399`) and slice-3 replay (`4572cd11e893`) still replay to their
    recorded digests, and `slice3-cross-save.json` recomputes its stored digest.
    `slice2-krenko-save.json` does **not** replay — but a build of the parent
    commit `4a6a3f0` in a throwaway clone produces the identical computed digest
    and the identical error, so that staleness predates slice 5 and is not a
    regression (see `legacy-replay.log` and `evidence/issue17/legacy-replay-note.md`).

Hidden-information check: `Event::Milled` carries the card and renders its name
for every viewer; that is correct, because a milled card goes to a public zone
(CR 400.2), unlike `Draw`, which stays viewer-gated. Milled objects get their
tapped/damage/summoning-sick/pump state cleared on the zone move, and the mill
path never goes through `move_to_graveyard`, so no `Destroyed`/dies trigger can
fire for a milled card (verified, not just read).

## Findings

### S5-R1-F1 — `package-slice5-mill.txt` is 50 cards, but is labelled 40 (low)

`decks/package-slice5-mill.txt` line 1 says "supported 40-card package fixture"
and the commit message says "40-card PACKAGE fixture", yet the list sums to 50
cards; every other `decks/package-*.txt` is exactly 40, and both selfplay reports
show `seats[].cards = 50`.

```
$ for f in decks/package-*.txt; do grep -E '^[0-9]+ ' $f | awk '{s+=$1} END{print s, FILENAME}'; done
40  decks/package-ayula-slice4.txt
40  decks/package-cross-corpus.txt
40  decks/package-krenko-tokens.txt
50  decks/package-slice5-mill.txt
40  decks/package-soul-warden-tokens.txt
40  decks/package-talrand-drakes.txt
40  decks/package-tatyova-landfall.txt
```

Fix: trim to 40 cards, or reword the file header / PR text to "50-card package
fixture".

### S5-R1-F2 — the Preordain test cannot tell ordering apart (low)

`crates/core/tests/issue17_slice5.rs:372-418` drives resolution with
`auto_answer`, which always answers `OrderCard { to_top: true }`. With the policy
"keep both on top", scry-then-draw and draw-then-scry produce the *same* final
state for `library = [Island, Forest, Mountain]`:

| order | hand | library after |
|---|---|---|
| Scry(2) then Draw | `[Island]` | `[Forest, Mountain]` |
| Draw then Scry(2) | `[Island]` | `[Forest, Mountain]` |

so the assertion passes either way. The behaviour is correct (verified in
`probe.log` by bottoming the first viewed card and by checking that `Scried`
precedes `Draw`), but the shipped test is not evidence for it.

Fix: assert the event order (`Scried` before the first `Draw`), or answer the
first `OrderCard` with `to_top: false` and assert the drawn card is the second.

### S5-R1-F3 — each-opponent mill order is seat order, not APNAP (info)

With P1 active, `Ruin Crab`'s landfall prints `mill event order: [0,0,0,2,2,2]`
— the loop starts at seat 0 (`engine.rs` 4434-4442) — where CR 101.4 asks for
APNAP order from the active player (P2 first, then P0). No gameplay consequence:
mills are simultaneous, there are no replacement effects and the ordering is
deterministic for replay. `Effect::DamageEachOpponent` has the same pre-existing
shape, so this is consistency-by-copy rather than a new regression.

Fix (optional): iterate `(seat + n - active) % n` like the existing APNAP queues,
or list the deviation in `HANDOFF.md`.

## Notes not filed as findings

* The CLI journey exercises mill and landfall (136 `mills` lines in
  `journey-verbose.log`) but the heuristic resolves no ritual and casts no
  Negate in that seed, so `ManaAdded`/`Countered` never appear in the journey.
  The deck *contains* the cards; coverage of those two primitives rests on the
  focused tests and this review's probes, not on the journey.
* `pool_total` uses `saturating_add`, so at 255 the reported `amount` could
  exceed the true delta. Not reachable with the six-card package.
