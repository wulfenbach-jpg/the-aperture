# Issue 17 — implementation handoff (slice 1, first starter package)

**Result: PARTIAL #17.** One supported starter is implemented, sourced,
analyzer-legal/supported and played as an exact four-seat 100-card snapshot.
The eight-list release gate remains open. No commit, push, PR, merge or issue
closure was performed.

* Branch: `aperture/issue-17`
* Base: `8a72cc607ea25f203be672c0007ad5e798be7145`
* Writer: ordinary OMP DeepSeek worker (not Vibe), sole writer
* Machine result: `evidence/issue17/implementation-result.json`
* TDD log: `evidence/issue17/TDD.md`

## Delivered package

`jasmine-vanilla-stompy`:

| item | value |
| --- | --- |
| commander | **Jasmine Boreal** — `{3}{G}{W}`, Legendary Creature — Human, 4/5, printed rules text empty |
| library | **99 cards** in `decks/starter-corpus/jasmine-vanilla-stompy.txt`; commander supplied separately |
| unique nonland cards | **58** (all singleton, all supported) |
| new sourced vanilla creatures | **47** |
| previously supported green/white nonland cards | 11 (4 vanilla creatures, 4 instants, 3 typed-ability creatures) |
| basic lands | **41** (20 Plains, 21 Forest) |
| total with commander | 100 |
| pool size | 31 → **79** (31 previous + 47 vanilla + Jasmine) |

No legendary library card, artifact/enchantment/snow creature, duplicate
nonbasic singleton, keyword, token, counter or unsupported text is used. The
unimplemented legend rule is not exercised outside the commander slot.

The commander registry is now the typed
`CardId::SUPPORTED_COMMANDERS = [IsamaruHoundOfKonda, JasmineBoreal]`; the
one-commander-per-seat and ordinary-library refusals are unchanged. No state
field, digest input, `RULES_REVISION`, replay version or save version changed.

## Source capture

All 61 referenced cards were fetched from Scryfall on **2026-10-25T07:59:45Z**:

* `GET https://api.scryfall.com/cards/named?exact=<name>` then
  `GET https://api.scryfall.com/cards/{set}/{collector_number}` for the returned
  concrete printing, paced at 120 ms.
* Raw responses, URLs and retrieval time:
  `evidence/issue17/scryfall-responses.json`.
* Per-card compact declaration:
  `fixtures/starter-corpus/sourced-cards.json` (47 vanilla, 11 existing,
  2 basic lands, 1 commander; `role` is a local classification, all card fields
  are provider data).
* Kind `real` card snapshot and exact deck snapshot:
  `fixtures/starter-corpus/jasmine-vanilla-stompy.cards.json` and
  `...deck.json`, with a fixture manifest binding `cards_sha256` and
  `deck_sha256`.
* Reproduction scripts: `evidence/issue17/source-starter-cards.py` and
  `evidence/issue17/generate-starter-fixtures.py`. A first re-run hit Scryfall
  HTTP 429; the successful capture at 07:59:45Z is the one recorded. No card
  metadata was invented.

A printing is not enough for support: every new card is also covered by
`crates/core/tests/starter_corpus.rs`, which asserts catalog name/cost/type/
power/toughness/empty rules **and** casts and resolves each of the 47 vanilla
creatures.

## Verification actually run

| gate | command | result |
| --- | --- | --- |
| focused tests | `cargo test -q -p aperture-core --test starter_corpus --test card_printings --test catalog_pool --test command_zone_setup --test practice_commander_guard --test issue13_commander_combat -p aperture-import --test starter_corpus_analyzer` | exit 0; log `evidence/issue17/focused-tests.log` |
| full suite | `cargo test -q --all-targets` | exit 0; **468 passed / 0 failed**, 71 suites, log `evidence/issue17/all-targets-tests.log` |
| fmt | `cargo fmt --all --check` | exit 0 |
| clippy | `cargo clippy -q --all-targets -- -D warnings` | exit 0 |
| release CLI | `cargo build --release -p aperture-cli` | exit 0 |
| analyzer | `target/release/aperture-cli.exe commander-check --deck fixtures/starter-corpus/jasmine-vanilla-stompy.deck.json --cards fixtures/starter-corpus/jasmine-vanilla-stompy.cards.json --json` | exit 0; format `legal`, engine `supported`, 1 commander / 99 library / 100 total / 61 unique names, report `evidence/issue17/commander-check.json` |
| exact four-seat game | release `selfplay --seed 1 --max-steps 12000` with the starter file as all four `--deck` paths, `--commander-combat-practice`, and `--commander 0..3:"Jasmine Boreal"` | exit 0; **terminal**, winner P1, **2364 steps / 53 turns**, 40 starting life, digest `d7f8c2adaa17…`; 55 casts, 30 distinct new vanilla cards cast, 5 commander casts |
| separate replay | `target/release/aperture-cli.exe replay --file evidence/issue17/jasmine-starter-game.replay.json` | exit 0; replayed digest matches recorded; log `evidence/issue17/replay-verify.log` |
| exact-list assertion | `python evidence/issue17/verify-starter-journey.py` | all four replay seats are exactly the 99-card list, all four commanders are Jasmine, digest matches; output `evidence/issue17/journey-verify.json` |

The game report is `evidence/issue17/jasmine-starter-game.json`; the full event
log (including `P0..P3 casts Jasmine Boreal` and the vanilla casts) is
`evidence/issue17/jasmine-starter-game.verbose.log`. A capped game would have
been reported as capped; this run is terminal.

`--help` was inspected from the release binary. `--commander` now documents both
explicit command-zone profiles and both supported commanders. Loading a deck
file now keeps the file's own name, so replay snapshots are not mislabeled as
"Temur Beats".

## Legacy preservation

No state/digest/version change was made, and the full suite's replay, save/load,
ordering and commander tests pass. A tracked issue-14 evidence replay
(`evidence/issue14-abilities-replay.json`, v6) fails to reproduce its recorded
digest; the **untouched base commit** fails identically with the same computed
and recorded digests. It predates issue-15 state fields and is not caused by
this change. All other tracked evidence replays are v1 and rejected by the
existing version policy. Details: `evidence/issue17/legacy-replay-note.md`.

## Corpus plan and gaps

`decks/starter-corpus/manifest.json` (version 1) lists all eight target ids and
commanders with statuses. `jasmine-vanilla-stompy` is `implemented` (not
`certified`); the other seven are `planned` with `deck_path: null`. No exact
future list is claimed. Six target commanders are not in the supported pool and
are named in each entry's `missing_cards`; interactions needed by the other
archetypes (tokens, triggered/activated abilities, keywords, counters, copying,
tribal payoffs, landfall, drain, sacrifice outlets) are listed in
`missing_interactions`. `docs/issue17-starter-corpus.md` explains the
archetypes, phased gaps and exclusion policy.

Remaining practical limits:

* The eight-list proof, per-card coverage for seven other styles, independent
  review and exact-head CI are still open; only one starter is playable.
* `isamaru-white-aggro` has its commander but no exact 99-card singleton library.
* The offline analyzer keeps its blanket `gameplay.available = false`; bounded
  command-zone/Commander-combat practice is still not the Commander format.
* The native UI's default startup table remains the practice decks; the starter
  is selected through explicit deck paths.
* The AI is the existing deterministic heuristic; no tuning was done.

## Coordinator-owned files

`.starter-run.yml` and `evidence/issue17/IMPLEMENT-1.md` were not edited.

A later `evidence/issue17/IMPLEMENT-2.md` appeared in the workspace during this
run. It was **not executed**: this admission bounded the run to slice 1 plus this
handoff, and the work order says to stop after implementation and handoff for
independent inspection. Its contents and other coordinator files were treated as
read-only.

The reporter pointer still targets completed m3-tabletop/#20. That discrepancy
was disclosed; no shared ops/reporting file was touched.
