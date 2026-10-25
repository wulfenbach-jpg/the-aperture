# Issue 15 — replacement, prevention and layered continuous effects

Rules citation: Comprehensive Rules revision pinned by `RULES_REVISION`
(2026-09-25). Everything below is the typed rules layer; no UI or AI code decides
these outcomes.

## Card text provenance

Every card this lane added to the pool is implemented from a printing, not from
hand-authored wording. The records (set, collector number, Scryfall URI,
retrieval date, how many printings were checked, and the exact oracle text and
P/T) live in `crates/core/tests/card_printings.rs`; the test
`every_new_pool_card_matches_its_recorded_authoritative_printing` fails if the
catalog string ever drifts from them, and
`every_pool_card_added_after_issue_15_has_a_recorded_printing` fails when a card
is added to the pool without a record — so the next hand-authored oracle string
is caught by a test instead of by review.

| Card | Printing recorded | Retrieved | Wording check |
| --- | --- | --- | --- |
| Mending Hands | `9ed` #29 — <https://scryfall.com/card/9ed/29/mending-hands> | 2026-10-24 | 3 printings, 1 distinct oracle text |
| Angrath's Marauders | `otc` #153 — <https://scryfall.com/card/otc/153/angraths-marauders> | 2026-10-24 | 4 printings, 1 distinct oracle text |
| Wight of Precinct Six | `jmp` #287 — <https://scryfall.com/card/jmp/287/wight-of-precinct-six> | 2026-10-24 | 8 printings, 1 distinct oracle text |

Round-1 review of this lane found two cards whose catalog text had been invented
(Wight of Precinct Six as a layer 7a characteristic-defining ability, Angrath's
Marauders narrowed to "a creature you control"). Both are corrected to the
printed wording above and locked by the provenance tests.

## What is implemented

| Rule | Mechanism | Tests |
| --- | --- | --- |
| CR 613.4c | Layer **7c**: printed count-based statics — `Wight of Precinct Six` is a 1/1 that *gets* +1/+1 for each creature card in its controller's **opponents'** graveyards, recomputed from the game state. A creature card in its controller's own graveyard never counts. | `printed_wight_is_a_one_one_with_empty_graveyards`, `printed_wight_counts_only_opponents_creature_cards` |
| CR 613.4c, 613.7 | Layer **7c**: until-end-of-turn modifiers stored as typed `ContinuousEffect` records with a monotonic timestamp, applied in timestamp order. Both groups are additive, so their relative order cannot change the resulting power/toughness (addition commutes); each group is still iterated in a fixed order so every replay recomputes identically. | `printed_wight_counts_only_opponents_creature_cards`, `until_end_of_turn_effects_end_in_cleanup` |
| CR 514.2 | Cleanup removes every `UntilEndOfTurn` effect and every prevention shield, then recomputes the layers. | `until_end_of_turn_effects_end_in_cleanup` |
| CR 614.1a, 614.5 | Replacement effect: damage from **any source its controller controls** is doubled, applied **once** per damage event. The source may be a creature (combat or activated-ability damage) or a spell on the stack (`Shock`), matching the printed word "source"; damage from a source another player controls is never doubled. | `damage_multiplier_doubles_every_source_its_controller_controls` |
| CR 615.1, 615.2 | Prevention shield with a finite amount (`Mending Hands`, 4 points) that absorbs damage instance by instance and is removed when exhausted. | `prevention_shield_absorbs_its_amount_and_stops_applying` |
| CR 115.4 | "Any target" never includes a spell: `legal_targets` offers only players and creatures for `AnyTarget`, and a cast aimed at a spell is refused with `BadTarget` before any state change, so Mending Hands cannot resolve into an empty shield. | `mending_hands_refuses_a_spell_as_an_any_target` |
| CR 616.1 | Order and chooser: the affected player (or the controller of the affected permanent) chooses the order. The engine acts for that chooser with a documented policy — **prevention before multiplication** — and records the chooser in `DamagePrevented` / `ReplacementApplied`. | `prevention_is_applied_before_the_multiplier` |
| CR 613/614/615 in a real game | Real two-seat `Game::new` run over the new cards that logs `DamageShieldCreated`, `DamagePrevented` and `ContinuousEffectStarted` and replays to the identical digest **and** event trace. | `a_real_game_logs_the_new_effects_and_replays_to_the_same_digest` |

New events: `DamageShieldCreated`, `DamagePrevented`, `ReplacementApplied`,
`ContinuousEffectStarted`. Each renders through `view::render_event` with its
rules citation.

## Deliberate exclusions (disclosed, not approximated)

* Layer **7a** (characteristic-defining power/toughness) is **not** implemented.
  Its only candidate in this pool, Wight of Precinct Six, prints an additive
  "+1/+1 for each ..." static (layer 7c), so the 7a code path was removed instead
  of being kept as untested production code for a card that does not use it. A
  card that really is characteristic-defining is refused at import until a lane
  adds it together with a recorded printing.
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
* The pool stays a bounded practice corpus (79 cards after the issue-17 starter
  package); this is not 100-card
  Commander and not full layer coverage.

## State schema note

`GameState` gained `effects`, `shields`, `next_effect` and `next_timestamp`
(serialized, `#[serde(default)]`). Older recorded replays therefore no longer
reproduce their stored digests; issue-15 evidence is regenerated on this head.
