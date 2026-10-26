# Slice 5 — independent practical review assignment

Repository/worktree (read-only for implementation): `C:\AI\aperture-worktrees\issue-17`
Branch `aperture/issue-17`, candidate head **eb0a6f5** on draft PR46
(<https://github.com/wulfenbach-jpg/the-aperture/pull/46>).

## What was implemented (verify by running, not by reading this file)

Six sourced cards, all in `crates/core/src/catalog.rs`, all with live Scryfall
provenance in `fixtures/starter-corpus/slice5-sourced.json`:

| card | printing | new behaviour |
|---|---|---|
| Stitcher's Supplier | tdc 196 | mill three on entering **and** on dying |
| Ruin Crab | znr 75 | landfall: each opponent mills three |
| Negate | tmt 47 | counter target **noncreature** spell |
| Preordain | tdc 161 | scry 2, then draw a card |
| Pyretic Ritual | m11 153 | add {R}{R}{R} |
| Seething Song | c21 179 | add {R}{R}{R}{R}{R} |

New primitives: `Effect::MillSelf` / `Effect::MillEachOpponent` (real
library→graveyard move, one `Event::Milled` per card), `TargetSpec::NoncreatureSpell`,
`Effect::AddMana`.

## Claims to test independently

1. Mill is a real zone move in library order and fires no "dies" trigger; the
   count matches `n` and stops at an empty library.
2. Milling an **empty** library does not lose the game (CR 704.5b is a draw rule).
3. Stitcher's Supplier mills on ETB and again on death — two distinct triggers,
   one printed ability.
4. Ruin Crab's landfall mill hits every live opponent and never its controller.
5. Negate never offers a creature spell on the stack and refuses a forged
   `Target::Spell` on a creature spell with `BadTarget` and no state change; a
   countered noncreature spell resolves no effect.
6. `AddMana` adds exactly the printed amount after the cost is paid from the pool;
   `Event::ManaAdded.pool_total` is truthful.
7. Preordain's scry-2-then-draw really orders then draws (not draw-then-scry).
8. Provenance: fetch each of the six cards from live Scryfall yourself and compare
   oracle text, mana cost, type line and P/T with the catalog strings.
9. `pool_size()` is 103 and every new card name is importable; `Hedron Crab`
   (target-player mill) is still refused.
10. Replay/logging: a new `Event` variant must not break the shipped replay
    fixtures or save format; check `replay_digest`, `ordering_persistence`,
    `projection` and `scenario`-style suites still pass.

## Commands

```
cargo test -p aperture-core --test issue17_slice5
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --release --locked
target/release/aperture-cli.exe selfplay --seed 501 \
  --deck decks/package-slice5-mill.txt --deck decks/package-slice5-mill.txt \
  --deck decks/package-slice5-mill.txt --deck decks/package-slice5-mill.txt \
  --commander-combat-practice --commander '0:Jasmine Boreal' \
  --commander '1:Jasmine Boreal' --commander '2:Jasmine Boreal' \
  --commander '3:Jasmine Boreal' --out <tmp>.json --replay-out <tmp>.replay.json
target/release/aperture-cli.exe replay --file <tmp>.replay.json
```

## Rules of engagement

* Do **not** edit implementation, tests, docs, the deck lists or the manifest.
  You may write only under `evidence/issue17/slice5/review/`.
* Do not merge, rebase, push, close the issue or touch global config.
* Reproduce at least one defect-or-clean verdict per claim with actual output.
* Verdict: `approve_increment` only if every claim above survives your own
  execution; otherwise `request_changes` with concrete, minimal findings.
