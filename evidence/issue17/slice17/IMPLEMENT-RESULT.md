# Slice 17 repair result

**Status: PARTIAL #17.** Implementation head `7e4a013` on `aperture/issue-17`
(based on `6f294cf`). No push, merge, PR, reviewer launch, or issue closure.

## Repairs
- Death history resets only when `turn.number` advances. Repeated cleanup
  (including cleanup priority) retains earlier same-turn deaths. Regression:
  `lib::slice17_cleanup_history_tests::repeated_cleanup_keeps_earlier_deaths_until_the_turn_number_advances`
  calls real `finish_cleanup` twice. The integration test also proves the next
  actual turn starts empty.
- `DrawResume::EachPlayer` stores source/cursor, next seat, cards left, per-seat
  count, completed live seats, and the fixed live total. A commander replacement
  consumes only that seat's draw; later live seats still draw exactly once.
  Regression: `command_zone_setup::each_player_draw_continues_every_live_seat_after_a_commander_replacement`
  includes a mid-suspension `SessionSave` validation/reload.
- Save validation now permits a triggered ability's source card to remain in
  its zone (e.g. graveyard) while its stack item exists.
- +1/+1 counters emit the existing authoritative counter state event with the
  resulting total.
- AI delta is only initialization/copy of the two new public state fields in
  `crates/ai/src/search.rs` and `crates/ai/tests/common.rs`.
- Closed-corpus death-tracking opt-in is documented: every playable card starts
  as an engine object; no outside-card source exists in this slice.

## Commands actually run
| command | result |
| --- | --- |
| `cargo test -q -p aperture-core --lib --test issue17_slice17 --test command_zone_setup` | 12 lib, 34 setup, 5 slice17 passed |
| `cargo test -q -p aperture-core --test replay_digest --test ordering_persistence` | 6 + 2 passed |
| `cargo fmt --all` | applied |
| `cargo clippy -q --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo build -q --release -p aperture-cli` | exit 0 |
| slice17 journey driver | terminal win seat 3; digest `85fabffc9aa16a0f9ba53a5f64a751c43bb9ae66a3ce96f3e7402f1c633c7027`; replay digest matched; 4 Bears; fresh `demo-save.json` round-trip equal |

## Unfinished / open
- Ayula desired list: 31 of 34 nonland cards unsupported.
- Eight explicitly supported 100-card lists: gate open.
- No full workspace rerun after repair, per repair instruction; focused suites
  + workspace clippy + release CLI only.
- No four-seat 100-card Commander snapshot.
- Reporter pointer still targets old #20; ops files untouched.
