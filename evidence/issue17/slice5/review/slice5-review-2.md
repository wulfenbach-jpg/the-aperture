# Slice 5 delta re-review — commit 4c03673

Verdict: **request_changes** (one low, documentation-only finding: the new HANDOFF limitation note is
still wrong about Crack the Earth).

Scope: only the three findings of the independent slice-5 review at `eb0a6f5`
(`evidence/issue17/slice5/review/slice5-review-1.json`). Every statement below was re-executed in this
worktree; nothing is taken from the commit message.

## S5-R1-F1 — deck is 40 cards and the shipped journey matches it: **repaired**

```
$ awk '/^[0-9]+ /{s+=$1} END{print s}' decks/package-slice5-mill.txt
40
$ for f in decks/package-*.txt; do printf "%s " "$f"; awk '/^[0-9]+ /{s+=$1} END{print s}' $f; done
ayula 40  cross-corpus 40  krenko-tokens 40  slice5-mill 40  soul-warden-tokens 40  talrand-drakes 40  tatyova-landfall 40
```

Header line 1 ("supported 40-card package fixture, NOT a desired 100-card starter") is true, and the six
primitives it names are all in the list (3 Stitcher's Supplier, 3 Ruin Crab, 3 Negate, 3 Preordain,
2 Pyretic Ritual, 2 Seething Song, 4 Grizzly Bears, 4 Runeclaw Bear, 3 Guttersnipe, 5 Swamp, 5 Island,
3 Mountain).

Fresh run of the release binary with the same seed and four copies of that deck:

```
$ ./target/release/aperture-cli.exe selfplay --seed 501 \
    --deck decks/package-slice5-mill.txt --deck decks/package-slice5-mill.txt \
    --deck decks/package-slice5-mill.txt --deck decks/package-slice5-mill.txt \
    --commander-combat-practice --commander 0:"Jasmine Boreal" --commander 1:"Jasmine Boreal" \
    --commander 2:"Jasmine Boreal" --commander 3:"Jasmine Boreal" --out … --replay-out …
selfplay seed=501 seats=4 heuristic vs heuristic vs heuristic vs heuristic -> status terminal
winner P2 in 1880 steps (45 turns)
  digest d0c888f680d7 | replay verified | casts 14 attacks 25 blocks 17 counters 0
```

The report I produced is **byte-identical** to the shipped `evidence/issue17/slice5/slice5-journey.json`:

```
$ diff <my report> evidence/issue17/slice5/slice5-journey.json
JSON IDENTICAL
```

so its `seats[].cards = 40` (x4), `status terminal`, `steps 1880`, `turns 45`, `winner 2`,
`events 4654`, `final_digest d0c888f680d7024189e41848f15d99220bb32edb142a13fee64bf3ba0487cb79`,
`replay_digest_matches true` describe the 40-card deck that is committed now. The replay also verifies on
its own, and my regenerated replay is byte-identical to the shipped one:

```
$ ./target/release/aperture-cli.exe replay --file evidence/issue17/slice5/slice5-journey.replay.json
replay seed 501 — 4 seat(s), 1880 action(s), recorded outcome Some(Win { winner: 2, reason: "opponent
lost by attempting to draw from an empty library (CR 104.3c, 704.5b)" })
  replayed digest d0c888f680d7 | matches recorded | events 4654
```

This matches `slice5-journey-replay.log` line for line (1880 actions, 4654 events, digest
`d0c888f680d7`).

## S5-R1-F2 — the preordain test now discriminates order: **repaired**

The test (`crates/core/tests/issue17_slice5.rs:372-462`) drives resolution by hand: it bottoms the
*first* looked-at card (`to_top = first_order_answered`, i.e. false then true) and asserts
`last OrderedCard index < first Draw index`, then `library == [Mountain, Island]` and `hand == [Forest]`.
A draw-then-scry implementation would draw the Island and could satisfy neither.

```
$ cargo test -p aperture-core --test issue17_slice5
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Independent proof by flipping the order — done on a throwaway copy of the workspace outside the
repository (`%TEMP%/r2-scratch-56878`, worktree never edited), catalog line 43 changed to
`&[Effect::Draw(1), Effect::Scry(2)]`:

```
$ cargo test -p aperture-core --test issue17_slice5 preordain --offline --locked
test preordain_scries_two_then_draws_one ... FAILED
panicked at crates\core\tests\issue17_slice5.rs:440:5:
both scry decisions precede the draw (scry then draw, not draw then scry)
test result: FAILED. 0 passed; 1 failed; 0 measured; 9 filtered out
```

So the assertion really is order-sensitive (the review-1 complaint was that the old test passed either
way). `slice5-red-F2-order.log` records the same failing assertion and message, at line 438:5 — two
lines earlier, i.e. captured before two comment lines were added; same assertion, no behavioural gap.

## S5-R1-F3 — APNAP note vs. the code: **partially repaired (finding stands, low)**

`HANDOFF.md:317-321` now claims:

> Effects that act on "each opponent"/"each player" (`Effect::MillEachOpponent`,
> `Effect::DamageEachOpponent`, Crack the Earth's sacrifice) iterate seats in seat order, not in APNAP
> order (CR 101.4).

- True for the two each-opponent effects: `engine.rs:4435` and `engine.rs:4457` both loop
  `for opponent in 0..self.state.seat_count()`, skipping the controller and dead seats.
- **False for Crack the Earth.** `Effect::EachPlayerSacrifice` (`engine.rs:4335`) calls
  `begin_apnap_sacrifice`, which builds the queue with `apnap_order_from(turn.active)`
  (`engine.rs:4032-4057`), and `apply_apnap_choices` (`engine.rs:4180-4197`) applies the batch in that
  same APNAP order. 17 passing tests in `crates/core/tests/apnap.rs` assert exactly that, e.g.

```
$ cargo test -p aperture-core --test apnap
test apnap_order_anchors_at_nonzero_active_and_wraps ... ok      # active seat 2 -> order vec![2, 3, 0, 1]
test apnap_skips_an_already_lost_seat ... ok                    # -> order vec![P0, 2]
test result: ok. 17 passed; 0 failed
```

`engine.rs` is not touched by 4c03673 (`git diff eb0a6f5 4c03673 --stat` lists only HANDOFF.md, the
focused test, the deck and `evidence/`), so those tests describe the shipped code.

Impact is documentation only — no rules or gameplay defect — but the note now tells a reader that
CR 101.4 sacrifice ordering is unimplemented when it is implemented and tested. Fix: remove
", Crack the Earth's sacrifice" (and the "each player" half if nothing else needs it), or say that
Crack the Earth's `EachPlayerSacrifice` does use APNAP from the active seat while the two each-opponent
effects use seat order.

## Gates re-run once

```
cargo test -p aperture-core --test issue17_slice5     -> 10 passed / 0 failed
cargo fmt --all --check                               -> exit 0, no diff
cargo clippy --workspace --all-targets --locked -- -D warnings
                                                      -> Finished `dev` profile … exit 0, no warnings
```

## Worktree integrity

- HEAD still `4c03673`; nothing committed, pushed, merged or closed.
- No tracked file was edited by this review; the F2 flip lived only in a throwaway copy under `%TEMP%`.
- `git status --porcelain` still shows the same single tracked entry as before this review,
  ` M crates/core/src/catalog.rs`, and it is a stat/mtime-dirty index entry rather than a content
  change: `git diff` is empty and `sha256sum crates/core/src/catalog.rs` =
  `git cat-file blob HEAD:crates/core/src/catalog.rs` =
  `73b58649c90c9424a0904131dbd4534845e367b88c2682d7971f08295f15a4c8`.
