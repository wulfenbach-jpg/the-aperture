# Slice 62 TDD receipts

## Initial compiling RED (before production behavior)

Scaffolding already present at this point: the two `CardId`s with catalog metadata
(no ability/static arrays yet) and the default-empty `attacked_this_turn` field, so
the tests compiled. `cargo test -q -p aperture-core --test issue17_slice62` on the
four initial tests produced exactly:

- `goblin_chieftain_anthem_and_haste_are_other_owned_goblins_only` FAILED:
  `left: 1, right: 2` at "other own Goblin gets +1/+1".
- `searslicer_casts_and_raid_end_step_makes_a_real_token_after_attackers_leave`
  FAILED: token count `left: 0, right: 1`.
- `raid_history_resets_on_turn_transition_and_roundtrips_a_save` FAILED in
  `SessionSave::from_json` at `attack history save is valid`: `Invariant ...
  embedded replay produced digest ... but the saved state is ...`.
  That call site expected acceptance of a fabricated **non-sandbox** history.
  The refusal was already correct; the test expectation was fixed to require the
  refusal and to use a sandbox capture for the positive persisted-history roundtrip.
- `raid_does_not_trigger_without_an_attack_this_turn` PASSED initially (negative
  control).

The fifth test, `actual_attack_declaration_records_the_turn_fact_after_the_attacker_leaves`,
was added after GREEN; it is not claimed as an initial RED.

## Native prepublication repair RED (new predicate hole)

Before changing `object_has_no_abilities`, the new test
`chieftain_haste_is_an_ability_for_ruxas_no_abilities_predicate` compiled and failed:

```
thread ... panicked at crates/core/tests/issue17_slice62.rs:318:5:
Chieftain's static grants the vanilla Goblin token haste
```

After adding the shared effective subtype/creature static check, the test is GREEN
and also verifies the Chieftain-departed, non-Goblin, opposing-Goblin and
graveyard-zone controls. The no-attack boundary test was then corrected to start in
PostcombatMain and actually enter EndStep; the real-declaration save/load
continuation test was added.
