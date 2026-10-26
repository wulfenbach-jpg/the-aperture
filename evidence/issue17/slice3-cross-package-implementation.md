# Issue 17 slice 3 — cross-corpus package handoff

Implemented five sourced cards that were missing names in the exact desired
lists, plus **Guttersnipe** with a complete controller-only spell-cast trigger.
No commit/push. Full #17 incomplete; desired lists unchanged.

## Cards and sources

From stored real Scryfall planned-list printing responses, retrieved
`2026-10-25T08:32:00Z` (`fixtures/starter-corpus/cross-package-sourced.json`):

| card | printing | text implemented |
| --- | --- | --- |
| Swamp | mdd/296 | Basic Land — Swamp, `{T}: Add {B}` |
| Lightning Bolt | msc/806 | `{R}` instant, 3 damage to any target |
| Forest Bear | ptk/135 | `{1}{G}` 2/2 vanilla Bear |
| Striped Bears | wth/140 | `{3}{G}` 2/2 Bear; ETB draw a card |
| Guttersnipe | hoc/196 | `{2}{R}` 2/2 Goblin Shaman; whenever you cast an instant/sorcery, deals 2 damage to each opponent |

Pool 88 → **93**. Swamp closes the black-basic-land gap; Elas still has 58
missing names.

## Genuine RED / GREEN

1. Cards absent: all five tests failed at catalog lookup (source + behavior).
2. After compile/scaffold entries:
   * `forest_bear_is_vanilla_and_striped_bears_draws_on_entry` — hand count
     `0` vs expected `1` (no ETB draw).
   * `guttersnipe_hits_each_opponent_but_not_its_controller` — P2 life `17`
     vs expected `15` (no trigger damage).
   * source-left Guttersnipe test also failed until the trigger/effect existed.
3. Fix: Striped Bears typed ETB Draw; Guttersnipe typed SpellCast trigger with
   `Effect::DamageEachOpponent(2)` (live opponents only, controller excluded,
   damage source is the ability source, source-left resolution preserved by the
   existing stack/trigger scheduler). No priority is granted mid-effect.
4. GREEN: 5/5 cross-package tests, including 4-seat each-opponent damage and
   source-left-by-Unsummon damage.

## Verification

```
cargo test -q -p aperture-core --test issue17_cross_package --test issue17_tatyova \
  --test issue17_tokens --test issue17_talrand --test card_printings \
  --test catalog_pool --test issue13_commander_combat \
  -p aperture-import --test starter_corpus_planned
  -> 5, 5, 10, 6, 4, 5, 43, 1 passed
cargo test -q -p aperture-cli
  -> 8 + 1 passed
cargo fmt --all --check
cargo clippy -q --all-targets -- -D warnings
cargo build --release -p aperture-cli
  -> exit 0
```

## Four-seat package journey

`decks/package-cross-corpus.txt` is a clearly named **40-card supported package
fixture**, not a desired starter. Four seats, ordinary practice, seed 5:

* terminal, P2 wins, 1112 steps / 25 turns, digest `4572cd11e893…`
* **8 Guttersnipe spell-cast triggers** in the log; actual damage to each live
  opponent; no controller damage
* replay digest matches; `--save-out` save replays to the same digest
* the earlier attempt with a removal-heavy build produced no Guttersnipe triggers
  (preserved in git-less evidence history by rerun); the final package uses
  Opt/Consider/Divination so the AI actually casts spells

Existing Tatyova replay still reproduces `b31adfd97c31…`.

## Corpus accounting

Coordinator audit script updated for `card_defender!` and rerun:

* catalog **93**; union **459**; supported **81**; missing **378**;
  manifest missing **378** and matches the exact names.
* Per-list missing: Isamaru 57, Jasmine 0, Talrand 54, Krenko 51, Ayula 50,
  Elas 58, Tatyova 56, Zada 57.
* All eight exact desired list files unchanged.

## Remaining gaps

* No desired 100-card starter is complete; all seven non-Jasmine lists remain
  planned/unsupported.
* Guttersnipe support is the printed trigger + damage primitive only; no
  additional Guttersnipe-like cards were added.
* Ayula vanilla/ETB additions beyond Forest Bear/Striped Bears were not added
  within the time budget.
* Legend rule, broader landfall/ramp, tokens beyond Goblin/Drake, counters and
  tribal payoffs remain unimplemented.

Changed: `crates/core/src/{card,catalog,ability,engine}.rs`,
`crates/core/tests/{issue17_cross_package,catalog_pool,card_printings,issue13_commander_combat}.rs`,
`decks/package-cross-corpus.txt`,
`fixtures/starter-corpus/cross-package-sourced.json`,
`decks/starter-corpus/manifest.json`, `docs/issue17-starter-corpus.md`,
`README.md`, `docs/issue15-effects.md`, and evidence `slice3-cross-*` logs.
