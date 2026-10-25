# Issue 15 — replacement, prevention and layered continuous effects

Rules citation: Comprehensive Rules revision pinned by `RULES_REVISION`
(2026-09-25). Everything below is the typed rules layer; no UI or AI code decides
these outcomes.

## What is implemented

| Rule | Mechanism | Tests |
| --- | --- | --- |
| CR 613.4a | Layer **7a**: characteristic-defining power/toughness (`Wight of Precinct Six`) recomputed from the game state before any modifier. | `characteristic_defining_base_applies_before_modifiers`, `characteristic_defining_zero_toughness_dies` |
| CR 613.4c, 613.7 | Layer **7c**: until-end-of-turn modifiers stored as typed `ContinuousEffect` records with a monotonic timestamp, applied in timestamp order after 7a. | same tests plus `until_end_of_turn_effects_end_in_cleanup` |
| CR 514.2 | Cleanup removes every `UntilEndOfTurn` effect and every prevention shield, then recomputes the layers. | `until_end_of_turn_effects_end_in_cleanup` |
| CR 614.1a, 614.5 | Replacement effect: damage dealt by a creature its controller controls is doubled, applied **once** per damage event. | `damage_multiplier_doubles_a_controlled_creature_once_per_event` |
| CR 615.1, 615.2 | Prevention shield with a finite amount (`Mending Hands`, 4 points) that absorbs damage instance by instance and is removed when exhausted. | `prevention_shield_absorbs_its_amount_and_stops_applying` |
| CR 616.1 | Order and chooser: the affected player (or the controller of the affected permanent) chooses the order. The engine acts for that chooser with a documented policy — **prevention before multiplication** — and records the chooser in `DamagePrevented` / `ReplacementApplied`. | `prevention_is_applied_before_the_multiplier` |
| CR 613/614/615 in a real game | Real two-seat `Game::new` run over the new cards that logs `DamageShieldCreated`, `DamagePrevented` and `ContinuousEffectStarted` and replays to the identical digest **and** event trace. | `a_real_game_logs_the_new_effects_and_replays_to_the_same_digest` |

New events: `DamageShieldCreated`, `DamagePrevented`, `ReplacementApplied`,
`ContinuousEffectStarted`. Each renders through `view::render_event` with its
rules citation.

## Deliberate exclusions (disclosed, not approximated)

* Layers **4, 5, 6** (type, colour, ability) and **7b** (base power/toughness
  setting) are not implemented: no supported card needs them, and a card that
  does is refused at import.
* Layer **7d** (+1/+1 counters) is **not** applied here. Counters remain
  display-only; making them characteristic-changing belongs to the counters
  lane, so this lane never claims 7d coverage.
* No damage **redirection**, no colour-based prevention (`Circle of
  Protection`), no prevention that outlives the turn, and no prevention shield
  that survives its source leaving the battlefield as a static ability.
* Replacement effects are limited to the printed damage-doubling wording; there
  is no general "instead" engine for arbitrary cards.
* No interact channel is offered for the CR 616.1 order choice: the engine
  records the chooser and applies the policy above deterministically.
* The pool stays a bounded 31-card practice corpus; this is not 100-card
  Commander and not full layer coverage.

## State schema note

`GameState` gained `effects`, `shields`, `next_effect` and `next_timestamp`
(serialized, `#[serde(default)]`). Older recorded replays therefore no longer
reproduce their stored digests; issue-15 evidence is regenerated on this head.
