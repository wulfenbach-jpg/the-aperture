# Next card: Ruxa, Patient Professor — verified integration map

No card was enabled in slice 51. Corpus remains **161** cards / **459** union = 148 supported +
311 missing; **1 of 8** certified, Ayula **5** missing (Dúnedain Rangers, Evercoat Ursine,
Rampaging Yao Guai, Ruxa Patient Professor, Ursine Monstrosity). The next implementation is
**Ruxa, Patient Professor** (C21 66; printing `e07b8142-6a49-46e7-b862-41f89a59b894`; oracle
`d760f2f0-6653-47c4-bd7b-aab2f398ddf2`; source stamp `slice46/sources/ruxa-patient-professor.stamp.json`,
retrieved 2026-10-28T13:49:35Z). Re-fetch the exact printing; do not paraphrase from memory.

Printed text (verbatim, three separate abilities):

```
Whenever Ruxa enters or attacks, return target creature card with no abilities from your graveyard to your hand.
Creatures you control with no abilities get +1/+1.
For each creature you control with no abilities, you may have that creature assign its combat damage as though it weren't blocked.
```

All three clauses must work before the card may leave `missing_cards`. Nothing here is
data-authoring: each clause needs the listed engine surface. The file/function anchors below were
re-read at slice-51 head, so the next run starts from them instead of re-deriving.

## Clause 1 — enters-or-attacks return from the graveyard

* `crates/core/src/action.rs` — `Target` currently has `Player` / `Permanent` / `Spell` only. A
  graveyard *card* is none of these (CR 108.1: a card in a graveyard is a card, not a permanent), so
  add a `Card(ObjectId)` variant; `Action::ChooseTriggerTarget { target }` then carries it without a
  new action.
* `crates/core/src/card.rs` — new `TargetSpec` for "target creature card with no abilities in **your**
  graveyard". Graveyard cards keep their printed characteristics, so the no-abilities test there may
  use the printed ability list and printed mana abilities; battlefield grants no longer apply
  (CR 113.6, 400.7).
* `crates/core/src/engine.rs` — `legal_targets` (~3158) needs the matching arm: `zone == Graveyard`,
  `owner == caster`, printed creature, printed ability list empty (`ability::abilities(card).is_empty()`)
  **and** no intrinsic mana ability (`CardDef::basic_mana` / `artifact_mana`), because those are real
  abilities (CR 605.1a).
* `crates/core/src/card.rs` + `engine.rs` — new `Effect` (graveyard card → its owner's hand) and its
  dispatch arm next to `Effect::ReturnToHand` (~7299). Two rules obligations:
  * CR 400.7: the card that arrives in hand is a **new object**; nothing may keep the old id.
  * CR 903.9b: a commander returned from a graveyard to hand is replaced into the command zone — that
    is why `state.rs` (~2078) maps producer effects to a predicted destination for the
    `CommanderReplacement` validation; the new effect must be added there or a commander can slip
    through the replacement.
* `crates/core/src/ability.rs` — two triggered abilities (`EntersBattlefield`, `Attacks`) in the
  `abilities()` table (~1043). Zero legal targets removes the trigger (CR 603.3d); an eligible target
  is **not** optional, so do not model it as "up to one".
* `crates/core/src/catalog.rs` — the `card!` entry, `crates/core/src/card.rs` — `CardId`, and
  `crates/core/tests/card_printings.rs` — the verbatim printing record with the source stamp.

## Clause 2 — the no-abilities anthem

* `crates/core/src/ability.rs` — `StaticAbility` (~1213) plus `static_abilities()` (~1548): a new
  layer 7c additive variant for "creatures **you** control with no abilities get +1/+1" (it includes
  Ruxa only if Ruxa somehow has no abilities — it has three printed abilities, so it never pumps
  itself, and the printed word is not "other").
* `crates/core/src/engine.rs` (~618–790) — the existing static pump computation; the new arm belongs
  beside `OtherBearsYouControlGet`. It needs **one** effective battlefield predicate:
  * no printed abilities (the `abilities()` table is empty **and** no intrinsic mana ability), and
  * no granted ability from equipment/statics (haste/shroud/hexproof/trample are granted through
    `EquipmentGrants` / static arms — a equipped creature with no printed abilities therefore has
    abilities and must **not** get the bonus), and
  * an animated noncreature permanent counts by its effective types but still carries its own
    printed/activated abilities, so it usually fails the test — that is correct, not a bug.
  Document the predicate once and reuse it for clause 3, so the two clauses cannot drift.

## Clause 3 — the optional "as though it weren't blocked" damage assignment

* `crates/core/src/state.rs` — a typed `Pending` (enum at ~106, validation block follows each variant)
  asked of the attacking creature's controller at the start of the combat damage step, carrying the
  ordered list of qualifying creatures still to answer and the answers so far. It must be bounded:
  a forged list (creature not attacking, not controlled by the answering seat, already answered,
  duplicated, or a non-creature) is refused by `GameState::validate` **and** by a real
  `SessionSave::load` — that is exactly the review-329 R329-3 class of defect.
* `crates/core/src/action.rs` — one answer action; `crates/core/src/view.rs` — the pending must be
  projected to its owner only (contents masked for every other seat), and `Game::describe_target`
  (`crates/core/src/engine.rs:9022`, **not** `view.rs` as an earlier revision of this brief said)
  needs the new `Target::Card` case for the stack view.
* `crates/core/src/engine.rs` — `deal_combat_damage` (~8827) currently assigns automatically: a
  blocked attacker with blockers splits its damage lethal-first, trample spills to the player, and
  there is no choice surface. A creature answered "yes" must assign its damage **to the defending
  player as though unblocked** while remaining blocked for every other rule (CR 509.1h/510.1c: it is
  still blocked, its blockers still deal damage to it, and blocking-triggered abilities still see it
  blocked). Do not model the choice as trample and do not mark it unblocked.
* `crates/ai/src/search.rs` (~760–800) — `PendingKind` mapping needs the new variant; that mapping is
  the only AI change allowed and is a declaration copy, not a policy, budget or sampling change.
  Disclose it as a narrow exception as slice 50 did.

## Discipline for the slice that lands it

Whole-field rewrites each time a card leaves `missing_cards`: update **both** `missing_cards` **and**
the entire `missing_interactions` list in `decks/starter-corpus/manifest.json` (review 314/316/323
rule), keep the corpus doc's current-status lines consistent, keep the frozen counts honest, retain a
compiling pre-repair RED per behaviour, run one real four-seat journey with a **separate** CLI replay,
plus fmt / Clippy `-D warnings` / locked release / workspace tests / exact-head CI on the **code**
head, and state the evidence tip apart with no CI claim for it. Original #17 stays unfinished until
all eight lists are genuinely certified; do not merge, push master, close the issue or unlock
dependents.
