# Issue 17 slice 7 — S6-R2-F1 repair + master integration handoff

No push. Full #17 unfinished; 8 desired, 1 implemented, 0 certified.

## Commits

* `f1b7ab6` — `fix(core): dispatch each self-death trigger once (S6-R2-F1)`
* `edd8008` — merge `origin/master` `8de0961370c595f82f3e1d7505b37c7e67958183`
  into `aperture/issue-17` (two-parent integration commit)
* Backup branch: `issue17-pre-slice7-rebase` at `f1b7ab6`
* Pre-merge feature head: `3987dd6`

## S6-R2-F1

Reproduced reviewer224's probe: `[22,18,18,18]` instead of `[21,19,19,19]` for
one dying Zulaport and `[23,17,17,17]` instead of `[22,18,18,18]` for
Zulaport+Jackal. Root cause: `move_to_graveyard_inner` queued the self-death
trigger, then the watcher batch dispatched the same dying object again.

Fix: the watcher batch now carries the **original battlefield ObjectId** and
skips the dying watcher's own death, because `queue_self_death` already queued
it. Surviving watchers still see every other death. Three probe tests were
ported into `issue17_slice6.rs` (both death orders, single dying watcher,
surviving watcher control) and all assert full drain (stack, trigger_queue,
resolving, pending empty) and exact life totals. External probe:
RED in `review224-probe/output.log`, GREEN in `output-green.log` /
`output-merged-green.log` (3/3).

## Integration

Fetched with command-scoped credential override: `origin/master` =
`8de0961` (#16 typed counters, tokens, keywords, library operations). Rebase
would require a forbidden force push, so used the documented merge fallback.
Conflicts: `HANDOFF.md`, `README.md`, `card.rs`, `catalog.rs`, `engine.rs`,
`state.rs`. Resolutions:

* Kept all corpus cards/variants and master `SoldierToken`/`SpiritToken`;
  unified `CardId::is_token` over all four engine tokens.
* Adopted master’s typed counter/token/keyword engine where it superseded the
  duplicate local token machinery; extended `tokens::keywords` for Drake
  (flying) and Jaddi (defender).
* Initial integration wrongly adopted #16's immediate-cessation token
  behavior. Repair-2 rejects that semantic regression: tokens now reach the
  real destination (graveyard/hand/library/exile) through `insert_object`, fire
  `Destroyed`/death watchers, then `check_sbas` records CR 111.7 cessation and
  removes the object. State validation still refuses any stable off-battlefield
  token. Tests expecting `Zone::Removed` tombstones now assert the object is
  absent (representation-only adaptation); substantive ordering/event assertions
  are restored from `f1b7ab6`. #16's invented “instead of exile/hand” assertions
  were corrected to destination-then-cessation ordering.
* Added required `counters` field to corpus test object literals.
* Docs: kept branch corpus/README content for conflicts.

## Verification

* Strong token ordering RED at `edd8008` is preserved in
  `slice7-token-ordering-red.log`; after repair tokens 10/10, slice6 13/13,
  counters_tokens_library 16/16, cross 6/6, Tatyova 5/5, Talrand 7/7,
  SBA 7/7; external probe 3/3.
* `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
  release CLI build: exit 0 (`slice7-gates.log`).
* Real four-seat package journey (`decks/package-slice6-dies.txt`, 40-card
  package, not a desired starter, seed 610): terminal, P3 wins, 2952 steps /
  66 turns, digest `8e0ab4b00e7f…`; separate replay matches 7239 events.
  This digest differs from the pre-integration slice6 `7864e4e9…`, recorded
  honestly; no old digest was rewritten.
* Corpus audit with the extended macro regex over the real 106-card pool:
  catalog **106**, union 459, supported **94**, missing **365**, manifest
  matches (`slice7-native-corpus.json`). The earlier 93/81/378 line was stale
  from the slice3 catalog and is corrected here.

## Unfinished

Full desired-starter certification is still open; no new feature cards were
added in this assignment. Typed counters/keywords and token object removal from
merged #16 are retained; corpus destination-then-SBA token semantics are preserved.
Required CI/independent review remain coordinator-owned.
