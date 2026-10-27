# #28 slice 1 — move review: ranked engine-offered alternatives with stated uncertainty

Card `t_bdd6bba7` · worktree `C:/AI/aperture-worktrees/issue-28` · branch
`aperture/issue-28` · base master `f3e8bb1` · owner surface: `crates/ui/**` and
`evidence/issue28/**` only.

This is a **bounded increment**. It does **not** claim #28 acceptance, engine
strength, stronger play, or a v1 milestone. `full_issue28_accepted=false`.

Repaired after the independent review of `330fc25` returned BLOCK (findings B1
and F1; see §"Repair 1" at the end and `REPAIR_1.md`). The code head these
measurements were taken at is **`ab71129`**.

## What is usable now

* A **MOVE REVIEW** panel in the native app, opened with the `Move review (V)`
  control in the action strip or the `V` key.
* It ranks the **engine-offered alternatives at the reviewing seat's current
  decision point** using `aperture_ai::search` on a background worker, with the
  viewer's masked `PlayerView` and the viewer's own declared deck list as the
  only observation.
* Every row shows the engine's own action label, the mean **practice outcome**
  (`win +1 / loss −1 / draw 0` over scored determinizations), the number of
  scored samples, the rollouts that produced **no result**, and how often the
  search world refused the root action.
* The row the search picked is marked as its own choice. It is called an
  *equally scored* choice only when another ranked row really ties the top mean;
  on a unique maximum the marker says only `the search's own choice`.
* The panel discloses its own compute budget, the engine decisions it actually
  used, the stopping rule, and the search's verbatim determinization-fidelity
  line (`opponents: neutral belief deck (N opponent(s)); no real hand or library
  order read; …`).
* A **typed refusal** is a first-class panel state: the search's own
  `SearchUnavailable` label is printed verbatim and nothing is ranked.
* **Rules text and policy preference are structurally separate sections** with
  different titles, different source labels and different constructors
  (`rules_section()` accepts only `coach::Explanation` records of the rule and
  simplification kinds; `MoveReview::policy_section()` is the only ranking
  producer).
* **Staleness** is enforced on `generation`, `revision`, `decision_serial` and
  `seat`, with the codes `[stale_generation]`, `[stale_revision]`,
  `[stale_decision]`, `[wrong_seat]`; a table change cancels the running search.
* **Read-only**: the panel carries no `Action` at all (the report type stores
  labels, never engine actions), submits nothing, grants no priority, does not
  enter omniscient postgame review, and does not change whether undo is offered.

## What is explicitly unverified or not built

1. **No candidate lines.** The search returns root `CandidateScore` values only
   — it exposes **no principal variation**. The panel says exactly that
   (`lines: the search exposes no principal variation …`) instead of
   synthesising a line by replaying a rollout. Criterion 2 of the card ("show
   candidate lines") is therefore **partially** met: uncertainty and the reason
   for each alternative are shown from the analysis' own counts, but no
   played-out line exists to show.
2. **Only the current decision point can be reviewed.** The panel reviews the
   decision the engine is offering the reviewing seat *now*. Reviewing an
   earlier ply would need a rewind surface that replays a recorded prefix and
   runs the search against the reconstructed past position; that surface is not
   part of this slice and is **not** claimed. (The existing postgame omniscient
   review and the study/branch panels are unchanged.)
3. **Criterion 4 — teaching-quality validation on a reviewed interaction /
   strategy corpus — is untouched and stays blocked on #17's certified starter
   lists** (1 of 8 certified at the time of writing). No teaching-quality claim
   is made anywhere in this increment.
4. **No strength claim.** Nothing in the code, tests, panel text or this file
   says the search plays better than the heuristic, and no win probability,
   percentage or "best move / blunder / mistake" verdict is produced. In the
   recorded journey the same search scored several alternatives identically at
   one sample each — exactly the situation where a confident verdict would be
   false, and the panel says so instead.
5. **No native (GPU) screenshot.** The journey below is driven through the real
   widget tree and the real worker (real pointer click, real frames, ranking
   read back out of rendered text), but no window surface was captured. It is
   interaction evidence, not appearance evidence.
6. **Rendering is 4-seat practice measured.** The search budget was tuned by
   measurement (see below); a two-seat table scores all offers comfortably
   inside the same budget, while a four-seat bounded table needs the larger
   per-rollout cap.

## Search budget actually used

`REVIEW_BUDGET` in `crates/ui/src/review.rs`:

| field | value |
| --- | --- |
| `seeds` | 2 |
| `node_budget` | 30 000 engine decisions |
| `max_rollout_nodes` | 6 000 per rollout |
| `max_millis` | 8 000 ms wall clock |
| `max_candidates` | 6 |

Why these numbers — measured, not guessed (temporary probe, deleted before
publication; `cargo test -p aperture-ui --test review_probe -- --nocapture`):

```
seats=2 offers=6 nodes=1500  millis=4000  -> decisions=2568 scored=6/6 without=0 elapsed=2.27s
seats=2 offers=6 nodes=6000  millis=20000 -> decisions=2568 scored=6/6 without=0 elapsed=2.27s
seats=4 offers=7 nodes=1500  millis=4000  -> decisions=2726 scored=1/6 without=1 elapsed=4.00s   (per-rollout cap hit)
seats=4 offers=7 nodes=6000  millis=20000 -> decisions=8540 scored=6/6 without=0 elapsed=13.2s   (debug build)
seats=4 offers=7 nodes=20000 millis=60000 -> decisions=8540 scored=6/6 without=0 elapsed=13.1s
```

At `max_rollout_nodes = 1500` a four-seat rollout never reaches a result, so
every row would read "no scored sample" — a surface that can never rank
anything. 6 000 is the smallest measured cap that lets a four-seat rollout
finish. The wall clock is the binding limit in debug builds, which is why the
panel states its stopping rule and reports the counts it actually obtained.

## Tests actually run (result lines)

Every log in this table was produced by the command it names, in
`C:/AI/aperture-worktrees/issue-28`. The table contains two different kinds of
artifact, and only the first kind is a head measurement:

- **Head runs** — `journey.log`, `tdd-green-review.log`, `ui-lib-tests.log`,
  `fmt-check.log`, `clippy.log`, `full-tests.log`, `release-build.log`: each was
  run at code head **`ab71129`** and carries a header with the head sha, branch,
  UTC capture time and the command, plus the command's own exit status. The
  commits after `ab71129` are evidence-only (`git diff ab71129..HEAD -- crates/`
  is empty), so those logs describe exactly the Rust code the branch publishes.
- **Earlier working-tree transcripts (no head header, no exit-status line)** —
  `tdd-red-review.log`, `tdd-red-session.log`, `tdd-green-session.log`,
  `tdd-mutation-red-app.log`: captured while the slice was being built, in
  working trees whose exact commit is not recorded inside the file.
  `tdd-red-review.log` and `tdd-red-session.log` show their tests failing before
  the implementation existed; `tdd-green-session.log` shows the session tests
  passing at a later point in that same build-up; `tdd-mutation-red-app.log`
  shows the review renderer and journey tests failing with the panel render
  disabled. They are kept as the RED/GREEN sequence each test was written
  against, not as head measurements.
  `tdd-red-review-repair.log` and `mutation-red-review-repair.log` do declare
  their own provenance in a header: the former against the pre-fix working tree
  at `330fc25` (the last commit before the F1 fix), the latter against
  `ab71129` with the equality qualifier deliberately forced on for every chosen
  row, ending in its own `exit status: 101`.

| command | result | log |
| --- | --- | --- |
| `cargo test -p aperture-ui --lib review::` (RED, stubs) | 1 passed, 7 failed — `not implemented` | `tdd-red-review.log` |
| `cargo test -p aperture-ui --lib review::` (RED, repair-1 F1 test against the pre-fix code) | 1 failed — `a unique maximum must not be described as an equal score`, exit 101 | `tdd-red-review-repair.log` |
| `cargo test -p aperture-ui --lib review::` (GREEN) | 10 passed, 0 failed | `tdd-green-review.log` |
| `cargo test -p aperture-ui --lib session::tests` (RED, stubs) | 1 passed, 6 failed | `tdd-red-session.log` |
| `cargo test -p aperture-ui --lib session::tests` (GREEN) | 7 passed, 0 failed | `tdd-green-session.log` |
| `cargo test -p aperture-ui --lib` (mutation: panel render disabled) | 0 passed, 3 failed — every review renderer/journey test fails without the panel | `tdd-mutation-red-app.log` |
| `cargo test -p aperture-ui --lib review::` (mutation: chosen-row qualifier forced on, `ties_top` dropped) | 9 passed, 1 failed — only `a_unique_maximum_is_not_described_as_an_equal_score` catches it | `mutation-red-review-repair.log` |
| `cargo test -p aperture-ui --lib` | 56 passed, 0 failed | `ui-lib-tests.log` |
| `cargo fmt --all -- --check` | clean, exit 0 | `fmt-check.log` |
| `cargo clippy --all-targets -- -D warnings` | clean, exit 0 | `clippy.log` |
| `cargo test --all-targets` | 83 suites `test result: ok`, 585 passed, 0 failed, exit 0 | `full-tests.log` |
| `cargo build --release` | `Finished release profile [optimized]`, exit 0 | `release-build.log` |

Behaviour-to-test map:

| required behaviour | test |
| --- | --- |
| panel reachable and readable at the enforced minimum 1024×640 (clip-rect readback, not label presence) | `app::viewport_tests::the_move_review_panel_is_readable_at_the_minimum_1024x640` |
| a search refusal is displayed and nothing is ranked | `review::tests::a_refusal_is_typed_and_ranks_nothing`, `session::tests::a_move_review_refusal_is_typed_and_ranks_nothing`, `app::viewport_tests::the_move_review_refusal_and_stale_states_show_no_ranking_at_the_minimum` |
| a zero-sample analysis is not an ordering claim | `review::tests::a_zero_sample_analysis_makes_no_ordering_claim` |
| a stale generation / decision / seat result is discarded and does not replace the panel | `session::tests::a_stale_move_review_does_not_replace_the_current_panel`, `session::tests::a_move_review_for_another_seat_is_discarded`, `session::tests::a_table_change_drops_the_review_and_stops_the_worker`, `app::viewport_tests::the_move_review_refusal_and_stale_states_show_no_ranking_at_the_minimum` |
| rules text and policy preference in separate labelled sections | `review::tests::rules_and_policy_are_separate_labelled_sections`, `app::viewport_tests::the_move_review_panel_is_readable_at_the_minimum_1024x640` |
| ranked alternatives only from engine offers | `session::tests::a_move_review_ranks_only_engine_offered_actions`, `review::tests::a_completed_search_report_keeps_only_offered_labels_and_counts` |
| read-only review semantics | `session::tests::a_move_review_never_submits_or_changes_the_table`, `app::viewport_tests::the_v_shortcut_opens_the_move_review_without_acting` |
| no certainty words anywhere in the panel | `review::tests::no_line_claims_certainty_or_strength` |
| displayed order never contradicts the stated preference on a tie | `review::tests::an_equal_score_is_displayed_in_the_searchs_own_order` |
| a unique maximum is not described as an equal score | `review::tests::a_unique_maximum_is_not_described_as_an_equal_score` |
| end-to-end journey | `app::viewport_tests::the_move_review_journey_reads_its_ranking_back_from_the_interface` |

## The recorded journey

`cargo test -p aperture-ui --lib app::viewport_tests::the_move_review_journey -- --nocapture`
(`journey.log`). The journey:

1. builds the real four-seat bounded table (`UiSession::bundled_seats`, profile
   `CommanderCombat40`, seed 1) and keeps every opening hand;
2. walks the **legal trace only** (engine-offered actions, AI seats stepped by
   the frozen heuristic) until the human seat faces a real choice;
3. **clicks** the `Move review (V)` control with a real pointer event in the
   real frame loop;
4. pumps frames while the worker runs (the frame callback never blocks);
5. reads the ranking back out of the rendered text and scrolls it into view.

Recorded output:

```
journey: reached the reviewing seat's decision (turn 3, step precombat main phase, 6 engine offers)
journey: review applied after 20 frame(s) · refused=None · 6 candidate(s) · 5298 engine decisions · budget ReviewBudget { seeds: 2, node_budget: 30000, max_rollout_nodes: 6000, max_millis: 8000, max_candidates: 6 }
journey: policy section lines read back from the panel:
  reviewed decision point: P0 · engine revision 98 · decision 98
  budget: 2 determinization seed(s) · 30000 engine decisions total · 6000 decisions per rollout · 8000 ms wall clock · at most 6 engine-offered candidates
  used: 5298 of 30000 engine decision(s)
  stopping rule: the analysis ends at whichever of the node budget, the per-rollout cap or the wall clock it reaches first, so a partial sample is normal
  fidelity: opponents: neutral belief deck (3 opponent(s)); no real hand or library order read; own future draws sampled from declared deck "Temur Beats" minus visible cards
  lines: the search exposes no principal variation — it returns root candidate scores only, not a played-out line — so no candidate line is shown here rather than synthesising one
  policy preference (a preference from scored practice outcomes — not a rule, and not a claim of strength): "Play Forest"
  ordering: highest mean practice outcome first
  score: mean practice outcome = win +1 / loss -1 / draw 0 over scored determinizations; a capped or failed rollout is counted as "without result", never as a win or a loss
  uncertainty: sample counts differ between rows and more samples do not make a row better; these samples are small and small samples are not evidence of strength
  1. Play Forest — mean +1.00 over 1 scored sample · 0 without result · 0 refused by the search world · the search's own choice among equally scored offers
  2. Pass priority — mean +1.00 over 1 scored sample · 0 without result · 0 refused by the search world
  3. Play Mountain — mean +1.00 over 1 scored sample · 0 without result · 0 refused by the search world
  unscored offers (no rollout reached a result; listed in engine offer order and not ranked against each other):
  — Play Mountain — no scored sample (mean: none) · 1 without result · 0 refused by the search world
  — Play Mountain — no scored sample (mean: none) · 0 without result · 0 refused by the search world
  — Play Forest — no scored sample (mean: none) · 0 without result · 0 refused by the search world
  this panel reports the engine's own counts and the search's own preference only: it passes no verdict on your play, and it labels no alternative as an error
journey: 6 of 18 policy lines were present in the rendered frame at 1280x720
```

Three alternatives tie at `+1.00` on a single sample each, three more produced no
result at all — and the panel says exactly that. This is the honest shape of the
result under this budget; it is why no verdict is printed.

This block is the verbatim body of `journey.log` (captured at code head
`ab71129`, exit 0). The decision count varies between runs because `max_millis`
is a wall-clock stop: the review at `330fc25` observed 4 780, this capture
reports 5 298, and the lane's original run reported 5 387 — same deterministic
shape (turn 3, 6 offers, 3 scored rows, 3 unscored), different timing. Read the
line from the log, not from memory.

**At least one refusal case** was reached and is recorded in the tests: at the
opening the engine's decision is a pending mulligan, so the search answers
`SearchUnavailable::PendingDecision { kind: "mulligan" }` and the panel prints
`refused: non-priority decision (pending kind mulligan); search not enabled
there` with `ordering: none` and zero candidate rows
(`session::tests::a_move_review_refusal_is_typed_and_ranks_nothing`).

`SearchUnavailable::RootRefused` from the work order does not exist as a
whole-decision refusal in `aperture_ai::search`: a refused root action is a
**per-rollout** outcome (`Rollout::RootRefused`) and is surfaced per candidate
as `refused_times` ("N refused by the search world"), which the panel prints for
every row. That is stated here rather than faked into a decision-level refusal.

## Ownership / scope

Changed (additive):

* `crates/ui/src/review.rs` (new): the report model, the labelled sections, the
  refusal and uncertainty text, and their unit tests.
* `crates/ui/src/session.rs`: `WorkerMessage::MoveReview`,
  `UiSession::{review, review_busy, request_move_review,
  request_move_review_with, review_fresh}`, the poll/stale arm, cancellation on
  `invalidate_pending_work`, and six focused tests.
* `crates/ui/src/app.rs`: the panel, the control, the `V` shortcut, the help
  text, and four viewport/journey tests.
* `crates/ui/src/lib.rs`: `pub mod review;`.

Not touched: `crates/core/**`, `crates/ai/**`, `crates/import/**`,
`crates/cli/**`, decks, catalog, import, eval, `evidence/issue17/**`,
`evidence/issue26/**`. No new dependency, no new crate.

## Checkpoint

`full_issue28_accepted=false`. Criterion 4 remains blocked on **#17's certified
starter lists** (the reviewed interaction/strategy corpus does not exist yet);
the card should park as a dependency wait on that corpus, not decompose.

## Repair 1 — after the independent review BLOCK at `330fc25`

The independent review (`REVIEW_28_independent.md`, card attachment 104) passed
every behavioural finding and blocked on two items. Both are fixed at code head
`ab71129`; nothing else was re-litigated and no assertion was weakened.

**B1 (blocking) — committed evidence did not correspond to the reviewed head.**
`journey.log` and `tdd-green-review.log` had been produced before the final
panel text and before the tenth `review::` test, yet this file labelled every
log a run "at the published head". Fixed by regenerating both at `ab71129` with
their real exit statuses inside the log, regenerating the other gate logs at the
same head, and replacing this file's blanket claim with the head each log was
taken at. Checkable invariants now:

* `tdd-green-review.log` → `test result: ok. 10 passed` and names
  `a_unique_maximum_is_not_described_as_an_equal_score`.
* `journey.log` quotes text that exists in the head source, e.g.
  `grep -F "it labels no alternative as an error" crates/ui/src/review.rs` hits
  and `grep -F "it names no alternative as the one to play" crates/ui/src/` does
  not.
* `git diff ab71129..HEAD -- crates/` is empty, so the logs describe the
  published code.

**F1 (must fix) — false equality on the chosen row.**
`candidate_line` appended "among equally scored offers" whenever the search's
pick led the list, so `+1.00` and `−0.50` on one screen were described as an
equal score. The qualifier is now emitted only when another ranked row actually
ties the top mean (`tie_at_top` in `policy_section`); otherwise the row says only
`the search's own choice`. Test:
`review::tests::a_unique_maximum_is_not_described_as_an_equal_score` — RED at the
pre-fix code (exit 101, `tdd-red-review-repair.log`, whose panic quotes the
contradictory panel line), GREEN after the fix. The tie path stays covered by
`review::tests::an_equal_score_is_displayed_in_the_searchs_own_order`.

Unchanged by the repair: the exported behaviour, the pinned budget, staleness
discard, read-only semantics, and the criterion-4 dependency on #17's certified
starter lists. `full_issue28_accepted=false`.
