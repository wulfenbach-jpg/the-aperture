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


## Native-reproduced activator-priority RED

At pre-repair head `8e967a6`, the native workspace gate failed
`aperture-core --test command_zone_setup`:

```
ayula_ignores_opposing_bears_and_token_entry_chooses_any_bear_for_counters ... FAILED
thread ... crates\core\tests\command_zone_setup.rs:2115:10:
pass: NotYourDecision { expected: 1, got: 0 }
test result: FAILED. 43 passed; 1 failed
```

Cause: `apply_activate_ability` left priority None and canonical settle granted
it to the active player. For a non-active activator (P1 active, P0 activates)
this was wrong. The fix records `activation_priority` and consumes it inside
`grant_priority` only after SBA/staging. This regression suite now includes a
non-active activator test. Assertions added after the original GREEN (save/load
continuation, missing-target legal-set, exact two death triggers, post-cost save,
enchantment control, pre-existing-token/expiry/commander-zone Siege cases) are
post-GREEN coverage, not claimed as pretest RED.

## Post-GREEN coverage additions
- R367-1 loaded/original full drainage digest equality.
- R367-2 whole-offer doomed-target absence, exact two Pashalik triggers,
  post-cost sandbox save, enchantment control.
- Siege: pre-existing token excluded from "those", next-turn haste expiry,
  real Unsummon/commander-replacement before resolution fizzles the trigger.
