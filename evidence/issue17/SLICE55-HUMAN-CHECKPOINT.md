# Slice 55 checkpoint — review-354's record items closed, two honesty defects fixed, no card enabled

**Original #17 is NOT complete and this increment is NOT self-approved.** It stays at
**1 of 8 certified**, Ayula 3 missing, corpus 163 catalog / 459 union = 150 supported +
309 missing, per-list `[55, 0, 51, 48, 3, 52, 52, 53]`.

Same writer, same card, same branch `aperture/issue-17`, same draft PR46. No merge, no
master push, no issue closure, no dependent unlock, no new team, no global provider or
model-config change.

Heads, stated apart as review 333/337 require:

| Role | Commit |
|---|---|
| Published evidence tip (this document) | see `git log -1` on `aperture/issue-17` |
| Product/code head with its own CI | `e4c5fcd` |
| Opening record commit | `a332e29` |
| Base (slice-54 evidence tip) | `55a55f6` |

## Why no card was enabled this slice

One sentence, as review 337 asked: **all three remaining Ayula names need a subsystem
that does not exist — X announcement with aggregate-mana-value multi-target selection for
Rampaging Yao Guai, the Ring emblem and Ring-bearer for Dúnedain Rangers, two hideaway
resolutions with linked face-down exile and free play for Evercoat Ursine — and none of
them is finishable inside this dispatch, so the run fixed two real user-facing defects in
the approved Ursine increment instead of shipping half of one card.**

## 1. Review 354's three record corrections (opening commit `a332e29`)

* **R354-1** — `README.md` no longer publishes the superseded declaration-only reading of
  Thijarian Witness. It now states the settled rule: CR 506.5 distinguishes "attacks
  alone" (declaration) from "is attacking alone" (live membership), the printed past tense
  names the live state, and the answer is one pre-departure snapshot per simultaneous
  batch (CR 704.3). The two real residual limits replace the false "rules gap": a save
  captured mid-combat before that record existed re-executes to a different digest, and
  `GameState::validate` does not constrain the recorded declaration lists.
* **R354-2** — `docs/issue17-starter-corpus.md` §Phased gaps now says the implemented pool
  is **163** cards, matching README:24 and docs:34/47/56. The slice-53 own-time "162"
  passes through untouched as history.
* **R354-3** — slice 54 has its own documentation section (`## Slice 54: Ursine
  Monstrosity — the CR 508.1d maximal attack requirement and indestructible`): the
  printing and retrieval date, the three clauses, the deliberate limits (maximisation
  inside the inherited single-defender model, indestructible not preventing a
  zero-toughness graveyard move, the six-kind graveyard count fixed at resolution, the
  combat-scoped requirement vs the until-end-of-turn pump, the `MustAttackThisCombat`
  controller-field note, the bounced-source log line), and both regression files. README's
  rules-engine list now names the CR 508.1d maximal attack requirement and indestructible.

## 2. Two product defects from the same review, fixed with a discriminating RED

**N-1 — the Ursine trigger narrated a grant it had skipped.** The effect pushed its
until-end-of-turn indestructible continuous effect, and its "gains
indestructible/+1/+1" event line, *before* checking whether the source was still a
battlefield permanent. A trigger that resolved after its source was bounced or destroyed
therefore still logged the full grant. **The observable defect was that false event
line** (it claimed an attack requirement and an indestructible/+1/+1 grant that never
happened), not a surviving effect: `recompute_layers` drops every effect whose target is
not a battlefield permanent, so a stale effect would have been at most transient and no
later recast incarnation was ever at risk. That earlier "left an effect behind" rationale
was wrong and is corrected here. The grant is now created only while the source is a
battlefield permanent, the mill still happens (CR 608.2b), and the event line states
which of the two happened.

**N-4 — the coach promised the death of an indestructible creature.** The state-based
action paragraph listed every creature with `damage >= toughness` under "will be
destroyed". CR 702.12b exempts an indestructible permanent from that destroy, while zero
toughness still moves it to the graveyard (CR 704.5f). The paragraph is now split into a
destroyed list and an indestructible-and-spared list, each carrying its own rule
citations.

**The RED discriminates and was produced against the pre-fix sources.** With
`crates/core/src/{coach,engine}.rs` restored from `a332e29`, the new suite is
**1 passed / 2 failed, exit 101** (`slice55/red-honesty.log`): both new assertions fail on
behaviour, and the third test — the "source still present, the grant is still narrated"
control — passes in both states, so the new wording did not silence the ordinary case.
`has_indestructible` was widened to `pub(crate)` for the coach read; its one existing
caller is unchanged.

## 3. What was executed at this head

| Check | Result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy -p aperture-core --all-targets --locked -- -D warnings` | exit 0 |
| focused suites: `coach_trace`, `combat_teaching`, `issue17_slice54`, `issue17_slice54_regressions`, `issue17_slice55`, `review_regressions` | **6 suites / 32 passed / 0 failed** (7+2+4+6+3+10), exit 0 |
| whole-field corpus audit | **AUDIT OK** 163 / 459 = 150 + 309, per-list `[55,0,51,48,3,52,52,53]`, Ayula 3, 1 certified, no `missing_interactions` entry cites a supported card |
| CI at the code head `e4c5fcd` | **no green gate**: 10 runs = 2 success + 8 cancelled by this lane's own evidence push (`ci.yml` `cancel-in-progress`); the green gate is `4738a2b`'s 10/10 completed success on the identical Rust tree — see §4 |

**Not run, and said plainly:** no four-seat journey was re-executed. The change is a log
line and a coach projection; no serialized field, digest input, priority, legality, combat
or policy behaviour changed, so a journey would reproduce the slice-54 recordings
(`Win{P3}`, 1801 actions, 4596 events, `cebf64e0…ff35fb`) rather than prove anything about
this delta. The exact-head CI run's workspace test pass is the full-suite gate here.

## 4. CI

Code head **`e4c5fcd`**; the evidence commit that publishes this checkpoint is evidence-only
and claims no CI of its own. The receipt is `slice55/exact-head-ci-e4c5fcd.json`, captured
from `gh api …/commits/e4c5fcd/check-runs`.

**The code head has no green gate and is not reported as one.** `e4c5fcd`'s ten check runs
are **2 success + 8 cancelled**: this lane pushed its evidence commit while the jobs were
still running, and `ci.yml` cancels in-progress runs on the same ref. The green gate for
the same Rust tree is **`4738a2b`'s 10/10 completed success** — the only delta between the
two commits is `evidence/`, so that run compiled and tested exactly the code reviewed as
`e4c5fcd`. `RESULT.json` carries the same statement in its `ci` object. Operating rule for
later slices: wait for CI to complete before pushing evidence, and never report a cancelled
job as green.

## 5. Next gate

Same writer, same history, same card, same branch, draft PR46: open the next product slice
with **Rampaging Yao Guai** and build the three primitives it actually needs as one package
— announced-X casting (`CastSpellWithX`, `{X}` in the cost parser, payment plan,
`StackItem` X with zero-omitted serialization), an aggregate-mana-value "any number of
target artifacts and/or enchantments" target specification (per-target legality, no
duplicates, sum of mana values ≤ X, bounded offer enumeration), and X-sized enter-with
counters plus the CR 603.4 trigger carrying the announced X. Then Dúnedain Rangers (Ring
emblem, Ring-bearer, four levels) and Evercoat Ursine (two hideaway resolutions, linked
face-down exile, optional free play). Whole-field `missing_cards` **and**
`missing_interactions` discipline on every card that leaves the gap list, then the exact
100-card four-seat certification with separate replays — report **2 of 8 only if it
passes**.
