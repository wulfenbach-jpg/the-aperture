# Review 27 — CI facts, observed directly (2026-10-27, after head 27d8f48)

## Push workflow — SUCCESS
`gh run view 40493923124` → event `push`, conclusion `success`, headSha
`27d8f483cf431626c0eb95f308e6e328cf8353c3`, workflow `Rust quality`.
Jobs (5): board and roadmap = success, windows package = success,
rust (macos-latest) = success, rust (ubuntu-latest) = success,
rust (windows-latest) = success.

## Pull-request workflow — FAILURE on the merge ref
`gh run view 40493931766` → event `pull_request`, conclusion `failure`,
workflow `Rust quality`, headBranch `aperture/issue-17`,
displayTitle `Partial starter corpus: Jasmine and typed goblin/Drake packages`.
Jobs (5): windows package = success, board and roadmap = success,
rust (windows-latest) = failure, rust (ubuntu-latest) = failure,
rust (macos-latest) = failure.

`gh api repos/wulfenbach-jpg/the-aperture/git/refs/pull/46/merge --jq .object.sha`
→ `f68997faa13abf29f75d650c9ccf5c5bd813b8b7` (the synthetic merge of this
branch with `origin/master` = `51dffdab` "#28 Move review ... (#56)").

`gh run view 40493931766 --log-failed` (identical on all three rust jobs):

    error[E0308]: mismatched types
      --> crates/ai/tests/review.rs:60:26
       |              ---         ^^^ expected `u32`, found `u8`
      --> crates/core/src/mana.rs:86:12

Root cause verified locally, not assumed:

* `crates/ai/tests/review.rs` does not exist on this branch (`ls crates/ai/tests/`:
  common, decider, decider_boundary, decider_observation, decider_redirect,
  decider_schema, decider_usage, heuristic, search, selfplay). It was added on
  master by `51dffda` (`git log --diff-filter=A 51dffda -- crates/ai/tests/review.rs`),
  and its `pool(&[(ManaColor, u8)])` helper passes `*count: u8` to `ManaPool::add`.
* `git show origin/master:crates/core/src/mana.rs | grep 'pub fn add'` →
  `pub fn add(&mut self, color: ManaColor, amount: u8)`.
* `grep -n "pub fn add" crates/core/src/mana.rs` on this branch → line 86,
  `amount: u32`; introduced by branch-only commit `e300bdd`
  (`git log -S "amount: u32" -- crates/core/src/mana.rs`).

## Attribution
The three red rust jobs are a semantic (not textual) conflict between master's
new test file 51dffda and this branch's pre-existing u32 widening e300bdd.
Nothing in the reviewed delta `80fff73..27d8f48` touches `mana.rs`,
`crates/ai`, or any type signature — the delta is comments, docs and evidence
only, and the driver assertion line. The push workflow at exactly 27d8f48 is
fully green (5/5). The PR-merge-ref failure is master drift interacting with
older branch code, outside this delta: not attributable to the delta.
