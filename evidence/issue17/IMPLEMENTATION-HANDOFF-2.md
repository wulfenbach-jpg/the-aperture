# Issue 17 — assignment 2 handoff (planned corpus plan)

**PARTIAL #17.** Assignment 1's Jasmine package is unchanged and still the only
supported/playable starter. Assignment 2 adds exact **desired** lists for the
other seven targets as metadata only. Nothing here is supported, playable or
certified; no new mechanics were implemented.

## Delivered

* Seven exact desired lists under `decks/starter-corpus/planned/`:
  `isamaru-white-aggro`, `talrand-draw-go`, `krenko-goblin-tokens`,
  `ayula-bear-counters`, `elas-sacrifice`, `tatyova-landfall`, `zada-spells`.
  Each is **99 library cards + separately named commander**, 57 unique nonland
  names + 42 basics (two-color lists split 21/21).
* `decks/starter-corpus/manifest.json` (version 1) now points every entry at its
  exact list. Statuses: Jasmine `implemented`; other seven `planned`; none
  `certified`. `missing_cards` is computed from current catalog membership.
* Real-card source capture: `evidence/issue17/source-planned-starters.py`;
  per-archetype search/theme URLs and per-card printing id/set/collector/URI in
  `fixtures/starter-corpus/planned/source-index.json`; retrieval
  `2026-10-25T08:32:00Z`. Search used
  `order=edhrec` plus archetype theme clauses (goblins, bears, instants/sorceries,
  sacrifice/dies, landfall/ramp, targeted spells, humans/equipment). No metadata
  was invented. Nonnormal/multiface/nonlegal/off-identity/pregame cards were
  filtered; all lists are format-legal offline.
* Per-list analyzer snapshots under `fixtures/starter-corpus/planned/`
  (`<id>.deck.json`, `<id>.cards.json`) using the existing
  `aperture.commander-deck.v1` / `aperture.card-metadata.v1` schemas.

## Certification evidence

`target/release/aperture-cli.exe commander-check` on each planned snapshot:

| id | format | engine support | unsupported cards | total |
| --- | --- | --- | --- | --- |
| isamaru-white-aggro | legal | unsupported | 57 | 100 |
| talrand-draw-go | legal | unsupported | 55 | 100 |
| krenko-goblin-tokens | legal | unsupported | 58 | 100 |
| ayula-bear-counters | legal | unsupported | 52 | 100 |
| elas-sacrifice | legal | unsupported | 59 | 100 |
| tatyova-landfall | legal | unsupported | 58 | 100 |
| zada-spells | legal | unsupported | 58 | 100 |

Reports: `evidence/issue17/planned-checks/<id>.json`; summary:
`evidence/issue17/planned-checks/summary.json`. The analyzer's blanket
`gameplay.available = false` is untouched. Format `legal` never means supported.

Corpus union (programmatically computed from the exact files and catalog):
**447 unique nonbasic names; 61 supported, 386 missing.** Per-list supported
overlap: Jasmine 58/58, Ayula 6/57, Talrand 3/57, others 0/57. Elas additionally
needs the unsupported **Swamp** basic.

## Structural regression

New `crates/import/tests/starter_corpus_planned.rs` verifies exactly eight ids,
every desired list as 100 cards including commander, singleton nonbasics with
basic exceptions, 56–61 unique nonland names, 38–43 basics, manifest
`missing_cards` exactly equal to names absent from the current catalog, no
`certified` status, and for each planned list: analyzer format `legal`, engine
`unsupported`, non-empty unsupported list, gameplay `false`.

RED before the planned files/manifest paths existed:

```
cargo test -q -p aperture-import --test starter_corpus_planned
  panicked: isamaru-white-aggro has no exact desired list
```

GREEN after generation:

```
cargo test -q -p aperture-import --test starter_corpus_planned
  -> 1 passed
cargo test -q -p aperture-core --test starter_corpus
  -> 8 passed
cargo test -q -p aperture-import --test starter_corpus_analyzer
  -> 3 passed
cargo clippy -q --all-targets -- -D warnings
  -> clean
```

The existing Jasmine package, deck, snapshots, replay and journey artifacts are
unchanged.

## Remaining gaps

* All seven planned lists are unsupported metadata; the eight-list gate is open.
* Token, trigger, activated-ability, keyword, counter, copy, tribal, landfall,
  death/drain, legend-rule/equipment/Aura and black-basic-land support remain
  unimplemented. `missing_interactions` records the specific gaps per list.
* Desired lists are curated from real Scryfall popularity/theme results, not
  optimized tournament decks; mana curves are plausible but not simulated.
* No merge, push, PR, issue closure or certification is claimed.

## Files

Changed: `decks/starter-corpus/manifest.json`,
`docs/issue17-starter-corpus.md`, `crates/core/tests/starter_corpus.rs`
(planned-path assertion), `crates/import/tests/starter_corpus_planned.rs`.
Added: `decks/starter-corpus/planned/*.txt`,
`fixtures/starter-corpus/planned/*`, `evidence/issue17/planned-checks/*`,
`evidence/issue17/source-planned-starters.py`, this handoff.

Reporter pointer still targets old #20; untouched. No commit/push.
