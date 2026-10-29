# Slice 56 checkpoint — review-355's four items closed; zero-toughness wording fixed; no card enabled

**Original #17 is NOT complete and this increment is NOT self-approved.** It stays at
**1 of 8 certified**, Ayula 3 missing, corpus 163 catalog / 459 union = 150 supported +
309 missing, per-list `[55, 0, 51, 48, 3, 52, 52, 53]`.

Same writer, same card, same branch `aperture/issue-17`, same draft PR46. No merge, no
master push, no issue closure, no dependent unlock, no new team, no global provider or
model-config change.

## Why no card was enabled this slice

One sentence, as the standing directive requires: **the next ordered package — announced-X
casting with aggregate-mana-value multi-target selection — cannot be enabled without the
card that exercises it (Rampaging Yao Guai), because an X-cost primitive with no X-cost
spell in the pool is untestable through `legal_actions`, and shipping Yao Guai's X clauses
without its aggregate-MV destroy trigger would be half a card; so this run closed
review-355's functional wording defect and its three record items instead of pretending to
deliver a primitive it cannot prove.**

The measured surface for that package, from this head (log `slice56/prereq-probe.log`):

| Need | State at this head |
|---|---|
| `{X}` in the casting cost | `mana.rs::parse_cost` has no `X` arm at all — the parser's match covers only `W U B R G C` and digits, so an `{X}{G}{G}` printing would silently lose its X. `card.rs::cost_mana_value` already scores `{X}` as 0, which is correct for a card outside the stack (CR 202.3b) and is *not* a casting path. |
| announced X carried through cast → stack → resolution | no `CastSpellWithX`, no X field on `StackItem`, no X in `Action` |
| aggregate-mana-value multi-target | `TargetSpec` has no any-number or total-mana-value variant; `validate_cast` requires exactly `targets.len() == 1` for every targeted spell, and `Pending::TriggerTarget` carries a single `legal` list plus an optional `first` |
| X-sized enter-with-counters + CR 603.4 trigger | not present |

## 1. R355-1 — the coach no longer calls a zero-toughness permanent "destroyed" (functional)

`crates/core/src/coach.rs` filed every creature with `toughness <= 0` under "has lethal
damage and will be destroyed". CR 704.5f moves a zero-or-less-toughness creature to its
owner's graveyard by state-based action — not a destroy — and there is no lethal damage
involved. The paragraph is now three lists: the **destroy** list is restricted to
`damage >= toughness > 0` minus indestructible (CR 704.5g / CR 702.12b), a separate
**zero-toughness** sentence cites CR 704.5f and says indestructible does not prevent it,
and the indestructible-and-spared list is unchanged.

**The RED is real and was produced before the repair.** With `crates/core/src/coach.rs`
restored from `4738a2b`, the two new tests fail on behaviour — `1 passed / 2 failed,
exit 101` (`slice56/red-r355-1.log`) — and the pre-existing indestructible test still
passes, so the new assertions discriminate. The restored file was written back and
re-hashed: `71d6bc7b9c55e4c712771bf2ecad17f1c689e6e66c02962ea19960842256dc79`.

## 2. R355-2/3/4 — the three record items, in this slice's opening commit

* **R355-2** — `docs/issue17-starter-corpus.md` limit (d) no longer claims a bounced source
  "logs the grant attempt". It states what the code does at this head: the mill still
  happens (CR 608.2b) and the log says the attack requirement, indestructible and +N/+N are
  skipped. `crates/core/tests/issue17_slice55.rs` is now named in that section's regression
  list.
* **R355-3** — the "left an effect targeting a card that is no longer a permanent"
  rationale is gone from all three sites (this checkpoint's predecessor §2,
  `slice55/RESULT.json` `product_defects_fixed[0]`, and the comment at
  `engine.rs:8534`). All three now state the observable defect — a **false event line**
  narrating a grant that never happened — and record that `recompute_layers` prunes effects
  whose target is not a battlefield permanent, so no recast incarnation was ever at risk.
* **R355-4** — the CI row is honest. `e4c5fcd`'s ten runs are **2 success + 8 cancelled**
  (this lane pushed evidence while the jobs ran, and `ci.yml` cancels in progress on the
  ref), so the code head has **no green gate**; `4738a2b`'s **10/10 completed success** is
  the green gate for the identical Rust tree. Recorded in `SLICE55-HUMAN-CHECKPOINT.md` §3
  and §4 and in a new `ci` object in `slice55/RESULT.json`, with the operating rule that
  evidence waits for CI.

## 3. What was executed at this head

Gate log with its own exit codes: `slice56/gates.log` (`slice56/run-gates.sh`).

| Check | Result |
|---|---|
| `cargo fmt --all --check` | `fmt_exit=0` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | `clippy_exit=0` |
| focused: `coach_trace`, `combat_teaching`, `issue17_slice54`, `issue17_slice54_regressions`, `issue17_slice55`, `review_regressions` | **6 suites / 34 passed / 0 failed** (7+2+4+6+5+10), `focused_exit=0` |
| whole-field corpus audit | **AUDIT OK** 163 / 459 = 150 + 309, per-list `[55,0,51,48,3,52,52,53]`, Ayula 3, 1 certified, no `missing_interactions` entry cites a supported card, `corpus_exit=0` |
| CI | keyed to this slice's pushed head — see §4 |

**Not run, and said plainly:** no four-seat journey. The delta is a coach sentence and
documentation; it creates no effect, moves no state and touches no digest input, so a
journey would reproduce the slice-54 recordings (`Win{P3}`, 1801 actions, 4596 events,
`cebf64e0…ff35fb`) without proving anything about this delta.

## 4. CI

Single push: the code, docs, records and evidence of this slice travel together, so the CI
run is keyed to the same commit tree that contains the product change — no evidence-tip
green is inherited and no cancelled job is reported as green.

**Result: `2ede45a` → 10 check runs, all `head_sha` keyed to `2ede45a`, 10 completed
success, 0 cancelled, 0 failed** (receipt `slice56/exact-head-ci-2ede45a.json`, captured
from `gh api …/commits/2ede45a/check-runs` after waiting for completion). The evidence
commit that follows it only rewrites this paragraph and the receipt; it claims no CI of its
own.

## 5. Next gate

Same writer, same history, same card, same branch, draft PR46. The supervisor's ordering
stands: **first** the announced-X plus aggregate-mana-value multi-target package, built as a
general primitive with its own casting, legality, save/load and offer tests — not a
card-shaped special case — then enable **Rampaging Yao Guai** in the same slice
(X counters on entry, CR 603.4 enter trigger, "any number of target artifacts and/or
enchantments with total mana value X or less" with a per-seat bounded offer and a budgeted
answer). Then **Dúnedain Rangers** (Ring emblem, Ring-bearer, four levels) and **Evercoat
Ursine** (two hideaway resolutions, linked face-down exile, optional free play).
Whole-field `missing_cards` **and** `missing_interactions` discipline whenever a card leaves
the gap list, then the exact 100-card four-seat certification with separate replays —
report **2 of 8 only if it passes**.
