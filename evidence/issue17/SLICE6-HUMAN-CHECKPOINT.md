# Issue17 — slice 6 checkpoint (death watchers / target-player package)

Full issue17 is **UNFINISHED**. This is a bounded implementation increment on the
existing draft PR46, not eight certified starter lists. No merge, master push,
issue closure or successor activation was performed.

## Published heads

* draft PR46: https://github.com/wulfenbach-jpg/the-aperture/pull/46
* code head of record: commit in this slice's `feat(corpus): … (slice 6)` plus
  the `fix(corpus): … (S6-R1-F1)` repair below. Local = remote = PR head.
* PR OPEN / MERGEABLE / still draft. Base change recorded but not applied: the
  supervisor reports #16 merged to master at `8de0961` (counters, tokens,
  keywords, library operations); PR46 has **not** been rebased onto it — that is
  the next integration step, and the slice-6 primitives touch
  `card.rs`/`ability.rs`/`engine.rs`/`catalog.rs`, which is the collision surface.

## What is usable now (real behaviour, sourced this run)

| card | printing | new behaviour |
|---|---|---|
| Blood Artist | soc 209 | any creature dies → **target player** loses 1 life, you gain 1 |
| Zulaport Cutthroat | soc 233 | a creature you control (or itself) dies → each opponent loses 1 life, you gain 1 |
| Hedron Crab | tdc 155 | landfall → **target player** mills three cards |

Provenance: `fixtures/starter-corpus/slice6-sourced.json` records printing,
`oracle_text`, P/T, keywords, `printings_checked`, the distinct printed texts and
the retrieval stamp for each card (18 / 14 / 4 printings, **one** distinct text
each). Elas il-Kor, Sadistic Pilgrim was captured too (`slice6-sourced-raw.json`)
but **not** added: it prints **Deathtouch**, which the rules layer does not model.

New primitives, each with focused coverage:

* `TargetSpec::Player` — every live seat is a legal target, the ability's own
  controller included (CR 115.4).
* `Effect::MillTargetPlayer` — the targeted player's library, never the
  controller's.
* `Effect::LoseLife` / `Effect::LoseLifeEachOpponent` — life loss, not damage
  (CR 118.2): only `LifeChange` is emitted, so a real prevention shield
  (Mending Hands) cannot stop it and no damage trigger sees it.
* `TriggerEvent::{CreatureDies, CreatureYouControlDies,
  AnotherCreatureYouControlDies}` — the "creature dies" watcher family, fired for
  card deaths **and** creature-token deaths (single departure path).

## Review round 1 finding (repaired)

Independent review of the first candidate returned **request_changes** for one
real defect (S6-R1-F1): simultaneous deaths were dispatched per object while the
dying objects were moved one at a time, so a watcher that died in the same batch
missed its fellow dying creature, and the result depended on object-id order.
Repair: `check_sbas` snapshots the watchers **before** moving anything
(`death_watchers`), moves the whole batch, and dispatches
`queue_watchers_for_deaths` once for every death in it (CR 704.3, 603.6c). A
watcher's own "this creature … dies" trigger is still queued at its own move, so
it is order-independent too.
Genuine RED on the pre-repair head: the new regression fails there
(`slice6-red-before-batch.log`); GREEN after the repair.

Two documentation nits from the same review are repaired: the catalog module doc
now says 106 cards, and this checkpoint exists at the path HANDOFF references.

## Corpus movement (native recomputation)

`evidence/issue17/slice6/update_manifest.py` recomputes every desired list's
`missing_cards` and asserts each desired deck file is still exactly 99 cards:

* catalog **103 → 106**;
* eight-list union **459** (unchanged — desired lists untouched);
* supported **91 → 94**, missing **368 → 365**;
* elas-sacrifice 57→55 (Blood Artist, Zulaport Cutthroat);
  tatyova-landfall 55→54 (Hedron Crab); other six lists unchanged;
* **1 implemented / 0 certified** starters; seven remain planned;
* no slice-6 card sits outside a desired list.

## Verification actually run

* `crates/core/tests/issue17_slice6.rs` — **10** focused tests.
* Genuine RED (watchers): with the watcher dispatch disabled, exactly the five
  behaviour tests fail and the four provenance/target-spec controls pass
  (`slice6-red-before-watchers.log`); restored → green.
* Genuine RED (S6-R1-F1): the simultaneity regression fails on the pre-repair
  head (`slice6-red-before-batch.log`); green after.
* `cargo test --workspace --locked` → **536 passed, 0 failed** (83 suites)
  (`slice6-workspace-tests.log`); `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo build --workspace --release --locked` → exit 0.
* Real executable four-seat journey (`decks/package-slice6-dies.txt`, exactly 40
  cards, four seats, 40 life, commander-combat practice, seed 610): **terminal**,
  winner P3 by life totals reaching 0, 3002 steps / 67 turns, digest
  `7864e4e956eb…`, 86 resolved triggered abilities, and a separate
  `replay --file` verification: 3002 actions, 7452 events, digest matches
  (`slice6-journey.json`, `.replay.json`, `slice6-journey-replay.log`).
  It is a 40-card **package** fixture, not a desired 100-card starter.

## Honest limitations

* **No desired 100-card starter is certified.** Seven lists are still planned and
  the implemented Jasmine list is unchanged.
* Sacrifice outlets (`Viscera Seer`, `Carrion Feeder`, `Ashnod's Altar`),
  artifacts/equipment (Sol Ring, Greaves, Boots), counters as rules objects,
  tutors, reanimation, Deathtouch and every graveyard *reader* stay
  unimplemented.
* `AnotherCreatureYouControlDies` is wired and documented but no card in the pool
  prints it yet; it is exercised only by the shared watcher path.
* Each-opponent effects still iterate seat order, not APNAP order (only visible in
  log order). Crack the Earth's sacrifice does walk APNAP.
* Milling a commander puts it in the graveyard and lets the existing CR 903.9a SBA
  offer the return; no mill-specific replacement is claimed.
* Shared core hooks (`card.rs`, `ability.rs`, `engine.rs`, `catalog.rs`) remain an
  integration hotspot with the merged #16 work; this slice does **not** approve
  #16 integration and PR46 is not yet rebased onto `8de0961`.
* Reporter pointer still targets completed issue20 and was not changed here.

## Exact resume state (next slice)

* Branch `aperture/issue-17`, PR46 still draft; no merge, no closure.
* Next integration step: rebase onto master `8de0961` and re-run the gates,
  reconciling `Object.counters`/`GameState.revealed` with the catalog additions.
* Cheapest next packages: sacrifice-a-creature cost + `Viscera Seer`/`Carrion
  Feeder` (Elas), artifact mana rocks (Ayula/Isamaru), Deathtouch for Elas.
