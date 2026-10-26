# Issue 17 slice 8 — cross-list missing-card package handoff

Eight real missing cards from the exact desired lists are now supported, using
existing effects plus six bounded reusable primitives. No push. Full #17
unfinished; eight desired, one implemented, zero certified.

## Cards and sources

Live Scryfall `cards/named?exact=` printings fetched before implementation;
records in `fixtures/starter-corpus/slice8-sourced.json` with URI and actual
`retrieved_at` (`2026-10-26T12:50:12Z` for the first eight, then the War Strike
fetch time):

| card | source | behavior |
| --- | --- | --- |
| Soul's Attendant | roe/44 | optional another-creature-enters gain 1 |
| Razorclaw Bear | p02/142 | becomes blocked → +2/+2 until end of turn |
| Volley Veteran | fdn/550 | ETB damage to target opponent creature = Goblins you control |
| Brightstone Ritual | ons/191 | add {R} for each Goblin on the battlefield |
| Aetherize | fdn/151 | return all attacking creatures to owner's hand |
| Dramatic Reversal | tle/158 | untap all nonland permanents you control |
| Friendly Teddy | dsk/247 | dies → each player draws a card |
| Goblin War Strike | live fetch | target player damage = Goblins you control |

Excluded: Refute (counter+draw+discard needed an effect-discard resumption path
not completed), Snap/Rewind/Sol Ring/Skirk Prospector and other choices with
additional costs or mana-ability timing. They were not added or stubbed.

## Genuine RED

1. Before cards existed: all seven initial slice8 tests failed at
   `catalog::by_name` (`Soul's Attendant must be supported`, etc.).
2. After scaffolding card/catalog data but before effects/abilities:
   * Soul's Attendant optional lifegain — no life change.
   * Razorclaw Bear — never pumped.
   * Volley Veteran — no ETB damage.
   * Brightstone Ritual — no red mana.
   * Aetherize/Dramatic Reversal — no return/untap.
   * Friendly Teddy — no each-player draw.
   * Goblin War Strike — target player took no Goblin-count damage.
3. Test-harness corrections were kept separate from production claims
   (Friendly Teddy P0 hand accounting, War Strike Goblin count, Aetherize blocking
   priority); each was re-run to GREEN.

New primitives: `OpponentCreature` target, `BecomesBlocked` trigger,
`DamageTarget{Creature,Player}ForEachGoblin`, `AddManaForEachGoblin`,
`ReturnAllAttackersToHand`, `UntapAllNonlandsYouControl`, `EachPlayerDraws`,
plus `Kind::Artifact`.

## Verification

```
cargo test -q -p aperture-core --test issue17_slice8 --test issue17_tokens \
  --test issue17_slice6 --test counters_tokens_library --test catalog_pool \
  --test card_printings --test issue13_commander_combat
  -> 8, 10, 13, 16, 5, 5, 43 passed
reviewer224 probe -> 3 passed
cargo fmt --all --check
cargo clippy -q --all-targets -- -D warnings
cargo build --release -p aperture-cli
  -> exit 0 (slice8-gates.log)
```

## Journey

`decks/package-slice8.txt` — 40-card supported package fixture, **not** a
desired 100-card starter. Four seats, Isamaru commander each seat,
Commander-combat 40 life, seed 801:

* terminal, P1 wins, 1600 steps / 40 turns, digest `6fea3673e366…`
* verbose log contains all eight new cards: Soul's Attendant 19, Razorclaw Bear
  15, Volley Veteran 12, Brightstone Ritual 1, Aetherize 2, Dramatic Reversal 1,
  Friendly Teddy 61, Goblin War Strike 1 occurrences
* separate replay matches digest, 4087 events; `--save-out` save replays to the
  same digest (`evidence/issue17/slice8/`)

## Corpus audit

Recomputed with the actual 114-card pool and eight unchanged exact lists:
catalog **114**, union 459, supported **102**, missing **357**, manifest
`missing_cards` matches. Per-list: Isamaru 55, Jasmine 0, Talrand 50, Krenko 48,
Ayula 45, Elas 55, Tatyova 54, Zada 55.

## Gaps

* No 100-card desired starter is complete; all seven non-Jasmine lists remain
  planned and uncertified.
* Refute/Snap/Rewind/Sol Ring/Skirk Prospector and other short-list candidates
  were deliberately not added because complete semantics were not implemented.
* Friendly Teddy's each-player draw uses the existing single-source draw resume;
  commander-draw replacement ordering across multiple simultaneous drawers is
  not separately modelled.
* Existing analyzer gameplay refusal is unchanged; no certification claim.
