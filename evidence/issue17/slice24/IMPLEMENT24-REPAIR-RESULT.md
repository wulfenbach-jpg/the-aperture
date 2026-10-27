# Slice24 repair result

- Base committed implementation: `c457e49`.
- Native behavioral RED preserved: `evidence/issue17/slice24/native-behavior-red.log`
  (two real assertion failures: hasty Werebear absent from offered mana sources; mandatory
  non-target trigger deleted when the entered object left).
- Repair commit is the final local head recorded in the final receipt; no push/merge/closure.

## Repairs
- `mana_sources` now consults `has_haste` for creature mana sources, so a hasty
  summoning-sick Werebear is actually in the offered legal activation list.
- Battlefield departure invalidates the captured `affected` binding to `None` across
  trigger queue, stack, and applicable pending trigger references. The mandatory ability
  is retained and later resolves without touching a recast incarnation.
- `add_haste_grant` uses captured `StackItem.card`, not a dereference of a possibly
  rekeyed/departed source object.
- Narrative event wording changed to “gets a +1/+1 counter”, not “enters with”.
- State validation/archived-reference traversal covers stack/queued affected references;
  legitimately invalidated `None` remains valid.
- Added real source-left Surrak (Unsummon response) and token-filtering controls.

## Real command exits
`evidence/issue17/slice24/repair-gates.log`: slice24 10 passed exit 0; abilities 13,
timing_mana 12, card_printings 8, catalog_pool 5, slice15 2, commander 43, replay_digest 6
exit 0; fmt 0; workspace clippy -D warnings 0; release 0.

Journey rerun: terminal P3, 1602 decisions, 37 turns, digest
`a41bbfd2760034f791a3616bd5d6faa0fd79ad8e9cee300eb69cc38c5a1ec631`, replay matched,
fresh save round-tripped; Surrak cast 6, 11 entry-buff events, 11 haste grants, 20 blocked
combat-damage events.

## Gaps
- Midtrigger save is a sandbox/direct fixture, not a reachable action-history save claim.
- Tap-cost haste activation beyond Werebear mana is not separately tested.
- No full workspace matrix rerun; exact-head CI owns it. Full #17 remains 1 of 8 certified.
