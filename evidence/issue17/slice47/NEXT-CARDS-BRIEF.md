# Ayula's six remaining names — implementation brief for the next run

Written by the slice-47 run **before** any of these cards is implemented, so the next run starts
from real printed text and a primitive inventory instead of re-deriving both. Nothing here is a
claim that any of it works: the pool is still `160` cards, the eight-list gate is still **1 of 8**,
and Ayula still has these **6** names in `missing_cards` (matching 6 whole-field
`missing_interactions` entries).

Authoritative text for all six is preserved from live Scryfall with printing id, oracle id, HTTP
request, retrieval stamp and `sha256`:

* `evidence/issue17/slice46/sources/<card>.json` (**raw** response bytes) and
  `<card>.stamp.json` (extracted fields + `retrieved_utc`), fetched 2026-10-28T13:49:35Z by the
  previous run through `https://api.scryfall.com/cards/named?fuzzy=<name>`.
* Filenames are lowercased with the original accents (`dúnedain-rangers.json`).

| Card | Printing | Cost | Type | P/T |
| --- | --- | --- | --- | --- |
| Dúnedain Rangers | ltr 159 / `630e1e36-2f5d-44d4-9ff2-19ae75295016` | `{3}{G}` | Creature — Human Ranger | 4/4 |
| Evercoat Ursine | blc 30 / `b299d768-b171-47e5-b69d-82ce13917c43` | `{4}{G}` | Creature — Elemental Bear | 6/5 |
| Owlbear Cub | clb 246 / `be177b66-dab3-47ef-bcdf-8d99eb57e19f` | `{2}{G}` | Creature — Bird Bear | 3/3 |
| Rampaging Yao Guai | pip 82 / `63619236-a27a-4fdd-add6-7a295901f906` | `{X}{G}{G}{G}` | Creature — Bear Mutant | 2/2 |
| Ruxa, Patient Professor | c21 66 / `e07b8142-6a49-46e7-b862-41f89a59b894` | `{2}{G}{G}` | Legendary Creature — Bear Druid | 4/4 |
| Ursine Monstrosity | dsc 36 / `73cc6df4-3564-4ace-bf8a-eac3e62d725a` | `{2}{G}` | Creature — Bear Mutant | 3/3 |

## Primitive inventory (checked in this checkout, not assumed)

Present today and reusable: typed triggers (`Attacks`, `AnotherCreatureDies`, landfall), the
private incremental `Pending::SearchLibrary` machinery with modes and a chosen list, self-mill and
each-opponent mill, +1/+1 counters that really change P/T (`Object::counters` feeds
`power()`/`toughness()`), vigilance/trample/flash/reach/shroud/hexproof as typed keyword flags,
typed layer-7c additive statics, token creation, and the new CR 506.5 declaration record.

**Absent today** (each is the actual blocker, verified by grep — none of these words exists in
`crates/core/src`):

| Primitive | Needed by | Note |
| --- | --- | --- |
| "the Ring tempts you" + Ring-bearer designation + Ring levels | Dúnedain Rangers | Whole external mechanic with an emblem-like designation; nothing of it exists |
| Face-down exiled cards attached to a specific permanent ("exiled with it") | Evercoat Ursine | No face-down zone object, no per-permanent exile link; hideaway twice compounds it |
| "play a card without paying its mana cost" from exile | Evercoat Ursine | No play-from-exile permission path at all |
| X on a **creature** spell (`{X}{G}{G}{G}`) | Rampaging Yao Guai | `Action::CastSpell` carries no X; `cost_mana_value` documents "no X-cost spell in the pool is a creature" |
| "destroy any number of targets with total mana value ≤ X" | Rampaging Yao Guai | No multi-target spell selection with a sum cap |
| "creature card with no abilities" predicate | Ruxa | No predicate; needs a definition over printed keywords **and** rules text |
| Combat damage "as though it weren't blocked" | Ruxa | Damage assignment is automatic lethal-first; no per-creature assignment choice |
| Indestructible | Ursine Monstrosity | No keyword; interacts with destruction SBAs and `try_regenerate` |
| "choose an opponent at random" | Ursine Monstrosity | No random-choice helper on the engine RNG |
| "attacks that player this combat if able" | Ursine Monstrosity | Declaration is free-form today; no forced-attacker rule or per-attacker defender |
| Put a creature from the top of a library onto the battlefield **tapped and attacking** | Owlbear Cub | Search machinery can put a land onto the battlefield, never a creature, and never "attacking" |

## Recommended order (cheapest genuine card first)

1. **Owlbear Cub** — the smallest real increment: one `Attacks` trigger with an intervening-if on
   the defending seat's land count, one private look at the top eight, one optional
   creature-to-battlefield choice, bottom-shuffle. Reuses `Pending::SearchLibrary` if a new
   destination mode (`CreatureFromTopToBattlefieldTappedAttacking`) is added, and needs the engine
   to append the new object to `turn.attackers` (and to `turn.declared_attackers`, which is now
   the honest place to record it: it did not *attack* per CR 508.1, but it is attacking).
   Tests that must fail first: defender with 7 lands does nothing; 8 lands offers the dig; the
   rest go to the bottom in a random order; the put creature is tapped, attacking and deals damage
   in that combat; no summoning-sickness interaction blocks the damage.
2. **Ruxa, Patient Professor** — "no abilities" is the whole card; define it once (printed
   keywords + non-empty rules text) and reuse for the +1/+1 static and the damage-as-unblocked
   permission. The damage clause is the expensive part; if assignment cannot be done honestly in
   one slice, split the card and say so rather than shipping the static alone.
3. **Rampaging Yao Guai** — X on creatures is an `Action`/validation change that touches AI, UI and
   replay, so it should be a deliberate, separately reviewed increment, not a side effect of a
   card. The destroy mode also needs a multi-target sum cap.
4. **Ursine Monstrosity** — mill (exists) + random opponent (new) + forced attack (new) +
   indestructible (new) + dynamic P/T by card types in graveyard (new). Four new primitives in one
   card: split it.
5. **Evercoat Ursine** — face-down exile linked to a permanent, twice, plus play-without-paying.
6. **Dúnedain Rangers** — the Ring is a mechanic, not a card. This is the one to schedule last and
   to treat as its own issue, not as "one more Ayula name".

Nothing in this brief authorizes implementations to be merged half-built: every card must land
with its own sourced fixture, printed-text assertions, discriminating behaviour tests, and the
whole-field `missing_cards`/`missing_interactions` rewrite at the moment it leaves the gap list.
