# Issue 17 slice 2 — Krenko goblin-token package handoff

Status: **PARTIAL #17**. The first reusable token package is implemented and
exercised; the exact 100-card Krenko desired list remains `planned` and is not
claimed supported or playable. No commit/push/merge/closure.

## What works

* Six sourced cards added: **Krenko, Mob Boss** (commander-only), **Dragon
  Fodder**, **Krenko's Command**, **Hordeling Outburst**, **Goblin Instigator**,
  **Beetleback Chief**.
* Engine-only **1/1 red Goblin** token card: not in `catalog::all`/`by_name`, not
  importable; each token is a unique `ObjectId` on the battlefield with correct
  controller, red color, Goblin subtype, 1/1 P/T and summoning sickness.
* Typed effects: `CreateGoblinTokens(n)` and
  `CreateGoblinTokensForEachGoblin` (Krenko counts every Goblin its controller
  controls on resolution, including Krenko and tokens).
* ETB triggers for Goblin Instigator (one token) and Beetleback Chief (two
  tokens) use the existing trigger queue/APNAP ordering; no priority is granted
  during resolution.
* Krenko uses the existing `{T}` activated-ability path: summoning sickness and
  tapped-source restrictions are enforced by the existing engine checks.
* Tokens cease to exist on leaving the battlefield (`Zone::Removed`), never enter
  a graveyard, and are removed by the existing SBA path.
* Replay/save digests round-trip; adding token CardIds/effects did not change
  the default-empty state fields, save version, replay version or rules
  revision.

## Source records

`fixtures/starter-corpus/krenko-tokens-sourced.json`: six card records copied
from the stored real Scryfall planned-list printing response (retrieved
`2026-10-25T08:32:00Z`) plus the live Scryfall Goblin token record. Fields:
name, role, oracle_id, Scryfall id, set/collector, layout, cost, type line,
power/toughness, Oracle text, color identity, keywords, Commander legality and
printing URL. Texts match the catalog exactly.

## Tests / RED-GREEN

Honest note: this slice was implementation-first; the regression file
`crates/core/tests/issue17_tokens.rs` was written alongside the engine work, so
I do **not** claim a retrospective RED for each mechanic. First execution
compiled with test-harness type errors, then 6/7 passed; the one failure was my
wrong expectation that a ceased token disappears from the object map. The engine
correctly retained it in `Zone::Removed`, and the assertion was corrected to the
engine's existing cessation contract.

Focused GREEN (log `evidence/issue17/slice2-focused-tests.log`):

```
cargo test -q -p aperture-core --test issue17_tokens --test catalog_pool \
  --test card_printings --test command_zone_setup --test starter_corpus \
  -p aperture-import --test starter_corpus_planned --test starter_corpus_analyzer
  -> exit 0; 7, 5, 4, 33, 8, 1, 3 tests passed
```

Covered: sourced text/cost/type/P-T; token non-importability; printed token
counts (2/2/3) with unique ids and red 1/1 Goblin characteristics; Instigator
and Chief ETB triggers; Krenko summoning-sickness/tap restriction and dynamic
Goblin count; token cessation; serialization digest round-trip.

Gates: `cargo fmt --all --check` and
`cargo clippy -q --all-targets -- -D warnings` exit 0 (`evidence/issue17/slice2-gates.log`),
release CLI build exit 0.

## Executable journey

`decks/package-krenko-tokens.txt` is a clearly named **40-card supported package
fixture**, not the desired 100-card Krenko starter. Four seats, each the same
exact package + `Krenko, Mob Boss` commander, Commander-combat practice 40 life:

```
target/release/aperture-cli.exe selfplay --seed 7 --max-steps 4000 \
  --deck decks/package-krenko-tokens.txt (x4) \
  --commander-combat-practice \
  --commander 0..3:"Krenko, Mob Boss" \
  --verbose --out evidence/issue17/slice2-krenko-game.json \
  --replay-out evidence/issue17/slice2-krenko-game.replay.json
```

Result: **terminal**, winner P0, 1805 steps / 40 turns, 40 starting life.
Report/log: `slice2-krenko-game.json`, `.log`. Actual events include
**317 token creations**, Krenko activations by all four seats, Instigator/Chief
ETB token triggers, 25 casts and 20 ability activations.

```
target/release/aperture-cli.exe replay --file evidence/issue17/slice2-krenko-game.replay.json \
  --save-out evidence/issue17/slice2-krenko-save.json
  -> digest f4768c8a3bd2 matches, 5930 events
target/release/aperture-cli.exe replay --file evidence/issue17/slice2-krenko-save.json
  -> stored/recomputed digest f4768c8a3bd2, reproduces the save
```

Logs: `slice2-replay.log`, `slice2-save-replay.log`.

## Unfinished / next

* Krenko desired list: 52 of its nonbasic/commander names still missing; stays
  `planned`. Manifest `missing_cards` recomputed against the 85-card pool.
* No Talrand registration: Talrand needs a spell-cast trigger plus flying, and
  neither is implemented completely; no fake flying/copies.
* Tokens are one specific Goblin type only; no generic token definitions,
  sacrifice/death payoffs on tokens, or tribal static effects.
* Legend rule still absent, so Krenko is commander-only.
* No priority during effect resolution is the existing engine behavior, not a
  new priority model.

Concrete next package: Talrand, Sky Summoner only if a complete spell-cast
triggered token effect and real flying semantics can be added together; otherwise
implement `Goblin Chieftain`-style tribal statics or a generic token definition
schema before broadening the Krenko list.
