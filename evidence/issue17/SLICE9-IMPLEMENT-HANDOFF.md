# Issue 17 slice 9 — widened mana and count-scaled red rituals

Exact head `4e8a856`. No push. Full #17 unfinished:
114 catalog, 459 union = 102 supported + 357 missing, 1 implemented / 0
certified of 8. Eight desired deck files unchanged.

## Cards and sources

Live Scryfall named printings fetched before implementation and recorded in
`fixtures/starter-corpus/slice9-sourced.json` with printing URI, Scryfall id and
actual UTC (`2026-10-26T13:18:56Z`):

* **Brightstone Ritual** (`{R}`, ons/191): “Add {R} for each Goblin on the battlefield.”
* **Mana Geyser** (`{3}{R}{R}`, soc/247): “Add {R} for each tapped land your opponents control.”
* **Rite of Flame** (`{R}`, csp/96): “Add {R}{R}, then add {R} for each card named Rite of Flame in each graveyard.”

Brightstone supersedes its slice8 excluded source status; the slice8 fixture and
evidence are unchanged.

## Genuine behavior-first RED

Before any mana/engine change, the tests failed on u8 saturation and absent
cards:

* `mana_pool_adds_and_pays_above_255_without_truncation` — 255+45 saturates.
* Brightstone/Mana Geyser/Rite tests failed at `catalog::by_name`.
* After scaffolding cards with no-op effects, the three spell tests failed at
  `left: 0, right: 300/4/4`.

Then implementation: `ManaPool` fields/`get`/`add`/`total` widened to `u32`;
`Event::ManaAdded.amount/pool_total` and `ManaPoolEmptied.amount` widened to
`u32`; `ManaCost`/pips stay `u8`, and `take` converts only the bounded pip
amount. Pool addition is saturating at `u32::MAX` (explicit upper bound, no
silent 255 truncation). A real Lightning Bolt cast from a 300-red pool pays one
pip and leaves exactly 299.

Counts resolve at resolution:

* Brightstone counts all battlefield Goblins (both players).
* Mana Geyser counts only tapped lands controlled by opponents (not own,
  untapped, eliminated).
* Rite of Flame adds 2 plus 1 per Rite of Flame card in all graveyards; the
  resolving spell is on the stack, not yet in a graveyard.

## Repair pass (bounded render + stronger tests)

* `ManaPool.render` no longer emits one symbol per unit unbounded. Per colour it
  keeps the exact old repeated format for quantities ≤32 and emits an aggregate
  `{R}x4294967295` above that, so max-quantity deserialize/render is bounded.
* Rite of Flame test corrected: cards are placed in their owners' physical
  graveyards and zones; zero-copy base-2 control added; a third copy moves into a
  graveyard after casting but before resolution, proving runtime counting
  (`2 + 3 = 5`).
* Added explicit `u32::MAX` saturation/no-panic test and multicolor 600-pool
  payment: Divination pays `{U}` + two generic from blue, leaving exactly
  blue 297 / red 300 / total 597.
* Genuine RED before implementation is retained in
  `evidence/issue17/slice9/retained-red-results.json` (actual absent-catalog and
  pre-engine pool saturation failures); no nonexistent named RED log is claimed.

## Verification

```
cargo test -q -p aperture-core --test issue17_slice9
  -> 6 passed
cargo fmt --all --check
cargo clippy -q --all-targets -- -D warnings
cargo build --release -p aperture-cli
  -> exit 0 (evidence/issue17/slice9/gates.log)
```

Compatibility: pre-existing slice8 replay still verifies unchanged
(`ffbeaee0db38…`, 6087 events). Serialized mana fields keep numeric JSON values;
a 300-red state round-trips.

## Journey

`decks/package-slice9.txt` — 40-card supported ritual package, not a desired
starter; four seats, Krenko commander each, 40 life, seed 901:

* terminal, P0 wins, 1339 steps / 32 turns, digest `5d20bb4a35e2…`
* separate replay matches 4379 events; save replay matches
* honest limitation: the simple heuristic never cast Brightstone and drew but
  did not cast Mana Geyser/Rite of Flame in this trace; focused tests prove the
  three behaviors

## Corpus audit

New audit only: `evidence/issue17/slice9/corpus-audit.py` →
`corpus-state.json`; historical audit scripts untouched. Current: catalog 114,
union 459, supported 102, missing 357, manifest matches. Per-list: Isamaru 55,
Jasmine 0, Talrand 51, Krenko 48, Ayula 46, Elas 55, Tatyova 54, Zada 53.

## Remaining gaps

No desired 100-card starter is complete or certified. Friendly Teddy and
Aetherize remain excluded. Broader legend/equipment/tutor/reanimation and Ayula
tribal/counter text remain open.
