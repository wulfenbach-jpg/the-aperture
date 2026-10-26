# PR43 integration reconciliation (issue 16 onto master 51fbc6b)

## What was integrated

- Branch head reviewed and approved: `ae85861` (`evidence/issue16/REVIEW_DELTA_RECOVERY.md`).
- New base: master `51fbc6b`.
- Merge commit: `2e97bb7` ("Merge master (51fbc6b) into issue16 for protected integration"), conflict-free.
- Compatibility fix on top: see below.

## Correspondence proof for the merge itself

The reviewed delta `git diff 1ec6d9a..ae85861` was re-applied onto a clean
worktree at master `51fbc6b` and written out as a tree:

- reconstructed tree: `227afc4e37341e5de6d8035d2700aaf0141422b0`
- merge result tree: `6ab8da6bb3517bce9d98a5ee890ffb2f01864fff`

The only tree difference was the scratch patch file itself; with it removed the
two trees are identical. So the merge realises exactly the reviewed delta on the
new base — no master content dropped, no unreviewed content introduced.

## Newly authored compatibility fix (needs its own review, not covered by the delta approval)

The merged frontier did not compile. Master landed `crates/ai/src/search.rs`
(#24) after `ae85861` was cut, and #16 adds two state fields that the
information-set reconstruction never set:

1. `Object.counters` — public and own objects are cloned straight from the real
   state, so their typed counters (CR 122) and counter-adjusted
   power/toughness already travel into the search world. Only fabricated
   belief objects needed an explicit empty `Counters::new()`; nothing is
   invented for hidden information.
2. `GameState.revealed` — deliberately not modelled. A revealed card is public,
   but it still sits in a library/hand list that the reconstruction re-deals
   from the belief deck with real counts only, so copying the id would create a
   reference the world never places. The world reports no reveals and reads
   nothing hidden to build the field. Disclosed in the `reconstruct` doc comment.
3. `crates/ui/src/app.rs` test fixture `inject_large_board` builds an `Object`
   literal; it sets no typed counters (synthetic board bulk only).

Regression test added: `crates/ai/tests/search.rs::a_live_reveal_neither_breaks_the_world_nor_leaks_a_hidden_card`
— a real engine reveal (`reveal_top`) is public in the masked view, the search
still returns scored candidates, and every candidate is an engine-offered
action.

## Gates actually executed at this head

- `cargo test --workspace --locked`: pass (full workspace suite; exit 0).
- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.

The untracked probe `crates/core/tests/scratch_pre16_replay_check.rs` left by an
earlier reviewer was moved out of the checkout to
`continuity/probes/scratch_pre16_replay_check.rs` (it alone made fmt/clippy
fail; it was never part of any commit).
