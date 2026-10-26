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
   but its id stays in a zone list the reconstruction rebuilds (belief entries
   for opponents, real ids only for the viewer's own hand or a supplied known
   library). Copying the id alone leaves a state that fails the engine's
   zone-container invariant — currently masked, because the `revealed_hidden`
   projection filters unknown ids — and copying the object too is rejected by
   validation. The world reports no reveals and reads nothing hidden to build
   the field. Disclosed in the `reconstruct` doc comment.
3. `crates/ui/src/app.rs` test fixture `inject_large_board` builds an `Object`
   literal; it sets no typed counters (synthetic board bulk only).

Regression test added: `crates/ai/tests/search.rs::a_live_reveal_leaves_the_search_world_runnable_and_legal`
— a real engine reveal (`reveal_top`) is public in the masked view, the search
still returns scored candidates, and every candidate is an engine-offered
action. It deliberately does not claim to observe the world's reveal state or
hidden-identity independence.

## Independent review of this delta

`evidence/issue16/REVIEW_COMPAT.md` — VERDICT: APPROVED (`385ac67`), no blocking
finding, three non-blocking accuracy notes. Those notes are applied as this
doc-only follow-up: the "dangling reference" wording is corrected to the actual
invariant violation, the belief re-deal sentence is scoped to the zone lists it
really covers, and the test name/docstring now state what the test observes.

## Gates actually executed at this head

- `cargo test --workspace --locked`: pass (full workspace suite; exit 0).
- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.

The untracked probe `crates/core/tests/scratch_pre16_replay_check.rs` left by an
earlier reviewer was moved out of the checkout to
`continuity/probes/scratch_pre16_replay_check.rs` (it alone made fmt/clippy
fail; it was never part of any commit).
