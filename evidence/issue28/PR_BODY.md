#28 Move review: ranked alternatives with stated uncertainty (increment)

Bounded slice 1 of #28 (`strategy-coach`, M4). **Draft on purpose.**
`full_issue28_accepted=false` — this does not claim #28 acceptance, engine
strength, stronger play, or a v1 milestone.

Repair 1 landed on this branch after an independent review of `330fc25` returned
BLOCK: the chosen-row "equally scored" qualifier is now printed only on a real
tie (a unique maximum says only "the search's own choice"), and the gate logs —
including `journey.log` and `tdd-green-review.log` — were regenerated at code
head `ab71129` instead of being relabelled. Details in `evidence/issue28/REPAIR_1.md`.

## What this adds

A read-only **MOVE REVIEW** surface in the native table (`Move review (V)`
control in the action strip, `V` key, panel beside the coaching block):

* ranks the **engine-offered alternatives** at the reviewing seat's current
  decision point from a real `aperture_ai::search` run, on a background worker,
  over the viewer's masked `PlayerView` and the viewer's own declared deck list
  only. Candidates are built by the search from `view.legal`; the UI never
  invents an alternative and the report type stores engine **labels**, never
  `Action`s, so the panel cannot submit anything.
* per candidate: engine label, mean **practice outcome** (win +1 / loss −1 /
  draw 0 over scored determinizations), scored samples, rollouts without a
  result, and how often the search world refused the root action.
* per analysis: pinned compute budget, engine decisions actually used, the
  stopping rule, and the search's own determinization-fidelity line verbatim
  (`opponents: neutral belief deck (N opponent(s)); no real hand or library
  order read; …`).
* **typed refusal is a first-class outcome**: the search's own
  `SearchUnavailable` label is printed verbatim and nothing is ranked; there is
  no silent empty panel.
* **rules and preference stay apart** structurally: `rules_section()` accepts
  only `coach::Explanation` records of the rule/simplification kinds, and
  `MoveReview::policy_section()` is the only ranking producer. Different
  titles, different source labels, different constructors.
* staleness on `generation` / `revision` / `decision_serial` / `seat` with
  `[stale_generation]`, `[stale_revision]`, `[stale_decision]`, `[wrong_seat]`;
  a table change cancels the running search and drops the stale panel.
* read-only: no submission, no priority, no sandbox/undo unlock, does not
  enter omniscient postgame review.

## Budget (measured, not guessed)

`seeds=2`, `node_budget=30_000`, `max_rollout_nodes=6_000`, `max_millis=8_000`,
`max_candidates=6`. A four-seat bounded-table rollout never reaches a result at
1 500 per-rollout nodes (it only ever produced "no scored sample" rows), so the
cap was raised to the smallest value measured to finish; the measurements are
in `evidence/issue28/MOVE_REVIEW.md`.

## What is explicitly not here

* **No candidate lines.** The search returns root `CandidateScore` values only
  and exposes no principal variation; the panel says exactly that instead of
  synthesising a line from a rollout.
* **Current decision only.** Reviewing an earlier ply would need a rewind
  surface this slice does not add.
* **Criterion 4 (teaching-quality validation on a reviewed interaction/strategy
  corpus) is untouched and stays blocked on #17's certified starter lists.**
* No win probability, no percentage, no "best move / blunder / mistake", no
  strength claim — `review::tests::no_line_claims_certainty_or_strength` pins
  that over every panel state.
* No GPU screenshot: the journey is a real pointer click, the real worker and
  the ranking read back out of rendered text, driven headlessly through the same
  widget tree the window draws.

## Evidence

`evidence/issue28/`:

* `MOVE_REVIEW.md` — full acceptance-shaped report: behaviour, budget
  measurements, behaviour→test map, the recorded journey output, scope.
* `CHECKPOINT.md` — what is usable, what is unverified, gates, next gate.
* `tdd-red-review.log`, `tdd-green-review.log`, `tdd-red-session.log`,
  `tdd-green-session.log`, `tdd-mutation-red-app.log` — RED before GREEN, plus a
  mutation run (panel render disabled → all three renderer/journey tests fail,
  so they are not vacuous).
* `journey.log` — the end-to-end journey at 1280×720 (turn 3 precombat main,
  6 engine offers, 5 298 engine decisions, ranked rows + honest "no scored
  sample" rows read back from the rendered panel; the decision count moves with
  the wall-clock stop, the shape does not).
* `tdd-red-review-repair.log` — the repair's RED (exit 101) for the
  unique-maximum wording test.
* `REPAIR_1.md` — each review finding, its fix, and how to re-check it.
* `ui-lib-tests.log`, `clippy.log`, `fmt-check.log`, `full-tests.log`,
  `release-build.log`.

## Gates at code head `ab71129`

`cargo fmt --all -- --check` exit 0 · `cargo clippy --all-targets -- -D warnings`
exit 0 · `cargo test -p aperture-ui --lib` 56/56 · `cargo test --all-targets`
83 suites ok, 585 passed, 0 failed · `cargo build --release` exit 0. Each log
records its own exit status and the head it was taken at; the commits after
`ab71129` are evidence-only (`git diff ab71129..HEAD -- crates/` is empty).

## Scope

Only `crates/ui/**` and `evidence/issue28/**`. No change to `crates/core`,
`crates/ai`, `crates/import`, `crates/cli`, catalog, import, eval, bundled decks,
`evidence/issue17/**` or `evidence/issue26/**`. No new crate, no new dependency.

Draft: no merge, no un-draft, no master push, no issue close, no successor
activation requested from this PR.
