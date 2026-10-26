# CHECKPOINT — #28 move review, slice 1

`full_issue28_accepted=false` · card `t_bdd6bba7` · branch `aperture/issue-28` ·
base master `f3e8bb1` · draft PR: **_PR URL is recorded on the Kanban card_**.

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

`cargo fmt --all --check` clean · `cargo clippy --all-targets -- -D warnings`
clean · `cargo test -p aperture-ui --lib` 55/55 · `cargo test --all-targets`
and `cargo build --release` results in `evidence/issue28/full-tests.log` and
`evidence/issue28/release-build.log`.

## Budget used

`seeds=2`, `node_budget=30000`, `max_rollout_nodes=6000`, `max_millis=8000`,
`max_candidates=6` — measured, with the measurements in `MOVE_REVIEW.md`.

## Next gate

Park as a **dependency wait on #17's certified starter lists**. Do not
decompose, re-dispatch or run the gated teaching-quality evaluation from this
card until that corpus is certified.
