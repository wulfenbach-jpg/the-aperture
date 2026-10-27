# Slice 35 — Words of Wilding: the pool's first draw replacement

Date: 2026-10-27. Branch `aperture/issue-17`, draft PR46, one writer. Continues
from the review-296 head `6575c8b` (code head `5b60c2c`).

## What was added and why it is reusable

**Words of Wilding** `{2}{G}` Enchantment — Scryfall **ONS 305**, immutable
`fdb9565f-5b09-4127-b169-3146079dab84`, oracle `888582df-4fe2-4d05-9b62-10c90aaa76f2`,
retrieved **2026-10-27T21:06:29.666092Z** (`evidence/issue17/slice35/sources/collection-raw.json`
plus `retrieval.json`; fixture `fixtures/starter-corpus/slice35-sourced.json`,
enforced by `card_printings::slice35_words_of_wilding_matches_its_recorded_printing`).
Printed text, verbatim:

> {1}: The next time you would draw a card this turn, create a 2/2 green Bear creature token instead.

This is the pool's first **replacement of a draw event** (CR 614.1a), and the
surface is general rather than card-shaped:

* `Effect::ReplaceNextDrawWithBearToken` + `GameState::draw_replacements` — an
  armed, one-shot, per-seat replacement (`Vec<u32>` indexed by seat, serde
  `default, skip_serializing_if = "Vec::is_empty"`, so a game that never resolves
  one keeps its pre-slice-35 serialized form **and digest**).
* `Game::draw_with_resume` now checks the armed replacement **before** the top
  card is looked at: the draw is replaced, no card moves, no `Event::Draw` is
  emitted, and an empty library is not a loss because nothing was drawn.
* `finish_cleanup` clears unspent replacements (CR 514.2); `arm_draw_replacement`
  accumulates, so several can be armed in one turn and each consumes exactly one
  draw (the printed ability has no once-per-turn limit).
* The armed replacement is deliberately **not** bound to the source: the ability
  resolves into an effect that already exists, so destroying the enchantment does
  not undo it.

The ability is `AbilityCost::mana("{1}")`, `TargetSpec::None`, no timing
restriction, `sorcery_timing: false`.

## Verification actually run at this head

* `cargo test -p aperture-core --test issue17_slice35 --locked` — **6 passed / 0 failed**:
  1. the draw step of the controller's turn becomes one 2/2 green Bear token — library and hand
     unchanged, **no `Event::Draw`**, no loss;
  2. one armed replacement replaces exactly one of `Divination`'s two draws (one card + one Bear,
     exactly one `Event::Draw`);
  3. two armed replacements replace both draws of `Divination` **with an empty library** — two
     Bears, no `DrewFromEmpty`, no loss (the CR 614.1a reading that a replaced draw cannot deck a
     player);
  4. an unspent replacement armed after the draw step is cleared in cleanup, so the next turn's
     draw is an ordinary card (CR 514.2);
  5. the replacement applies only to its controller's draws — an opponent's draw step in the same
     turn is an ordinary draw;
  6. the ability is not offered when `{1}` is not payable.
* `cargo test -p aperture-core --test card_printings --locked` — **13 passed / 0 failed**
  (12 before this slice plus the new sourced-printing test).
* `cargo test --workspace --locked` — see `evidence/issue17/slice35/gates.log` (the log records
  the runner's own exit codes; `fmt` in that first capture predates the `cargo fmt --all` run and
  the final fmt/clippy receipts were re-taken clean).
* `cargo clippy --workspace --all-targets --locked -- -D warnings` — exit 0.
* Corpus audit (`evidence/issue17/slice35/corpus-audit.py`, log `corpus-audit.log`) —
  **150 catalog / 459 union = 138 supported + 321 missing**, per list
  `55/0/51/48/15/52/52/53`, Ayula **15**, **1 of 8 certified**, manifest `missing_cards` equal on
  all eight.

## Four-seat journey (real executable, separate replay)

`decks/package-slice35.txt` (40-card evidence-only package) with the
`commander-combat-practice-40` profile, seed **3535**:

```
aperture-cli selfplay --seed 3535 --seats 4 --commander-combat-practice \
  --commander 0..3:Jasmine Boreal --deck decks/package-slice35.txt ×4 \
  --out evidence/issue17/slice35/demo-journey.json \
  --replay-out evidence/issue17/slice35/demo-replay.json
```

Terminal **Win{P2}**, first player P2, **1360 steps / 32 turns / 3564 events**, digest
`59f8bf20176824c406e53f4f963819d00971e1a2dfcc567c1d18bbb07975e9eb`,
`replay_digest_matches: true`, `pool_size 150`, `starting_life 40`, 21 casts, 10 ability
activations. A separate `aperture-cli replay --file evidence/issue17/slice35/demo-replay.json`
matches (`replayed digest 59f8bf201768 | matches recorded | events 3564`, exit 0;
`cli-replay.log`).

**Disclosed limitation of that journey.** The heuristic policy **drew Words of Wilding three
times and never cast it** (`verbose-events.log`), so this game does not itself exercise the new
replacement — the six focused engine tests above are what discriminate it. The journey is real,
four-seat, 40-life, exact-snapshot and separately replayed; it is not evidence of the card's
behaviour, and it is a 40-card package, not a 100-card certificate.

## Scope and state

Touched: `crates/core` (card/state/engine/ability/catalog), `crates/core/tests`
(new slice-35 suite, `card_printings`, `catalog_pool` arms, pinned pool counts), docs/fixtures/
decks/evidence, and two **forced initializer-only** edits outside core —
`crates/ai/src/search.rs` (copies the new field into the search projection) and
`crates/ai/tests/common.rs` (empty `Vec`). No AI-search behaviour, no UI, no import path, no
model edit. Desired starter deck snapshots untouched; the manifest delta is the single removal of
"Words of Wilding" from the Ayula `missing_cards`.

Original issue #17 remains **unfinished**: 1 of 8 desired 100-card lists certified, Ayula 15 names
short, 321 of 459 union names unsupported. No merge, no master push, no issue closure, no
downstream unlock (`t_26a7a4ff`, `t_3c2fe51a` stay parked), no new worker/team/PR.
