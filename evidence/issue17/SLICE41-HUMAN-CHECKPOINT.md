# Issue17 — slice 41 checkpoint (The Earth King)

Card / branch / PR / histories unchanged: kanban `t_67778feb`, branch
`aperture/issue-17`, draft PR46. Approved increments 1–40 are untouched; slice 40
was approved by review 307 at `a9e3429` and this slice starts from that head.

**Original issue-17 contract remains unfinished**: 1 of 8 desired 100-card lists
certified, Ayula **12** names short, **318** of 459 union names unsupported
(catalog 153, supported 141). No merge, master push, closure or downstream unlock.

## What landed

`The Earth King` (`{3}{G}`, 2/2 Legendary Creature — Human Noble Ally; Scryfall
**TLA 172**, immutable printing `a8d5dca6-381a-4361-b265-27de8d04335c`, oracle
`d5770b0f-4493-42e9-a2d7-0e74a28da4ba`, retrieved **2026-10-28T07:02:29Z**; raw
response + retrieval stamp in `slice41/sources/`, fixture
`fixtures/starter-corpus/slice41-sourced.json` chained into `card_printings`).

* **Distinct 4/4 Bear token.** `CardId::EarthKingBearToken` is a separate
  characteristic card from the existing 2/2 `BearToken`; it follows the pinned
  CR 111.4 name ("Bear Token"), is never importable, and is registered in
  `card::TOKEN_CARDS`, which is the single source of `is_token()` — so the
  CR 111.7 cease-to-exist path applies to it by construction.
* **Attack trigger, once per attack event.** `qualifying` counts the declared
  attackers that are the trigger controller's live creatures with effective
  power ≥ 4 (layer-aware, so pumps and animation count) and captures the count
  on the trigger; opponents' creatures never count, and the trigger is staged
  once for the batch rather than once per attacker.
* **Incremental private basic-land search.** A new `Pending::SearchLibrary`
  holds `remaining`, `chosen` and `eligible`; `Action::SearchLibraryPick` /
  `SearchLibraryDone` are the offered actions, so a library with many basics
  never needs a combinatorial offer list. Only basic lands in the searching
  player's library are eligible, the choice is owner-only in projections, and
  the done instruction puts each chosen land onto the battlefield **tapped**
  through the same entry path as any other permanent, then shuffles through the
  engine RNG even when nothing was chosen.
* **Landfall on searched lands (defect found and fixed by the native owner).**
  The writer's first cut put searched lands onto the battlefield without firing
  existing landfall watchers, because `apply_play_land` held the only call that
  queued `LandEnters`. The land-entry hook now lives in `enter_battlefield`, so
  any effect that puts a land onto the battlefield fires landfall once per land
  and no land play is spent.

## Verification (native owner, this worktree)

* `evidence/issue17/slice41/native-red-landfall.log` — the retained **genuine
  RED**: with the writer's code as committed, adding the landfall regression
  failed on `left: 20, right: 22` (the searched lands gained no life for Jaddi
  Offshoot). It went green only after the entry-path fix, so the test
  discriminates.
* `evidence/issue17/slice41/native-green.log` — `issue17_slice41` **9/9**,
  exit 0. New native-owner tests beyond the writer's four:
  `searched_lands_fire_existing_landfall_once_each_without_spending_land_play`,
  `attack_search_resolves_from_the_captured_count_after_its_source_leaves`
  (Swords to Plowshares on the King with its trigger on the stack: the search
  still opens with the captured amount),
  `carved_bear_token_ceases_to_exist_after_leaving_the_battlefield` (real Swords
  cast → "ceases to exist", object gone, absent from the graveyard,
  `state.validate()` Ok), and
  `zero_pick_search_still_shuffles_and_forged_or_duplicate_picks_are_atomic_refusals`.
* `evidence/issue17/slice41/native-gates.log` — every step's own exit status:
  `fmt_exit=0`, `slice41_exit=0`, `card_printings_exit=0`,
  `catalog_pool_exit=0`, `issue17_slice15_corpus_exit=0`,
  `issue13_commander_combat_exit=0`, `replay_digest_exit=0`,
  `counters_tokens_library_exit=0`, `abilities_exit=0`, `clippy_exit=0`,
  `workspace_exit=0`, `release_exit=0`.
* `evidence/issue17/slice41/native-corpus-audit.log` — fresh audit, `AUDIT OK`:
  153 catalog, 459 union = 141 supported + 318 missing, per list
  `[55, 0, 51, 48, 12, 52, 52, 53]`, Ayula 12, 1 of 8 certified.
* `evidence/issue17/slice41/separate-cli-replay.log` — the release CLI replays
  four recorded games and every digest matches: slice 41 `d646deedfac9`
  (1780 actions, 4576 events), slice 38 `726fe20e72e1` (3861), slice 35
  `1c13d8fdcdce` (5347), slice 37 `c4c5985cd048` (4530). The last three are the
  behaviour-preservation check for the shared land-entry hook: no recorded game
  moved.

## Four-seat journey

`evidence/issue17/slice41-journey-driver` plays `decks/package-slice41.txt`
(40 cards, 4 seats, 40 life, seed 2323, Jasmine Boreal in the command zone for
every seat) choosing only from the masked legal actions of the seat with
priority. Terminal `Win{P3}`, 1780 decisions, 40 turns, 4576 replay events,
digest `d646deedfac9b96744448b7f981f74f28825279022a64970f7651ba2f59bf127`,
snapshot SHA-256 `d7805f3f2ccdc6fad30c8bfced4d636c44877839bf9f3509810c310c8db00f58`.
It casts The Earth King 11 times, creates 11 Bear tokens, and **14 attack
triggers resolve into 14 shuffles carrying 18 tapped basic-land entries**, with
the replay verifying and a fresh `SessionSave` reloading. It is a 40-card
evidence package and a masked demonstrator, not a starter certificate and not
product AI or playing-strength evidence.

## Disclosed limits

* **The legend rule (CR 704.5j) is not implemented and The Earth King is the
  first legendary card the pool admits into a library.** Until now every
  legendary card was gated commander-only by `DeckList::parse_practice`, so the
  state-based action had no reachable input; the evidence package even carries
  four copies, so two same-name legendary permanents can coexist on this engine.
  A real Commander list is singleton, so the Ayula certification path cannot
  reach that state, but the gap is real for ordinary practice decks and is
  stated in `docs/issue17-starter-corpus.md` rather than described as an
  existing convention. Closing it means implementing the CR 704.5j choice.
* The search offers one action per eligible basic land; with a very large basic
  count the offer list is long (but linear, not combinatorial) and no cap is
  applied.
* `Pending::SearchLibrary` validation added this slice refuses malformed saves
  (a chosen card outside `eligible`, a duplicate choice, a non-basic or
  non-library object, a search outside a resolution); it does not cross-check
  `remaining` against the amount the trigger originally captured, which is not
  stored.
* Review 290's two non-blocking notes stay open (7b-before-7a ordering
  inversion, unreachable today; `card_printings` colour-identity wording).
* Slice 41 is **unverified by an independent reviewer**; it was authored by this
  run. An independent review of the exact head is the next gate.
