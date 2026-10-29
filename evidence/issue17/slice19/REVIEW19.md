# Slice 19 — independent review, round 1 (execution lens)

Reviewer: independent sub-agent session (`deleg_331b3470`), no part in writing the
code, read-only in the worktree. Scope: `21fdefb..e8f8b18`, the slice-19 delta only.

## Verdict: CHANGES_REQUESTED

## Findings (all accepted; none argued)

1. **The reviewed head did not compile its test suite.** `crates/core/tests/catalog_pool.rs`
   matches `Effect` exhaustively; `Effect::SkipNextUntapStep` had no arm. Proved by
   exporting `HEAD` with `git archive` into a scratch tree and running
   `cargo check -p aperture-core --tests` → `E0004`. The fix existed only as an
   uncommitted working-tree edit.
2. **`crates/core/tests/card_printings.rs:550` failed**: "Spectral Bears has no
   recorded authoritative printing" — the slice-19 fixture was not chained into
   `every_pool_card_added_after_issue_15_has_a_recorded_printing` in the committed
   file (again only in the dirty worktree).
3. **`issue13_commander_combat.rs:1375` pinned `pool_size() == 135`** → 42/43 failed
   (`fog_is_a_sourced_catalog_card`).
4. **`issue17_slice15_corpus.rs:124-125` pinned 30 missing / 135 pool** → both
   assertions failed. Neither file is in the delta, so the delta silently broke two
   previously green suites.
5. **`evidence/issue17/slice19/gates.log` was not honest evidence**: it contained a
   failing line from the author's own run and a mangled clippy exit code, while the
   handoff claimed full workspace tests and clippy were green.
6. **The reviewed commit was not the code that was tested** (four uncommitted edits
   to in-delta test files).

## Coverage gap the reviewer found (non-blocking as stated, fixed anyway)

The resolution-time re-check of the printed intervening-if had **no** test, so a
mutation disabling it stayed green. `the_printed_condition_is_rechecked_when_the_trigger_resolves`
and `red-mutation-resolution-recheck-disabled.log` close it.

## What the reviewer executed (its own numbers)

- `cargo test -p aperture-core --test issue17_slice19 --offline` → 5 passed (also 5/5
  in a clean `git archive HEAD` export).
- `--test abilities` 13 ok, `--test command_zone_setup` 44 ok, `--test replay_digest`
  6 ok, `--test sba_cleanup` 7 ok.
- `--test issue13_commander_combat` 42 ok / 1 failed; `--test issue17_slice15_corpus`
  1 ok / 1 failed.
- Release CLI `replay --file evidence/issue17/slice19/demo-replay.json` → seed 1919,
  4 seats, 1773 actions, digest `5fd841b2…` matches, 4709 events, Win P3; its own
  recomputation of `snapshot_sha256` matched.
- Slice-17 and slice-18 demo replays still verify (`85fabffc…`, `09e6bb25…`), so the
  serde addition did not move recorded digests; `demo-save.json` contains no
  `skip_next_untap` key, confirming the skip is real.
- Own Python audit → 136 catalog names, 459 union = 124 supported + 335 missing,
  Ayula 29 missing, eight lists at 99 cards, no listed-but-supported names.
