# Slice 19 — independent re-review, round 2 (fixes only, execution lens)

Reviewer: a second independent sub-agent session (`deleg_cabac6e0`), read-only, its
own `CARGO_TARGET_DIR` so it could run while the owner's full gate suite occupied the
shared target directory. Scope: whether the six round-1 findings are actually fixed
at `f80ef86`, plus preservation of previously passing behavior.

## Verdict: APPROVE

## Fixes verified (each with a command, not a claim)

1. `catalog_pool.rs:240` now has `Effect::UntapTargetAndCounterIfBear | Effect::SkipNextUntapStep`;
   `--test catalog_pool` → 5 passed.
2. `card_printings.rs` const/`slice19_cards()`/`.chain(...)` at :49/:165/:547;
   `--test card_printings` → 8 passed.
3. `issue13_commander_combat.rs:1377` → `pool_size() == 136`; suite 43 passed.
4. `issue17_slice15_corpus.rs:126-127` → 29 missing / 136 pool; suite 2 passed.
5. `README.md:24,304` say 136; no live 135 reference remains.
6. The new test is non-vacuous on all four points it must check: it asserts the
   trigger is on the stack, asserts no marker *before* resolution, stages a black
   nontoken permanent while the trigger is on the stack, then asserts both the
   `TriggerRemoved` "at resolution" event and an empty marker set afterwards. The
   mutation log shows exactly that one test failing (5 passed / 1 failed) when the
   re-check is disabled.

## Preservation

- `git diff e8f8b18..f80ef86 -- crates/*/src` is empty: the fixes changed tests, docs
  and evidence only, so the engine behavior approved in round 1 is unchanged.
- Focused regression sweep it ran: `catalog_pool` 5, `card_printings` 8,
  `issue13_commander_combat` 43, `issue17_slice15_corpus` 2, `issue17_slice19` 6,
  `abilities` 13, `command_zone_setup` 44, `replay_digest` 7, `sba_cleanup` 6,
  `issue17_slice16` 6, `issue17_slice17` 7 → **147 passed, 0 failed**; `cargo fmt
  --all --check` exit 0.
- Worktree check: no untracked or modified file under `crates/`, `fixtures/`, `docs/`,
  `decks/` or `README.md`, so the tested Rust sources are byte-identical to the
  reviewed head. Only untracked entries are pre-existing backup/evidence directories.

## Non-blocking notes it raised, and what happened to each

1. `.starter-run.yml` (agent-runner configuration) did not belong in a `test(core)`
   commit → removed from tracking; the file stays local and untracked, as before.
2. The refreshed `gates.log` was uncommitted at re-review time → committed in the
   final evidence commit.
3. Repository convention for saved session logs (root `review-sessions/` untracked vs
   `evidence/issue17/review-sessions/` tracked) is inconsistent → pre-existing, left
   to the integration supervisor rather than changed inside a card increment.

## Caveat the reviewer stated

It could not re-run the mutation itself (read-only), so finding 6 rests on the
committed mutation log plus the assertion structure it read. The mutation log was
captured before `rustfmt`, so it panics at line 301 while the assertion sits at
305/311 at HEAD; the delta is exactly the reformat recorded in the pre-fix gate log —
same test, same assertion.
