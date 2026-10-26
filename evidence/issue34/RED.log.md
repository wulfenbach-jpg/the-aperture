# #34 D1 — RED evidence captured before the fix

Base: `8de0961` (master) in worktree `C:/AI/aperture-worktrees/issue-34`,
branch `aperture/d1-discard-fix`. The new tests in this slice were added first and
run against the unmodified engine/AI/session code. Every line below is the
verbatim panic from that run.

## 1. The engine's offer for a cleanup discard contained one action, not a choice

```
$ cargo test -p aperture-core --test issue34_discard
running 4 tests
test multi_card_cleanup_discard_offer_is_bounded_and_legal ... FAILED
test the_offer_only_lists_the_choosing_seats_cards ... FAILED
test single_card_cleanup_discard_offers_every_hand_card ... FAILED
test invalid_discard_submissions_are_refused_with_the_state_untouched ... ok

---- single_card_cleanup_discard_offers_every_hand_card stdout ----
thread 'single_card_cleanup_discard_offers_every_hand_card' panicked at
crates\core\tests\issue34_discard.rs:68:5:
assertion `left == right` failed: expected one discard offer per hand card, got 1
  left: 1
 right: 8

---- multi_card_cleanup_discard_offer_is_bounded_and_legal stdout ----
thread 'multi_card_cleanup_discard_offer_is_bounded_and_legal' panicked at
crates\core\tests\issue34_discard.rs:123:5:
a real choice needs at least two offers

---- the_offer_only_lists_the_choosing_seats_cards stdout ----
thread 'the_offer_only_lists_the_choosing_seats_cards' panicked at
crates\core\tests\issue34_discard.rs:273:5:
assertion `left == right` failed
  left: 1
 right: 8
```

`invalid_discard_submissions_are_refused_with_the_state_untouched` passed before
the fix and still passes: `check_selection` was already strict, and the fix does
not weaken it.

## 2. The session dropped the AI's answer and then waited forever

```
$ cargo test -p aperture-ui --test issue34_discard
running 3 tests
test an_answer_bound_to_the_wrong_seat_is_still_refused ... ok
test both_seats_on_ai_autoplay_reach_a_terminal_result_with_a_verified_replay ... FAILED
test the_cleanup_discard_answer_is_applied_instead_of_dropped ... FAILED

---- both_seats_on_ai_autoplay_reach_a_terminal_result_with_a_verified_replay stdout ----
thread 'both_seats_on_ai_autoplay_reach_a_terminal_result_with_a_verified_replay'
panicked at crates\ui\tests\issue34_discard.rs:156:9:
step 327 dropped the AI's own legal answer:
Some("discarded an AI decision [illegal_choice] (not in the live legal offer)") ([])

---- the_cleanup_discard_answer_is_applied_instead_of_dropped stdout ----
thread 'the_cleanup_discard_answer_is_applied_instead_of_dropped' panicked at
crates\ui\tests\issue34_discard.rs:73:9:
the table never reached a cleanup discard (turn 1,
pending Some(BottomCards { player: 1, count: 1 }),
busy_note Some("discarded an AI decision [illegal_choice] (not in the live legal offer)"))
```

This is the packaged symptom reproduced headlessly, without the GUI: the same
`illegal_choice` drop, no crash, no further progress. It also showed that the
identical one-sample offer pattern stalls the **London mulligan bottoming**
choice (CR 103.5), which the packaged runs had not reached before the cleanup
one; both are fixed by the same change.

`an_answer_bound_to_the_wrong_seat_is_still_refused` passed before the fix: the
`wrong_seat` guard was never the problem and is untouched.
