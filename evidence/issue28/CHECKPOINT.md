# CHECKPOINT — #28 move review, slice 1

`full_issue28_accepted=false` · card `t_bdd6bba7` · branch `aperture/issue-28` ·
base master `f3e8bb1` · draft PR: **_PR URL is recorded on the Kanban card_**.
Code head for every measurement below: **`ab71129`** (repair 1, after the
independent review BLOCK at `330fc25`; see `REPAIR_1.md`).

## Usable now

A read-only **MOVE REVIEW** panel (`Move review (V)` control / `V` key) that
ranks the engine-offered alternatives at the reviewing seat's current decision
point from a real `aperture_ai::search` run on a background worker:

* per candidate: engine label, mean **practice outcome**, scored samples,
  rollouts without a result, refused-root count;
* per analysis: the pinned compute budget, engine decisions actually used, the
  stopping rule, and the search's own determinization-fidelity line;
* a typed refusal (`SearchUnavailable` label, verbatim) whenever search cannot
  run, with **no ranking at all**;
* rules explanations and policy preference in separate labelled sections with
  separate source labels;
* staleness enforced on generation / revision / decision serial / seat, with
  `[stale_generation]`, `[stale_revision]`, `[stale_decision]`, `[wrong_seat]`,
  and cancellation of the running search when the table changes;
* nothing is submitted, granted or unlocked by the panel.

## Unverified / not built (no claim made)

1. **No candidate lines**: the search exposes no principal variation; the panel
   states that instead of inventing one.
2. **Current decision only**: reviewing an earlier ply needs a rewind surface
   this slice does not add.
3. **Criterion 4 not attempted** — teaching-quality validation on a reviewed
   interaction/strategy corpus stays blocked on **#17's certified starter
   lists**.
4. **No strength, win-probability, percentage or verdict claim** anywhere.
5. **No GPU screenshot**: the journey is a real click + real worker + rendered
   text readback, headless.

## Gates (this head)

All run at `ab71129`, each log carrying its own real exit status and the head
sha it was taken at (`git diff ab71129..HEAD -- crates/` is empty):

* `cargo fmt --all -- --check` — exit 0 (`fmt-check.log`)
* `cargo clippy --all-targets -- -D warnings` — exit 0 (`clippy.log`)
* `cargo test -p aperture-ui --lib` — **56 passed, 0 failed** (`ui-lib-tests.log`)
* `cargo test -p aperture-ui --lib review::` — **10 passed, 0 failed**
  (`tdd-green-review.log`)
* the recorded journey — pass, 6 of 18 policy lines rendered at 1280×720
  (`journey.log`)
* `cargo test --all-targets` — **83 suites ok, 585 passed, 0 failed**, exit 0
  (`full-tests.log`)
* `cargo build --release` — exit 0 (`release-build.log`)

Repair 1 (same branch, same draft PR): the chosen-row "equally scored"
qualifier now appears only on a real tie (`tie_at_top`), with
`review::tests::a_unique_maximum_is_not_described_as_an_equal_score` RED at the
pre-fix code (`tdd-red-review-repair.log`, exit 101) and GREEN after; the stale
`journey.log` / `tdd-green-review.log` were regenerated at this head rather than
relabelled.

## Budget used

`seeds=2`, `node_budget=30000`, `max_rollout_nodes=6000`, `max_millis=8000`,
`max_candidates=6` — measured, with the measurements in `MOVE_REVIEW.md`.

## Next gate

Park as a **dependency wait on #17's certified starter lists**. Do not
decompose, re-dispatch or run the gated teaching-quality evaluation from this
card until that corpus is certified.
