VERDICT: APPROVED

# Independent review — PR43 integration compatibility fix (`385ac67`)

Reviewed range: `git diff 2e97bb7..385ac67` on branch `aperture/issue-16`,
worktree `C:/AI/aperture-worktrees/issue-16`. HEAD during review:
`385ac67aff39acd9e47dfb88b4ba6641b4d9c83c`, tracked worktree clean
(`git status --short` empty) before this report was written. Scope limited to
the reconciliation delta; `2e97bb7` and everything before it was not re-audited.

## Commands executed and exit codes

| # | Command | Exit | Note |
|---|---|---|---|
| 1 | `cargo test --workspace --locked` | 0 | full suite, 431.65 s |
| 2 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 3.18 s |
| 3 | `cargo fmt --all --check` | 0 | 1.69 s |
| 4 | `cargo test -p aperture-ai --test search --locked a_live_reveal_neither_breaks_the_world_nor_leaks_a_hidden_card` | 0 | 1 passed, 8 filtered out |
| 5 | `cargo test -p aperture-core --test scratch_reveal_dangling --locked -- --nocapture` | 101 | throwaway probe (see finding 2); test file deleted immediately after the run, worktree clean again |

`git diff 2e97bb7..385ac67 --numstat` is insertion-only (18/48/3/55/30 lines,
0 deletions), so no existing assertion was rewritten or relaxed.

## Findings

### 1. `Object.counters` handling — PASS, no disagreement path found
`crates/ai/src/search.rs:536-544` clones each real object entry whose id is in
the view ledger unchanged; `visible_ids` (`crates/ai/src/search.rs:426-461`)
covers both seats' battlefield plus every public zone and the viewer's own hand.
`Object::power/toughness` (`crates/core/src/state.rs:489-497`) add
`counters.power_delta()/toughness_delta()`, so both the counters and their
counter-adjusted characteristics travel into the search world. Fabricated belief
objects get `Counters::new()` (`crates/ai/src/search.rs:548-565`); that is
correct — their cards are unknown, the 28-card pool prints no counter creator,
and `GameState::validate` (`crates/core/src/state.rs:1551-1558`) independently
forbids counters outside the battlefield. The engine clears counters only on
zone change (`crates/core/src/engine.rs:916,2572`, `crates/core/src/library.rs:57`)
and annihilates opposing kinds in `engine.rs:3576`, so the search world mutates
them by the same rules as reality.

I found no path where a permanent's counters disagree. The one adjacent nuance
is not counters: copied real objects also carry real `pump_power/pump_toughness`
while `effects`/`shields` are emptied (`crates/ai/src/search.rs:718-719`), so a rollout that recomputes layers could lose a real pump. That is
a pre-existing #24 determinization question; the counters contribution itself is
never recomputed or dropped. Out of scope for this delta.

### 2. `GameState.revealed: Vec::new()` — PASS on information safety; documentation wording NB
`crates/ai/src/search.rs:698-707` assigns a literal empty vector.
`grep -n revealed crates/ai/src/search.rs` shows `view.revealed` is never read
by the module, and `visible_ids` (`crates/ai/src/search.rs:426-461`) deliberately
omits `view.revealed`, so no revealed card's object entry is copied either.
Claim (a) holds: nothing hidden is read to build the field.

Claim (b), verified empirically with the deleted probe: because the ledger
excludes revealed ids, copying `view.revealed` ids alone yields a
`GameState.revealed` that names an id absent from `objects`.
- `Game::from_state`/`GameState::validate` do not check `revealed` references,
  so the state is accepted; `PlayerView` construction goes through
  `revealed_hidden()` (`crates/core/src/library.rs:104-113`), which filters
  missing ids, so the stale id is silently invisible and `legal_actions()`
  still works (probe output: `A from_state: Ok`, `A view.revealed: []`,
  `A legal actions: 2`).
- If the implementation also copied those objects to avoid the orphan id,
  validation rejects the world because the re-dealt hidden lists do not own the
  object: `InvalidState { reason: "object #2 in library has 0 zone-list and 0
  stack container entr(ies); exactly one matching container is required" }`
  (`crates/core/src/state.rs:1735`, container check), i.e. `search` would return
  `SearchUnavailable::ReconstructionRejected`.

So the deferral is justified, but the doc phrase "would reference an object the
world never places (a dangling reference)" overstates what is observable for the
minimal naive variant: the failure is a state-invariant violation that the
projection currently masks, not a panic or a rejected world. **Non-blocking**
documentation precision only; the production choice is sound because
`revealed` has no rules effect — the engine writes it in `engine.rs:5293` and
never reads it for legality — and the only cost is a publicly known card
identity missing from the re-dealt belief library (search quality, not safety).

A correct leak-free model is feasible and would look like this: add the
`view.revealed` ids to the ledger (they are public information), clone their
objects, substitute them into the owning seat's re-dealt library/list in reveal
order at the known top positions instead of generated belief ids (preserving the
real count), subtract them from `own_draws` when the viewer's own library is
partly known, and set `GameState.revealed` to those ids. The masked view is
sufficient to do this (revealed ids are public; zone/owner can then be read for
ledger ids), but it is extra modeling that changes belief-library composition
and is not required for this reconciliation.

### 3. New regression test — real, but does not discriminate the stated dangling case (NB)
`crates/ai/tests/search.rs:132-180` asserts the engine returns one reveal
(`reveal_top`), the masked view exposes it (`view.revealed.len() == 1`), search
succeeds, candidates are non-empty, and every candidate is in `view.legal`.
Those are genuine assertions on engine and search behavior, not tautologies.

The test does not assert the search world reports no reveals, nor anything
about the analysis being independent of hidden card identity; the "nor leaks a
hidden card" half of the name is unverified. More importantly, the naive
copy-ids-only variant of finding 2 passes this test (probe variant A above),
while the copy-object variant would be caught via `search` returning
`ReconstructionRejected`. So the test is a valid smoke/regression guard for
"a live reveal does not break search" but it does not discriminate the specific
dangling-reference mistake the doc comment names. **Non-blocking** test-coverage
inaccuracy; production code is correct.

RED was not available, as stated: `git show 2e97bb7:crates/ai/src/search.rs`
shows the `Object` literal without `counters` and the `GameState` literal
without `revealed` against a core state that already has both fields, and
`crates/ui/src/app.rs` had the same missing field — the pre-fix head did not
compile. That is confirmed from the committed blobs, not inferred from the
handoff.

### 4. Scope — PASS
`git diff 2e97bb7..385ac67 --stat` touches exactly
`crates/ai/src/search.rs`, `crates/ai/tests/search.rs`,
`crates/ui/src/app.rs` and `evidence/issue16/`. The `ui` change sits inside the
`#[cfg(test)]` module (`crates/ui/src/app.rs:2647`) in the test-only
`inject_large_board` fixture (`crates/ui/src/app.rs:4380-4414`). The diff is
insertion-only, so no production behavior and no existing assertion changed.

### 5. Evidence file accuracy — PASS with one NB wording note
`evidence/issue16/INTEGRATION_RECONCILIATION.md` is accurate on the merge tree
(`git rev-parse 2e97bb7^{tree}` = `6ab8da6bb3517bce9d98a5ee890ffb2f01864fff`),
on the gates (all three re-executed here exit 0), on the regression-test name,
and on the relocated probe
(`continuity/probes/scratch_pre16_replay_check.rs` exists). **Non-blocking**
imprecision: "a reveal stays inside a hidden zone list that this function
re-deals from beliefs" is not literally true for the viewer's own hand (real ids
are kept) or for the viewer's library when `own_draws` is supplied; only the
library path is relevant to `reveal_top` today.

## Non-blocking items, consolidated
1. `search.rs:698-707` / evidence: "dangling reference" describes an invariant
   violation masked by `revealed_hidden`, not an observable failure; the
   copy-object variant is the one validation rejects.
2. `search.rs:701-703`: the "re-deals from beliefs" justification is imprecise
   for the viewer's own known hand/library.
3. `crates/ai/tests/search.rs:132-180`: name/docstring overstate coverage; the
   test does not observe the search world's reveal state or a leak, and passes
   the naive copy-ids-only variant.

No blocking defect was found in this bounded delta. No merge, push, issue
closure, or production/test edit was performed; the only file written is this
report. The scratch probe was deleted and the worktree returned to a clean
tracked state.
