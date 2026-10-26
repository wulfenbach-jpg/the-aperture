# Issue 17 slice 8 — five supported cards + three explicit exclusions

Repair after native inspection. Five sourced missing cards are implemented; three
formerly claimed cards are **explicitly excluded** because the printed text is
not fully representable in this bounded engine. No push. Full #17 unfinished.

## Implemented (5)

From exact desired lists, live Scryfall printings recorded in
`fixtures/starter-corpus/slice8-sourced.json`:

| card | source | behavior |
| --- | --- | --- |
| Soul's Attendant | roe/44 | optional another-creature-enters gain 1 |
| Razorclaw Bear | p02/142 | becomes blocked once → +2/+2 until end of turn |
| Volley Veteran | fdn/550 | ETB damage to target opponent creature = Goblins you control |
| Dramatic Reversal | tle/158 | untap all nonland permanents you control |
| Goblin War Strike | live fetch | target player damage = Goblins you control |

New bounded primitives: `OpponentCreature` target, `BecomesBlocked` trigger,
`DamageTarget{Creature,Player}ForEachGoblin`, `UntapAllNonlandsYouControl`.

## Excluded (3, records retained with role/status `excluded`)

* **Brightstone Ritual** — actual Goblin count can exceed 255 and `ManaPool` is a
  saturating `u8`; silently clamps the printed text. Excluded until a bounded
  mana representation exists. Removed from CardId/catalog/effect/tests/package.
* **Friendly Teddy** — `EachPlayerDraws` returned on the first commander-draw
  replacement, losing later players’ draws, and iterated P0.. instead of
  active-player order. Removed rather than approximated; `Kind::Artifact` added
  only for it was also removed. Neither test is claimed as fixed.
* **Aetherize** — `ReturnAllAttackersToHand` called `move_to_hand` directly and
  bypassed the existing CR 903.9b commander hand-replacement choice. Implementing
  a serialized remaining-attacker continuation was not completed; excluded
  rather than shipping a known bypass. Source record retained as excluded.

Both remain in the manifest missing set via fresh recomputation; desired lists
untouched.

## Genuine RED / controls

Original RED: all slice8 cards absent from catalog; after scaffold, each new
behavior failed before implementation (optional lifegain, blocked pump, Volley
damage, Reversal untap, War Strike count).

Added controls:
* OpponentCreature target: with two opponent creatures a `TriggerTarget` pending
  is offered; forged self/own-Goblin target is refused with unchanged state
  digest; the legal opponent target succeeds.
* Volley source-left/runtime count: bounce Volley with Unsummon before the
  trigger resolves; X counts the two remaining Goblins and deals lethal 2.
* Razorclaw with two blockers triggers exactly once (power 5, not 7); existing
  one-blocker pump/revert test retained.
All drains assert `pending`, `resolving`, `stack` and `trigger_queue` empty.

## Verification

```
cargo test -q -p aperture-core --test issue17_slice8 --test issue17_tokens \
  --test issue17_slice6 --test counters_tokens_library --test catalog_pool \
  --test card_printings --test issue13_commander_combat
  -> 8, 10, 13, 16, 5, 6, 43 passed
cargo fmt --all --check
cargo clippy -q --all-targets -- -D warnings
cargo build --release -p aperture-cli
  -> exit 0
```

## Journey

`decks/package-slice8.txt` was updated to remove the three excluded cards
(Counterspell, Shock and Divination replace them). 40-card supported package, **not** a desired
starter; four seats, Isamaru commander each, 40 life, seed 801:

* terminal, P0 wins, 2377 steps / 68 turns, digest `ffbeaee0db38…`
* all five implemented cards appear in the verbose log
* separate replay matches, 6087 events; `--save-out` save replays to the same
  digest (`evidence/issue17/slice8/`)

## Corpus audit

New audit only: `evidence/issue17/slice8/corpus-audit.py` →
`evidence/issue17/slice8/corpus-state.json`. Historical
`slice3-native-corpus.{py,json}` restored exactly from `7e1eb5f` and never
rerun for this slice.

Current: catalog **111**, union 459, supported **99**, missing **360**;
manifest `missing_cards` matches. Per-list: Isamaru 55, Jasmine 0, Talrand 51,
Krenko 49, Ayula 46, Elas 55, Tatyova 54, Zada 55.

## Gaps

* No 100-card desired starter is complete; seven remain planned/uncertified.
* Excluded Brightstone Ritual/Friendly Teddy/Aetherize need bounded mana
  widening, serialized multi-player draw continuation and attacker-list
  commander-replacement continuation respectively.
* Ayula’s printed counter/tribal text remains unimplemented even though typed
  counters exist from #16.
