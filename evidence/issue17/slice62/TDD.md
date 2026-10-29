# Slice 62 TDD tracer bullets

The card metadata and the default-empty `attacked_this_turn` field were added first
so the new tests compile; the behavior was absent. `cargo test -q -p aperture-core
--test issue17_slice62` then produced these compiling assertion failures:

- `goblin_chieftain_anthem_and_haste_are_other_owned_goblins_only`: own Goblin
  power was `1`, expected `2` ("other own Goblin gets +1/+1").
- `searslicer_casts_and_raid_end_step_makes_a_real_token_after_attackers_leave`:
  real Goblin-token count was `0`, expected `1`.
- `raid_history_resets_on_turn_transition_and_roundtrips_a_save`: the fabricated
  history was not represented by the embedded non-sandbox replay and was not
  rejected as an untrusted save (`Invariant ... embedded replay produced digest`).
- `raid_does_not_trigger_without_an_attack_this_turn` passed before production
  behavior (a useful negative control, not RED).

After the static, keyword, Raid queue/resolution/reset and validation work, all
five tests pass. Additional GREEN coverage then added the actual sick-attack
declaration test. No compile failure was counted as RED.
