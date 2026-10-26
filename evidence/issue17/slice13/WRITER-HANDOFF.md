# Issue 17 slice 13 — landfall counters and Beast/Pest tokens

Exact head recorded after commit. No merge/master push. Full #17 unfinished:
**122 catalog / 459 union = 110 supported + 349 missing**, 1 certified / 7 planned.

## Sources and cards

Three records extracted from the coordinator’s immutable
`evidence/issue17/slice13/candidate-source.json` (retrieved
`2026-10-26T17:44:38Z`, 0 not-found) into
`fixtures/starter-corpus/slice13-sourced.json` with Scryfall id, API printing
URL and retrieval stamp:

* **Sazh's Chocobo** — `{G}` 0/1 Bird; landfall put a +1/+1 counter on itself.
* **Rampaging Baloths** — `{4}{G}{G}` 6/6 Beast with Trample; landfall create a
  4/4 green Beast token.
* **Professor of Zoomancy** — `{3}{G}` 4/3 Bear Druid; ETB create a 1/1 black
  and green Pest token with “When this token dies, you gain 1 life.”

Token names follow pinned CR 111.4 generated-token naming: **Beast Token** /
**Pest Token**, not Scryfall token labels. Token CardIds are absent from
`catalog::all`/`by_name`, so they cannot be imported or cast as cards.

## Engine behavior

* New declarative effects: `PutPlusOneCounterOnSource`, `CreateBeastToken`,
  `CreatePestToken`. The counter uses typed `Object.counters`
  (`CounterKind::PlusOnePlusOne`), not an end-of-turn pump or the legacy display
  field; it persists through cleanup and alters actual P/T.
* Landfall uses the existing controller-scoped `LandEnters` queue. A queued
  source-left token-creating landfall still creates the token; a targetless
  counter effect whose source has left is a normal no-op, never a stale
  ObjectId replacement.
* Pest has its own typed dies trigger; it gains 1 life exactly once after
  death, before the token ceases, and causes no opponent drain.
* Existing token creation/ETB watcher hook is reused; Goblin/Drake and slice12
  Removed archive compatibility unchanged.

## Genuine RED

1. Absent catalog: all four slice13 tests failed at `catalog::by_name`.
2. After card/effect scaffolding with no-op mechanics: Sazh counters stayed 0;
   Baloths produced no Beast; Professor produced no Pest — genuine mechanic RED.
3. A focused run then caught an over-broad repair-3 predicate: the archival
   reference check was applied to every departed-owner Removed object, rejecting
   legitimate non-token APNAP source-left provenance. Scoped it to tokens only;
   `four_seat_co_loser_apnap_source_departure_continues_survivors` is green again.

## Verification

```
issue17_slice13            4 passed
catalog_pool               5 passed
card_printings             7 passed
issue13_commander_combat  43 passed
issue17_tokens            10 passed
issue17_slice6            13 passed
cargo fmt --all --check   exit 0
cargo clippy --all-targets -D warnings  exit 0
cargo build --release -p aperture-cli   exit 0
```

## Journey

`decks/package-slice13.txt` — 40-card supported package, **not** a desired
100-card starter. Four seats, Krenko commander each, 40 life, seed 1301:

* terminal, P1 wins, 1568 steps / 34 turns, digest `4cbaa34f0de1…`
* actual events: Sazh’s Chocobo cast 2, Rampaging Baloths cast 3, Professor cast
  4, 18 +1/+1 counter events, 1 Beast token creation, 4 Pest token creations
* separate replay matches 5018 events; `--save-out` save reloads/replays to the
  same digest
* no Pest death occurred in this heuristic trace; the focused test proves the
  dies/gain-life behavior. No event count is inferred from cards merely drawn.

## Corpus/docs

New audit only: `evidence/issue17/slice13/corpus-audit.py` →
`corpus-state.json`. Current catalog **122**, union 459, supported **110**,
missing **349**, manifest matches. Per-list: Isamaru 55, Jasmine 0, Talrand 51,
Krenko 48, Ayula 42, Elas 53, Tatyova 52, Zada 53. Eight desired snapshots
unchanged. Historical scripts/results untouched.

## Remaining gaps

No desired 100-card starter is newly certified. Friendly Teddy/Aetherize/
Commander’s Sphere exclusions remain; broader legend/equipment/tutor/reanimation
and other Ayula/Tatyova cards remain unsupported.
