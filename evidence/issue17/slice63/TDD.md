# Slice 63 compiling RED receipts

Card IDs/catalog metadata were scaffolded first (no ability/static arrays) so the
new tests compiled. The initial `cargo test -q -p aperture-core --test
issue17_slice63` produced five assertion failures, all compiled successfully:

- `warchief_reduces_only_goblin_generic_costs_and_stacks`: `CannotPay { cost:
  "{2}{R}{R}", available: "{R}{R}" }` — no subtype reduction was implemented.
- `warchief_haste_includes_self_and_other_controlled_goblins_only`: failed
  "Warchief's text includes itself" — no haste static.
- `king_anthem_and_mountainwalk_cross_controllers_and_exclude_source`: own other
  Goblin power `left: 1, right: 2` — no all-controller anthem.
- `mountainwalk_reads_the_defending_players_effective_mountain_and_self_is_excluded`:
  failed "defending Mountain makes another Goblin unblockable".
- `all_three_cards_cast_from_hand_through_the_stack`: initially failed on
  Goblin King payment because that test setup had only five Mountains. This was
  a test-setup failure, not a production RED; it was corrected to seven lands
  while behavior REDs were retained.

The King expectation was later corrected from 2 to 3 because two Kings each add
+1/+1 to the other; that is a test-expectation correction, not a behavior change.
After statics, landwalk, cost reduction and the shared no-abilities update, the
suite is GREEN. Later added tests cover Warchief-granted haste/mountainwalk in the
no-abilities predicate, departure, Chieftain-style recipient controls, colored-pip
refusal and the Dragon-typed static. Final targeted test count: 10 (the
five-compiled-failure RED phase produced 8 tests; two controls — the real-cast
positively-reduced Dragon payment and the Krenko command-tax case — were added
after the prepublication cooperative-payment review, so the shipped binary runs
10, not 8).

No compile error is counted as RED.
