# Slice 51 — review-333's three record items closed; public view now shows the orphan blocker

**ORIGINAL #17 REMAINS UNFINISHED: 1 of 8 lists certified, catalog 161, union 459 = 148 supported +
311 missing, Ayula 5 missing. This slice adds no card and no certificate.**

## 1. R333-1, R333-2, R333-3 (all executed, no functional defect claimed)

**R333-1 — the focused total is no longer mislabelled.** `SLICE50-HUMAN-CHECKPOINT.md` §3 now says
**11 suites / 129 passed** (slice46 14 + slice48 6 + slice49 8 + slice43 4 + combat 8 +
combat_teaching 2 + commander_combat 43 + m1_multiplayer 28 + ordering_persistence 2 +
replay_digest 6 + starter_corpus 8) and states the separate `aperture-ai search` suite **9**
explicitly, with the twelve-command total labelled an **aggregate 138**. `RESULT.json` now carries
`focused_suites: 11`, `focused_passed: 129`, a separate `separate_search_suite`, and an explicit
`aggregate_twelve_command_receipt_passed: 138` plus a note that nothing claims the 9 is excluded.

**R333-2 — the last stale comment is rewritten.** `crates/core/src/engine.rs` in
`move_to_graveyard_inner`, three lines above `combat.died_alone(id)`, now says the alone fact comes
from the **live pre-death membership captured once per batch by `capture_combat_alone`
(CR 704.3)**, that it must be captured before `detach_object` prunes the live lists, and that the
declaration record is **not** this predicate (CR 506.5). It no longer states the superseded
slice-47 reading. **Comment only — no engine behaviour changed.**

**R333-3 — code head and evidence tip are stated apart.** The checkpoint §4 and `RESULT.json.ci`
now publish `code_head dc9e99b` (10/10 exact-head checks SUCCESS) and `evidence_only_tip 1a6ae1c`
**separately**, with the tip's own CI pass explicitly *not* claimed.

Non-blocking precision items from the same review were also corrected in the slice-50 records rather
than deferred again: the journey rows say **content-identical after newline normalisation** (the
committed JSONs are CRLF), the "135 binaries" figure is restated as 135 result lines = 131 test
binaries + 4 doc-test runs, and the stale line numbers in `red-orphan-blocker.log` are labelled as
stale while the discrimination itself is preserved.

## 2. Review 333 N-1 fixed: the orphan blocker is now public combat state in `PlayerView`

Review 333 observed that `view.rs` projected `attackers`/`blockers` but **not** the slice-50
`unpaired_blockers`, so a seat's public combat state reported no blocking creature while the engine
still held one (CR 509.1g). `PlayerView` now carries `unpaired_blockers: Vec<ObjectId>`, populated
from the same engine field, **separately** from `blockers` — a blocker whose attacker has left is not
invented as a pair with a departed attacker.

Discrimination is executed, not asserted: with the projection temporarily replaced by `Vec::new()`
the committed combat regression fails

```
left: []
 right: [ObjectId(8)]
test result: FAILED. 0 passed; 1 failed; ... 13 filtered out
EXIT=101
```

(`slice51/red-view-projection.log`). With the projection restored the same test passes, and it now
asserts for **both seats** that `blockers` is empty, `attackers` is empty and
`unpaired_blockers == [blocker]` at the moment the attacker has been bounced. Docs §"Unpaired
blockers" carries the same statement. This changes no engine state, no digest and no rules outcome;
it is a projection/observability fix, not a policy change.

## 3. Executed evidence (all exit 0, `slice51/gates.log`)

* `cargo fmt --all --check`
* focused, 6 suites / **43 passed / 0 failed**: `issue17_slice46` 14, `issue17_slice48` 6,
  `issue17_slice49` 8, `starter_corpus` 8, `catalog_pool` 5, `issue17_slice15_corpus` 2
* `cargo test -p aperture-ai --test search --locked` → **9 passed / 0 failed** (separate crate,
  reported apart; the first attempt used `--lib search`, which matched **zero** test names — an own
  harness-path error, retained rather than hidden)
* corpus audit **AUDIT OK**: 161 catalog / 459 union / 148 supported / 311 missing, per-list
  `[55,0,51,48,5,52,52,53]`, Ayula 5, **1 certified**
* `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0
* `cargo build --workspace --release --locked` exit 0
* `cargo test --workspace --all-targets --locked` → **133 result lines / 947 passed / 0 failed**,
  exit 0
* one real four-seat 40-life journey re-executed from the committed driver and compared with the
  committed artifacts (see §4)

## 4. Journey and CI

* **Journey** (`slice51/journey.log`): the committed four-seat witness driver rebuilt and re-run at
  this head. Outcome `Win{P1}`, 3073 steps, 54 turns, 8098 events, digest
  `da4eacd5363fc16b6242d1c52fe86c2157c556d5e66a01696a6d6ba15b995bde`, with the three artifacts
  **content-identical after newline normalisation** to the committed ones. This is a 40-card
  evidence-only package, **not** a 100-card certificate and not AI-strength evidence.
* **CI**: code head and evidence tip are stated apart. Code head for this slice's product change and
  the CI result keyed to it are in `slice51/RESULT.json` `ci` and `slice51/exact-head-ci-*.json`;
  the published evidence-only tip has **no CI pass of its own** and none is claimed.

## 5. Persistence

`SAVE_VERSION` stays **10**, `REPLAY_VERSION` stays **6**. Slice 51 changes no serialized engine
field and no rules behaviour, so it moves no digest; `PlayerView` is a projection, not state.

## 6. What is deliberately not done — and the exact map for the next card

Ruxa, Patient Professor is **not** implemented and **not** enabled: the printed card needs three
independent mechanics, and enabling a half implementation is not allowed. This run re-read the
engine against all three and wrote the integration map in `slice51/NEXT-CARDS.md`
(Target/TargetSpec for a graveyard card, the no-abilities predicate, the layer 7c anthem, and a
typed damage-assignment pending), so the next slice does not re-derive it. Ayula still has five
missing names, the exact 100-card four-seat certification has **not** run, and 1 of 8 lists is
certified.

Children `t_26a7a4ff` (scheduled evaluation dependency wait) and `t_3c2fe51a` (unassigned soak
backlog) inspected and left gated. This run co-authored the increment and is **not** its independent
reviewer. No merge, master push, issue closure, downstream unlock, new team or provider/model-config
change.
