# Slice 21 review — Dragon-Scarred Bear

**CHANGES_REQUESTED** at `d9d2c945673a256a62639b7a789adca549ea7984` (code `412f63a`; base `cdfc32e`).

## Blocking finding

**Regeneration shields transfer through a bounce/recast of the same non-commander card.**
The shield is keyed only by `ObjectId` (`engine.rs` `try_regenerate` / `regeneration_shields`),
but `insert_object` retains the old id for every non-commander object, and
`move_to_hand` → `move_object_to_stack` → `enter_battlefield` reuse that id. The
shield is cleared only at cleanup, not on the zone change. Therefore:

1. Activate Dragon-Scarred Bear's shield.
2. Unsummon it to hand.
3. Recast the same card.
4. Shock it.

The recast permanent still finds the old shield and regenerates. A temporary reviewer
probe reproduced this against exact head: after recast + Shock the bear was
`Battlefield`, not `Graveyard`, with the assertion message
`old shield must not transfer through bounce/recast`.

This violates the required “source left/no transfer” behavior. The existing test
`exile_and_bounce_are_not_destruction_and_do_not_transfer_shields` misses it because it
inserts a **new** object (`add(...)`) instead of recasting the bounced card.

Required correction: drop shields when the object leaves the battlefield (or anchor them
to a battlefield incarnation/new id), and add a regression that bounces and **recasts
the same card** before testing destruction. No implementation edit was made here.

## Checks actually run

- `cargo test -p aperture-core --test issue17_slice21 --test issue13_commander_combat --test replay_digest --locked`:
  slice21 **8/8**, issue13 **43/43**, replay_digest **6/6**.
- `cargo test -p aperture-core cleanup_expires_regeneration_shields --locked`: **1/1**.
- Temporary bounce/recast probe: **RED** as above; probe source removed, log retained at
  `evidence/issue17/slice21-review-probe.log`.
- `cargo fmt --all -- --check`: exit 0; `git diff --check`: exit 0.
- Release replay of `evidence/issue17/slice21/demo-replay.json`: 1906 actions,
  4909 events, digest `0d20c7597183` matches.
- Legacy release replay `slice20/demo-replay.json`: 2064 actions, 5281 events,
  digest `72cd69fb1143` matches.
- Source/fixture inspection: DTK 183, `{2}{G}`, 3/2 Bear, oracle text matches the
  fixture/raw retrieval. Formidable uses current power, checks at activation only,
  and the journey shows 25 activations / 25 shield-creation events / 1 regeneration.
- AI diff is mechanical shield-field initialization in `search.rs:727` and the test
  fixture only; no policy change.

## Nonblocking notes

- The mass-destroy path calls `try_regenerate`, but no current card destroys a
  Dragon-Scarred Bear (the existing destroy-all effect targets artifacts/enchantments);
  no new test exercises that branch.
- The slice21 test set does not cover a destroy-all creature case; existing lethal,
  multiple-shield, cleanup, save/reload and blocked-attacker tests pass.
- Full original #17 remains **1/8 certified**, **333 missing** / Ayula 27; no merge,
  closure, or downstream unlock. Exact-head CI remains parent-owned.
