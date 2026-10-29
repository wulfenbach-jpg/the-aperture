# Ayula package — next-slice work order (sourced 2026-10-27T11:39:32Z)

Sources: one Scryfall `/cards/collection` call for all 22 remaining names, raw response and
stamp retained in `evidence/issue17/slice27/ayula-missing-sources/collection-raw.json` and
`retrieval.json`; normalized per-card rows (set, collector number, Scryfall id, oracle id,
cost, type line, P/T, oracle text) in `cards.json`. No name was missing (`not_found: []`).

State at head `27d8f48`: Ayula is **22 names short** of a 100-card certificate; pool is 143
cards; the union across the eight desired lists is 459 names = 131 supported + 328 missing.

## Recommended order (reuse first, new machinery last)

| # | Card (printing) | Cost / P·T | What it needs | Verdict |
|---|---|---|---|---|
| 1 | **Circle of the Moon Druid** (AFR 177) | `{2}{G}` · 2/4 printed, 4/2 on your turn | A **turn-conditioned static** that reuses two layers already implemented: `Layer::Type` (subtype line → Bear, losing the printed Human Elf Druid line) and a conditional `Layer::Setting` base P/T 4/2. Needs a static-ability predicate "during your (controller's) turn" and careful interaction with Ayula's Bear counting only on your turn. | **Implementable now** — smallest genuine step, exercises the layer pipeline instead of a new one. |
| 2 | Stuffed Bear (MID 259) | `{2}` artifact | Animation: layer 4 must **add** the creature type to an artifact (today `Layer::Type` only *replaces* a creature subtype line) plus a layer 5 **colour** set to green and a 7b base P/T 4/4 with cleanup expiry. | Needs two new primitives (additive type change, colour layer). Second target. |
| 3 | Drover Grizzly (OTJ 161) | `{2}{G}` · 4/2 Bear Mount | `Layer::Ability` trample grant already exists; the trigger is "attacks while saddled". Needs the **saddle** activated ability (tap any number of other creatures with total power ≥ 1, sorcery timing, saddled-until-EOT state). | Feasible once saddle exists; medium. |
| 4 | Kamahl's Summons (ONS 269) | `{3}{G}` sorcery | "Each player may reveal any number of creature cards from their hand; then each creates a 2/2 green Bear token for each revealed." Reveal-from-hand and Bear-token creation both exist; the new part is a **set choice over an arbitrary hand subset** and APNAP per-player resolution. | Feasible; the choice surface is the work. |
| 5 | The Earth King (TLA 172) | `{3}{G}` · 2/2 | Enters → create a **4/4 green Bear token** (new token def; today only 2/2 Bear and 1/1 Goblin exist). The attack trigger searches basic lands — **blocked**: library search is unimplemented. | Half-doable; the search half must stay unsupported, so the card stays out of the pool until search lands. |
| 6 | Rampaging Yao Guai (PIP 82) | `{X}{G}{G}{G}` · 2/2 | Enters with X +1/+1 counters; enters → destroy any number of target artifacts/enchantments with total mana value ≤ X. Needs X-costs and a **total-MV-bounded multi-target** choice. | Medium-hard. |
| 7 | Ruxa, Patient Professor (C21 66) | `{2}{G}{G}` · 4/4 | "no abilities" predicate across the pool, +1/+1 static (7c exists), graveyard→hand recursion, and "assign combat damage as though not blocked" (needs the damage-assignment narrowing this engine deliberately does not expose). | Blocked by the documented combat-assignment limitation. |
| 8 | Beorn the Fierce (HOB 119) | `{3}{G}{G}` · 6/6 | Trample (printed), Bear anthem (7c), trample counter + "becomes a Bear in addition" (additive type change), conditional draw two. | Needs the additive type change from #2 plus a counter-kind extension. |
| 9 | Beorn's Hospitality (HOB 120) | `{1}{G}` enchantment | Landfall counter (exists) plus a permanent animation with a characteristic-defining P/T equal to lands. | Needs CDAs (7a), deliberately absent. |
| 10 | Druid's Familiar (AVR 175) | `{3}{G}` · 2/2 | **Soulbond** pairing state machine. | New subsystem. |
| 11 | Lumra, Bellow of the Woods (BLB 183) | `{4}{G}{G}` · \*/\* | Reach/vigilance, CDA P/T = lands (7a), mill four (exists), mass return of lands from graveyard. | Needs 7a + mass reanimation. |
| 12 | Owlbear Cub (CLB 246) | `{2}{G}` · 3/3 | Conditional top-8 look with a free creature put onto the battlefield tapped and attacking. | Needs a bounded look-and-deploy (bigger than Vivien's reveal). |
| 13 | Evercoat Ursine (BLC 30) | `{4}{G}` · 6/5 | **Hideaway** (exile face down, later play without paying), twice. | New zone/permission subsystem. |
| 14 | Spirit of the Aldergard (KHM 195) | `{3}{G}` · 0/4 | Enters → **search library for a snow land**; +1/+0 per other snow permanent. | Blocked: library search unimplemented. |
| 15 | Titania's Command (BRO 194) | `{4}{G}{G}` | "Choose two" modal with four modes (graveyard exile + life, land search, two Bears, two counters each). | Blocked by search + modal choice. |
| 16 | Words of Wilding (ONS 305) | `{2}{G}` | Draw replacement with a token instead, once per turn. | Needs a draw-replacement hook; medium. |
| 17 | Dúnedain Rangers (LTR 159) | `{3}{G}` · 4/4 | Landfall + **the Ring tempts you**. | Blocked: Ring mechanic absent. |
| 18 | Thijarian Witness (WHO 111) | `{1}{G}` · 0/4 | Flash (exists) + dies-if-alone exile + **investigate** (Clue token). | New token type + alone predicate. |
| 19 | Spider-Ham, Peter Porker (SPM 114) | `{1}{G}` · 2/2 | **Food token** + an 18-subtype anthem. | New token type. |
| 20 | Lightning Greaves (MSC 202) | `{2}` equipment | Equipment + shroud. | Blocked: no equipment subsystem. |
| 21 | Swiftfoot Boots (TRC 501) | `{2}` equipment | Equipment + hexproof. | Blocked: no equipment subsystem. |
| 22 | Ursine Monstrosity (DSC 36) | `{2}{G}` · 3/3 | Trample, mill (exists), random opponent must-be-attacked, indestructible + P/T per card type in graveyard. | Needs forced-attack and indestructible primitives. |

## Slice-28 recommendation

Take **#1 Circle of the Moon Druid** first: it is the only remaining Ayula card whose printed
text is fully expressible with primitives this branch already ships (`Layer::Type`,
`Layer::Setting`, `Layer::Modifying`), and the new surface is one typed static predicate
("during your turn") rather than a new subsystem. Keep its genuine RED first (the card is
currently refused as unsupported), then prove the turn-conditional 4/2 / Bear behaviour
against Ayula's fight revalidation and Little Bear's counter branch, including the
end-of-your-turn reversion, and re-run the four-seat journal + separate replay as usual.

Do **not** claim Ayula certification from this: even after #1 the list is still 21 names
short, and the eight-list gate remains 1 of 8.
