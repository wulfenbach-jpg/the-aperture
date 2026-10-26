# Slice 17 TDD record

Production was scaffolded (CardId variants, catalog records, effect enum arms as no-ops)
before the regression test, so the engine could compile; the test then failed on behavior,
not on a missing symbol.

## RED (before mechanics)

Command: `cargo test -p aperture-core --test issue17_slice17`

Initial run failed to pay Shock because the fixture had no Mountain; that was a test-setup
bug, fixed by adding a Mountain inside the kill helper. The next run (mechanics still no-op):

```
failures:
    caller_counts_owned_nontoken_battlefield_deaths_even_if_card_later_leaves
    friendly_teddy_dies_each_live_player_draws_after_source_left
...
test result: FAILED. 2 passed; 2 failed
```

- Friendly Teddy: no player drew (EachPlayerDraws arm was `false`).
- Caller: 0 Bear tokens (count arm was `false`).
- Ulvenwald initially passed only because the test tolerated "no target chosen"; the test was
  strengthened to assert exactly two +1/+1 counters on a pre-existing target, and it passes.
- Sourced-card conformance already passed for printed text/types/costs.

## GREEN

Implemented death history (serde-default/skip-empty, maintained only while Ulvenwald or
Caller is present), cleanup reset, intervening-if skip, APNAP each-player draw, targeted
+1/+1 counters, and owned nontoken death counting.

Command: repair RED->GREEN:
- `slice17_cleanup_history_tests::repeated_cleanup_keeps_earlier_deaths_until_the_turn_number_advances`
  failed while cleanup cleared history; now calls real cleanup twice and passes while
  `turn.number` is unchanged.
- `command_zone_setup::each_player_draw_continues_every_live_seat_after_a_commander_replacement`
  failed because the EachPlayer resume was absent (and then because save validation rejected
  a triggered source in the graveyard); now passes with save/load and a later-seat draw.
- `cargo test -p aperture-core --test issue17_slice17` -> **5 passed**
  (actual turn boundary: a second Caller next turn creates no tokens).

## Regression coverage

- `crates/core/tests/card_printings.rs` verifies every new card against
  `fixtures/starter-corpus/slice17-sourced.json` (name, cost, type, P/T, oracle, legal).
- `catalog_pool` verifies the new effects are legal/tested effect shapes.
- Full core suite, focused clippy (`-D warnings`), and legacy replay tests were run; results
  are in `IMPLEMENTATION-HANDOFF.md`.
