# Slice 68 TDD / coverage record

No pre-implementation compiling RED was retained for the two new mechanics;
the initial Skirk/Zo-Zu GREEN is not claimed as TDD.

Native coauthor RED before repair: `prospector_is_not_a_no_abilities_creature`
compiled and failed with "Skirk has its printed activated mana ability, including
under Ruxa". The shared predicate now includes `CardId::sacrifice_mana()` in its
intrinsic-mana check; the failure is retained in
`evidence/issue17/slice68-native/probe-corrected.log` and the committed suite now
asserts predicate, Ruxa power, and graveyard exclusion.

Post-GREEN additions (not claimed as RED): non-active Skirk save/load continuation
with identical events/state/digest/revision and final active-player priority;
Zo-Zu bounced-source damage through Angrath's Marauder; own-land and
opponent-land controls; self-sacrifice while sick+tapped; malformed affected-player
save refusal.
