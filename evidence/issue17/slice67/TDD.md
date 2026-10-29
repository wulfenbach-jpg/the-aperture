# Slice 67 TDD receipts

## R367-1/R367-2 retained RED

Before the activation scheduler change, the owned checkpoint suite compiled and
`cargo test -q -p aperture-core --test issue17_slice67_checkpoint` produced:

- `a_real_offered_sacrifice_with_pending_target_produces_valid_saveable_state`:
  the offered Pashalik sacrifice ended with
  `pending=TriggerTarget`, `priority=Some(0)`, validation
  `a pending choice cannot coexist with a priority holder (CR 608.2d)`, and
  `SessionSave::from_json` refused with the same invariant.
- `sacrifice_anthem_runs_sbas_before_choosing_cost_trigger_targets`: the doomed
  1/1 token with one damage remained on the battlefield and was still offered as
  a Pashalik target instead of dying before trigger staging.

After removing the manual `put_triggers`/priority overwrite and letting the
canonical settle checkpoint recompute/SBA/stage, both pass. The tests now also
assert no false priority, no doomed-target offer, exactly two Pashalik death
triggers after drainage, and identical original/loaded continuation digests.

## Siege-Gang Lieutenant

The card metadata was scaffolded before the targeted behavior suite was added,
but no pretest RED was retained for the Lieutenant effect or activated ability;
that GREEN is not claimed as TDD. The final tests cover real cast/resolve,
commander-present/absent/opponent, token stats/haste, source departure and the
real `{2}, Sacrifice a Goblin` activation. This limitation is recorded rather
than reconstructed.
