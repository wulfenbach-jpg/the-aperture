# Move review — presentation layer (card t_808ff701)

Branch `wt/t_808ff701`, code commit `7cf4c0b` on top of the ranking layer
(`aperture_ai::review`, sibling card t_f4baa891) and the existing panel.

This increment replaces the panel's private view model with the ranking
contract. Nothing in `crates/ui/**` decides a candidate, a score, a line step or
a sentence: every one of them is read out of a `RankedReview` the ranking layer
produced (spec D-1). Only the presentation is authored here.

## What the panel now does

| requirement (card) | where | test |
| --- | --- | --- |
| candidate lines render strictly from the recorded legal traces | `push_line`, `line_steps` (`crates/ui/src/review.rs`) | `a_candidate_line_renders_from_the_recorded_trace_step_by_step`, `a_candidate_without_a_line_says_why_and_never_invents_one`, session `a_move_review_ranks_only_engine_offered_actions`, app journey |
| uncertainty is explicit, never a fake probability | `tie_heading`, `row_facts`, `push_ordering_claim` | `tied_rows_render_as_one_group_that_is_not_ordered_inside`, `a_single_sample_row_says_it_is_not_ordered`, `unscored_offers_are_listed_separately_and_not_ranked`, `zero_scored_samples_rank_nothing_and_count_the_attempts` |
| reason tags attached as a labelled heuristic, not a rule | `push_teaching_note`, `TEACHING_NOTE_LABEL` | `unscored_offers_are_listed_separately_and_not_ranked`, pinned snapshot |
| rules and policy structurally separate | `rules_section`, `MoveReview::sections`, `TextKind` → badge/block mapping | `rules_and_policy_blocks_are_structurally_separate`, `a_payload_rule_sentence_is_placed_in_the_rules_block_and_nowhere_else` |
| no percentage / probability string in rendered output | whole panel | `no_rendered_line_carries_a_percent_figure_or_a_certainty_claim`, session percent scan, pinned snapshot |
| degrades gracefully with one candidate | `push_ordering_claim`, sole-offer numbering | `a_sole_legal_offer_is_not_a_preference_and_is_not_ordered` |

Spec criteria owned by this card:

- **AC-7** — ties are one unordered group, one-sample rows are labelled, unscored
  offers are unranked (tests above; the group heading reads
  `effectively tied at +1.00 (2 scored samples each) — not ordered inside this group:`).
- **AC-8** — "the search's own choice among equally scored offers" appears only on
  a real tie; a unique maximum keeps "the search's own choice" (tie and snapshot
  tests, plus `a_single_sample_row_says_it_is_not_ordered` asserting the phrase is
  absent without a tie).
- **AC-11** — distinct constructors, titles, source labels and badges; strategies
  never reach the rules block; asserted on rendered output.
- **AC-12** — read-only: `a_move_review_never_submits_or_changes_the_table` now
  also checks priority and pending state, and re-renders the panel before
  re-checking the digest.
- **AC-14** — the binding is `(seat, generation, revision, decision, view_digest)`;
  `MoveReview::discard_reason` names each mismatch and `poll_workers` discards on
  it. New test `a_review_bound_to_another_masked_view_is_discarded` proves the
  digest arm (same seat/revision/decision, different masked projection).
- **AC-15** — every rendered step is a recorded engine-submitted step, a row with
  no recorded line says so, and the line is scoped to the search world. The
  end-to-end journey asserts that real recorded lines reach the panel.

## Deliberate presentation decisions

1. **Payload sentences render verbatim by kind.** Notes are placed with a
   `[POLICY]` / `[UNCERTAINTY]` / `[PROVENANCE]` badge; `Rule`-kind sentences are
   diverted to the rules block. The panel never re-words a payload sentence and
   never drops one.
2. **Rows are numbered only when an order is claimed.** With a tied or
   single-sampled leading row the rows are bullets, because numbering them would
   assert an order the payload denies. This showed up on a real position: two
   single-sample `+1.00` offers were numbered 1./2. before the change.
3. **Lines wrap instead of clipping.** `review_block` now uses a wrapping label;
   the 1024x640 viewport test (`the_move_review_panel_is_readable_at_the_minimum_1024x640`)
   caught the long sentence clipping in the side column.
4. **The panel adds no measurement.** No depth, no plies, no cost-per-option; the
   only work figure is the payload's budget line (`decisions_used` of
   `node_budget`).

## Evidence

- `render-gates.log` — `scripts/run-move-review-render-gates.sh` output with each
  command's own exit status (head recorded in the log).
- `render-journey.log` — the real end-to-end journey through the egui widget tree:
  click, background search, panel lines read back from the rendered frame.

## Out of scope / still open

- Criterion 4 (corpus validation) remains blocked on #17 certified starter lists;
  `full_issue28_accepted` stays false.
- The panel still shows one decision point at a time: earlier-ply review is spec
  open question Q4, not this card.
- The guardrail module (`crates/ai/src/review/honesty.rs`, card t_df6dd2e0) is not
  in this branch; this card's own percent/fragment scan covers the rendered
  output, and the guardrail's source-literal scan will additionally cover
  `crates/ui/src/review.rs` once both branches are integrated.
