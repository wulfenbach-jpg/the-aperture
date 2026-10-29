# Ruxa prerequisite verification — every anchor in `slice51/NEXT-CARDS.md` re-read at `6abd820`

**Status: verification of a plan, not card support.** No card is enabled by this file, the corpus is
unchanged at **161 catalog cards / 459 union = 148 supported + 311 missing**, Ayula is still **5**
missing, and **1 of 8** lists is certified. Ruxa, Patient Professor remains in
`missing_cards` and no engine surface for it was written in slice 52.

Why this file exists: review 335 found one wrong anchor in the slice-51 brief (`describe_target` was
attributed to `view.rs`; it is `engine.rs:9022`). A brief with a wrong anchor becomes the next run's
false premise, so every anchor, function name and line number below was re-read in this checkout at
head `6abd820` before being repeated. Where the slice-51 map was wrong or imprecise, the correction is
recorded explicitly instead of silently rewritten.

## Corrections to the slice-51 map (all four verified by reading the code at this head)

| Claim in `slice51/NEXT-CARDS.md` | Verified reality at `6abd820` |
|---|---|
| `describe_target` lives in `view.rs` | `Game::describe_target` is `crates/core/src/engine.rs:9022`; `view.rs` has no such function (review 335 non-blocking note; corrected in the map by slice 52) |
| AI mapping at `crates/ai/src/search.rs` `~760-800` | the pending projection enum is `PendingKind` in `crates/core/src/view.rs`, and the only AI-side consumer is `pending_kind_name` at `crates/ai/src/search.rs:345-365` (declaration copy, no policy) |
| commander destination prediction at `state.rs` `~2078` | `Pending::CommanderReplacement` validation is `state.rs:2026-2050`; the dispatch/answer path starts at `state.rs:1108`; the ReturnToHand destination arm that `~2078` meant is `state.rs:2078`. Correction (review 337 R337-1): the move-to-graveyard *insertion* is **not** in `state.rs` at all — it is `engine.rs:2077` |
| `deal_combat_damage` at `~8827` | `fn deal_combat_damage` is `engine.rs:8832`; the only caller is the `Step::CombatDamage` arm of `advance_step` at `engine.rs:5955-5960` |
| `abilities()` table at `~1043` | `pub fn abilities(card)` is `ability.rs:1066`; `static_abilities(card)` is `ability.rs:1548`; `pub enum StaticAbility` is `ability.rs:1213` (these three were right to within a dozen lines) |

## Claim-by-claim verification

### Clause 1 — "Whenever Ruxa enters or attacks, return target creature card with no abilities from your graveyard to your hand."

* **No such primitive exists today.** `grep -rn` for a graveyard-to-hand effect, `GraveyardCard`,
  `ReturnFromGraveyard` and `graveyard_to_hand` across `crates/core/src/` returns nothing; the only
  **effect-driven** zone mover that touches a hand is `fn move_to_hand` at `engine.rs:1971`. So clause 1
  is new engine surface, exactly as the map says — it is not data authoring.
  Correction (review 337 R337-1): `move_to_hand` is *not* the only code that writes a hand list. The
  draw path inserts into hand directly at `engine.rs:2301` (it emits `Event::Draw` and set
  `drew_from_empty`), and the commander replacement intercepts a hand move at `engine.rs:7302`. A
  graveyard-to-hand effect must therefore go through `move_to_hand`, never through the draw path —
  routing a returned card through the draw path would fire draw triggers and set `drew_from_empty`.
* **Graveyard cards are real objects with printed characteristics.** A card put into a graveyard is
  inserted as an `Object` in `Zone::Graveyard` (`engine.rs:2077`, `engine.rs:2339`, `engine.rs:5683`,
  `engine.rs:5702`, `engine.rs:10296` — correction from review 337 R337-1, which found the earlier
  `state.rs:1774/2077/2339` citations were the `SearchLibrary` validator, the `CommanderReplacement`
  ReturnToHand destination arm and the `OrderCard` validator, not insertions) and
  `Game::object`/`try_object` (`state.rs:2863`, `state.rs:2869`) resolve it. So a graveyard-card target
  can be checked against the *printed* ability list and intrinsic mana ability, which is what CR 113.6
  and 400.7 require.
* **`Target` really has three variants** (`Player`, `Permanent`, `Spell`) at `action.rs:7-13`, and
  `Action::ChooseTriggerTarget { target }` (`action.rs:130-132`) already carries a `Target`, so the
  choice surface needs a new variant, not a new action.
* **`legal_targets` is `engine.rs:3158`** and is a single `match spec` over `TargetSpec`; the
  graveyard arm belongs beside the `TargetSpec::BearYouControl`/`CreatureYouControl` arms.
* **`Effect::ReturnToHand`'s dispatch arm is `engine.rs:7299`** — the new effect's dispatch belongs
  there, and `Effect` lives at `card.rs:126`.
* **The commander path is not optional.** `Pending::CommanderReplacement` is validated against the
  producing effect at `state.rs:2026-2050` and answered at `state.rs:1108`; a graveyard-to-hand effect
  must be added to that prediction or CR 903.9b can be bypassed. This is the same defect class review
  329 filed as R329-3 (a pending that bypasses a saved-instruction guard).

### Clause 2 — "Creatures you control with no abilities get +1/+1."

* `StaticAbility::OtherBearsYouControlGet` (`ability.rs:1275`) is the closest existing static and is
  computed in the pump block around `engine.rs:678`, inside the region the map cites as `618-790`.
  Correction (review 337 R337-1): it is **not** the only layer 7c additive statics —
  `OtherControlledCreaturesWithAnySubtypeGet` (`ability.rs:1287`), `OtherControlledCreaturesHaveTrample`
  (`ability.rs:1249`) and `PairedEachGet` (`ability.rs:1299`) are also 7c statics. The conclusion stands
  (a "no abilities" anthem is a new variant, not a reuse of the Bear one), but the *plumbing* to copy is
  `OtherControlledCreaturesWithAnySubtypeGet`'s recipient-set loop, not the Bear one's subtype filter.
* The predicate the map asks for ("printed abilities empty **and** no intrinsic mana ability **and**
  no granted ability") is the part that must be written once and shared with clause 3. The engine
  already distinguishes the three sources: `ability::abilities(card)` (`ability.rs:1066`),
  `CardDef::basic_mana` / `artifact_mana` (`card.rs:628-630`), and granted abilities via
  `equipment_grants` (`ability.rs:460`) plus static grants. No single existing predicate combines
  them, so the map's instruction to define one is correct and still unbuilt.

### Clause 3 — "For each creature you control with no abilities, you may have that creature assign its combat damage as though it weren't blocked."

* **`deal_combat_damage` (`engine.rs:8832`) has no choice surface at all**, as the map says: a blocked
  attacker splits lethal-first, trample spills to the player (`engine.rs:8865-8901`), and the only
  early return is the `combat_damage_done` guard (`engine.rs:8833`). The step is entered from
  `advance_step`'s `Step::CombatDamage` arm (`engine.rs:5955`), which deals damage, runs SBAs, and
  grants priority — so a pending created before the damage call must return without dealing damage and
  the answer path must deal it, then run SBAs and grant priority exactly once.
* **The `Pending` enum is `state.rs:106`** with per-variant validation immediately after each arm's
  match in `GameState::validate`, and `PendingKind` in `view.rs` is the only projection the AI sees.
  Both must gain the new variant; the AI change is a declaration copy, not a policy change, and must be
  disclosed as slice 50 disclosed its one-field exception.

## What this means for the slice that builds Ruxa

1. The work is four coupled surfaces (target/target-spec/effect, static anthem, damage-assignment
   pending, and the saved-state validators for all of them), not a card-table entry. It is a
   multi-session card; slicing it as "clause 2 first, then 1, then 3" would ship unreachable code with
   no card to exercise it, so the card should be enabled only when all three clauses have behaviour
   tests through real actions.
2. The two highest-risk validators are the ones reviews 327/329 already burned: a forged pending
   (R329-3) and a forged target list that passes `GameState::validate` but not a real
   `SessionSave::load`. Both need refusals pinned by tests, not prose.
3. Any digest change must be stated as review 327 R327-1 required: recording new state changes the
   canonical digest of states that carry it, and older saves keep loading only because
   `SAVE_VERSION` 10 / `REPLAY_VERSION` 6 stay put.
