# Issue 17 — TDD record (first starter package)

Base: `8a72cc607ea25f203be672c0007ad5e798be7145` (branch `aperture/issue-17`).
Writer: ordinary OMP DeepSeek worker, one implementation writer.

The RED commands below ran **before** the `crates/core/src/card.rs`,
`crates/core/src/catalog.rs` and CLI help edits. The sourced fixtures
(`fixtures/starter-corpus/sourced-cards.json`, deck snapshot, cards snapshot)
and the exact list `decks/starter-corpus/jasmine-vanilla-stompy.txt` were created
first from captured Scryfall printing data; creating data is not the production
card-support edit.

## RED 1 — vanilla cards and Jasmine are absent from the pool

Command:

```
cargo test -q -p aperture-core --test starter_corpus
```

Observed (7 tests, 1 passed, 6 failed):

```
every_sourced_vanilla_creature_matches_its_printing --- FAILED
  panicked at crates\core\tests\starter_corpus.rs:98:32:
  Alaborn Trooper is not in the supported pool

every_sourced_vanilla_creature_casts_and_resolves --- FAILED
  panicked at crates\core\tests\starter_corpus.rs:143:41:
  supported vanilla

jasmine_boreal_is_a_typed_supported_commander --- FAILED
  panicked at crates\core\tests\starter_corpus.rs:185:54:
  Jasmine Boreal is supported

jasmine_is_refused_in_every_ordinary_library --- FAILED
  panicked at crates\core\tests\starter_corpus.rs:215:5:
  line 1: 'Jasmine Boreal' is not in the supported pool

jasmine_commander_cast_tax_and_return_use_the_existing_paths --- FAILED
  panicked at crates\core\tests\starter_corpus.rs:46:62:
  the starter library parses: "line 7: 'Savannah Lions' is not in the supported pool"

the_first_starter_library_is_exact_and_fully_supported --- FAILED
  panicked at crates\core\tests\starter_corpus.rs:46:62:
  the starter library parses: "line 7: 'Savannah Lions' is not in the supported pool"
```

Expected failure: the behaviors under test (catalog conformance, cast/resolve,
typed commander registry, ordinary-library refusal, exact 99-card shape) cannot
pass until the cards exist. `the_starter_manifest_names_eight_targets_and_claims_only_one`
passed at RED because the manifest is data and was already written; it is not
claimed as TDD evidence.

## RED 2 — offline analyzer reports the exact starter list unsupported

Command:

```
cargo test -q -p aperture-import --test starter_corpus_analyzer
```

Observed: 3 tests, 1 passed, 2 failed.

```
the_exact_starter_list_is_format_legal_and_engine_supported --- FAILED
  left: Unsupported
 right: Supported
  (first unsupported entries: Alaborn Trooper, Alpha Tyrranax, ... reason
   "not in the implemented 31-card practice pool")

the_deck_snapshot_binds_the_exact_text_library --- FAILED
  panicked at crates\import\tests\starter_corpus_analyzer.rs:74:78:
  deck text: "line 7: 'Savannah Lions' is not in the supported pool"
```

`analyzer_gameplay_refusal_is_preserved` passed at RED deliberately: it pins
the existing blanket gameplay refusal, which this slice must not relax.

## GREEN — implementation order

1. `crates/core/src/card.rs`: append 47 vanilla `CardId`s and `JasmineBoreal`;
   replace the one-card `matches!` allow-list with the typed
   `SUPPORTED_COMMANDERS` slice (`IsamaruHoundOfKonda`, `JasmineBoreal`) and keep
   the legend-rule refusal in the deck parser and engine constructors.
2. `crates/core/src/catalog.rs`: append the 47 sourced vanilla entries plus the
   commander entry; no effect/ability/static semantics were added.
3. `crates/core/tests/card_printings.rs`: make the provenance guard data-driven
   for the starter fixture (48 new cards) while retaining the hand-recorded and
   frozen legacy lists.
4. `crates/core/tests/issue13_commander_combat.rs`: update the truthful pinned
   pool size from 31 to 79 with the reason.
5. `crates/cli/src/decks.rs`: a file-loaded deck keeps its own name instead of
   inheriting the bundled fallback name, so exact snapshots are not mislabeled.
6. `crates/cli/src/main.rs`: correct the `--commander` help text.

Post-GREEN focused runs:

```
cargo test -q -p aperture-core --test starter_corpus
  -> 8 passed

cargo test -q -p aperture-import --test starter_corpus_analyzer
  -> 3 passed

cargo test -q -p aperture-core --test card_printings --test catalog_pool
  -> 3 passed, 5 passed
```

## Assignment 2 — planned corpus structure

RED before the seven planned files/manifest paths existed:

```
cargo test -q -p aperture-import --test starter_corpus_planned
  panicked at crates/import/tests/starter_corpus_planned.rs:76:32:
  isamaru-white-aggro has no exact desired list
```

GREEN after the exact lists + snapshots were generated (see
`evidence/issue17/IMPLEMENTATION-HANDOFF-2.md`): same test -> 1 passed. The test
reads planned-list text with a local metadata-only parser; the engine's
`DeckList::parse_practice` still refuses every unsupported name.

The eighth assignment-1 test (`jasmine_commander_damage_uses_the_existing_ledger`)
was added after GREEN as regression coverage for the requested "Commander damage
uses existing paths" behavior; it is not claimed as RED evidence. It stages
Jasmine on the battlefield through public state, declares her as the only
attacker, resolves combat and asserts 4 damage is credited to her
`CommanderId` against P1.
