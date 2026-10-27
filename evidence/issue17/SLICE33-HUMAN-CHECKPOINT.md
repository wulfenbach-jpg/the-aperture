# Slice 33 — Beorn's Hospitality (HOB 120): the pool's first layer 7a characteristic-defining power/toughness

Date: 2026-10-27. Branch `aperture/issue-17`, draft PR46, one writer.

Continues from the approved slice-32 head `e165873` (review 289 APPROVED_INCREMENT at `770cb64`, code head `40a87aa`). Approved increments 1–32 are preserved without re-audit.

## Sourcing

| Card | Printing | Immutable id | Oracle id | Retrieved (UTC) | Fixture |
|---|---|---|---|---|---|
| Beorn's Hospitality | Scryfall **HOB 120** | `153ca57e-30f0-4ad7-ae9d-c55cbf0fd4c9` | `d9ed7252-11d1-432b-9101-ac08cd28826d` | **2026-10-27T18:21:29.939111Z** | `fixtures/starter-corpus/slice33-sourced.json` |

`evidence/issue17/slice33/fetch-source.py` fetches by exact name, records the verbatim provider response (`sources/hob-120-raw.json`), lists every printing (`sources/hob-120-printings.json`) and **refuses** the record unless all printings carry one identical `oracle_text`. Note for the reviewer: the provider answers `GET /cards/hob/120` with HTTP 400 from this host while `GET /cards/named?exact=…` and `GET /cards/<id>` answer 200, and Python's `urllib` is refused where `curl` succeeds, so the helper transports through bounded `curl` calls (`--max-time 60`, non-zero exit is a hard failure).

Printed text, verbatim:

> Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature you control.
> {5}{G}{G}: This enchantment becomes a Bear creature in addition to its other types and gains "This creature's power and toughness are each equal to the number of lands you control." (This effect doesn't end.)

## What is new and reusable

* **Layer 7a exists now.** `Layer::CharacteristicDefining` (label `7a`, CR 613.4a) + `ContinuousKind::PowerToughnessEqualToLands`. The pass sits between the layer 6 ability grants and the layer 7b setters in `Game::recompute_layers`, and it **recomputes the value from the board on every pass** (`Game::lands_controlled_by`) instead of capturing it at resolution. That is the difference this slice is really about: a land that enters after the animation changes the P/T with no second activation (CR 604.3), and a 7b snapshot would fail it. Counters still land on top through `Object::power`/`toughness`, which is the 7d contribution.
* **A duration that does not end.** `EffectDuration::WhileSourceOnBattlefield` is used for an activated ability for the first time, because the printing says "(This effect doesn't end.)" (CR 611.2b). Cleanup therefore leaves the animation alone; departure pruning (CR 400.7) ends it with the object, through the existing pipeline.
* **Additive animation.** The layer 4 half reuses `ContinuousKind::AddCreatureSubtypes`, so the permanent is an Enchantment **and** a Bear creature: `Game::is_creature`, the `Bear` target spec and `has_subtype` all answer for it, and the printed type line is untouched.
* **The first targeted landfall watcher.** `TargetSpec::CreatureYouControl` with the existing `Effect::PutPlusOneCountersOnTarget(1)`. With exactly one legal target the engine chooses it (existing `stage_triggers` behaviour); with several it asks through `Pending::TriggerTarget`; with none it does not put the trigger on the stack (CR 603.3d).
* **`card_green!` catalog macro.** The pool records `colors` only where a supported rule can read colour; this permanent becomes a creature, so its printed colour is now an observable characteristic and is recorded (CR 202.2a).
* **`SourceContinuous`** groups the continuous-effect helper's arguments, keeping `start_source_continuous` inside clippy's argument limit while carrying the printed duration.

## Verification actually run (logs in `evidence/issue17/slice33/`)

* `issue17_slice33` — **7/7** focused tests: the sourced catalog row and its two typed abilities; the animation adding the creature type and Bear subtype while keeping Enchantment, with **P/T following the land count live** (3/3 → 4/4 after the fourth land, no second activation); a 7c pump and a +1/+1 counter on top of the CDA value (7/7) and the return to the land count; the animation surviving cleanup, still animated and still a Bear on the next turn, then ending when the permanent dies to lethal damage; the targeted landfall trigger with two legal targets (both offered, the opponent's excluded) and exactly one counter; an opponent's land not triggering it; and the ability not being offered without `{5}{G}{G}`.
* `card_printings` — **13/13**, including the new record test that ties the catalog row to the HOB 120 printing (cost, type, oracle text, colour, keyword list) and to the two typed abilities' text.
* **Four discriminating mutation REDs** (`run-mutations.py`): the CDA value snapshotted to `0`, the additive Bear subtype ignored, the animation's duration forced to until-end-of-turn, and the landfall trigger pointed at the wrong event. Each fails its named test (and, where the mutation breaks the shared animated-permanent precondition, neighbouring tests too — reported in each log rather than hidden), and every touched file is restored **byte-for-byte** (`pre-`/`post-mutation-sha256.txt`).
* Gates: `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets --locked -- -D warnings` **exit 0**, `cargo test --workspace --locked` **119 suites / 841 passed / 0 failed** (`WORKSPACE_EXIT=0`).
* Four-seat / 40-life exact-snapshot journey from `evidence/issue17/slice33-journey-driver` (`decks/package-slice33.txt`, seed 3333, Jasmine Boreal injected for all four seats by the demonstrator): terminal **Win{P3}**, **1117 actions**, 24 turns, **2967 replay events**, digest `3dfb14d4ce762cdcf52bd9c8d5f1832cebe3dbd4584b21c4cd1dff57dfd14c22`, snapshot sha256 `a7a45e4b4165b6431b285d584fd1a5dd60e599f3ca555e82def717a7cb7c581d`; 6 Beorn's Hospitality casts, **2 animation activations**, 4 `ContinuousEffectStarted` events, **13 landfall triggers staged**, 4 +1/+1 counters in play at the end. The driver re-asserts the CDA invariant **after every accepted action** (`pump_power == lands controlled`, `power() == lands + counters`, `is_creature`) with **29 on-turn** and **28 off-turn** sightings — the off-turn ones are the evidence that the animation is not until-end-of-turn. Re-running with `--locked` reproduces `demo-journey.json`, `demo-save.json` and `demo-replay.json` **byte-identically**, and a separate release `aperture-cli replay` of the recorded replay matches the digest (`cli-replay.log`).
* Independent corpus audit (`corpus-audit.py`, recomputed from `catalog.rs` + the eight desired decks + commanders and asserted equal to every manifest `missing_cards` entry, with the new `card_green!` macro in the parser): **149 catalog / 459 union = 137 supported + 322 missing**, per list `55/0/51/48/16/52/52/53`, Ayula **16**, **1 of 8 certified**.

## Disclosures

* The journey is a **40-card package**, not a 100-card starter certificate, and the driver is an evidence-only masked demonstrator, not product AI or strength evidence.
* The characteristic-defining "you" is read from the permanent's **current controller**. The pool has no control-changing effect, so this is indistinguishable from the resolving controller today; it is recorded rather than silently assumed.
* `card_green!` records a colour for this one card. The other printable cards still carry `colors: &[]`, which is a pre-existing modelling choice across the whole pool, not a claim that they are colourless. Only this card needed the value because only this card becomes a creature.
* The animation can be activated again on an already animated permanent; the engine then holds two effects per layer for it. The result is unchanged (the 7a pass overwrites, the type addition is idempotent), and no test depends on the duplicate being absent.
* Pre-existing, untouched: three tracked slice-23 evidence files carry uncommitted edits from earlier runs; they are not part of this slice's record.

## Exact-head CI

* Tip `e9ec8f50b0671bf48f01cf2f700fa02537e34ca0` (implementation, tests, fixture, docs **and** the slice evidence): **10/10 check runs completed SUCCESS, every `head_sha` = `e9ec8f50`**, capture exit 0 — `evidence/issue17/slice33/exact-head-ci-e9ec8f5.json`, log `ci-capture-tip.log`.
* The code-only commit `01fdc3b` had no check runs of its own: pushing the evidence commit cancelled its in-flight runs (`cancel-in-progress`), which the receipt `exact-head-ci-01fdc3b.json` records as `cancelled` rather than as a pass.
* The commit that adds this receipt line is documentation-only; it inherits the `e9ec8f5` runs and has no runs of its own.

## State after this slice

Original issue #17 remains **unfinished**: 1 of 8 desired 100-card lists certified, Ayula 16 names short, 322 of 459 union names unsupported. No merge, no master push, no issue closure, no downstream unlock (`t_26a7a4ff`, `t_3c2fe51a` stay parked). No new worker, team or PR was created.
