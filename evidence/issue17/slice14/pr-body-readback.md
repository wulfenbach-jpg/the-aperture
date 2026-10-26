Draft — **partial issue #17 corpus work, not full acceptance**. Exactly **1 of 8 desired starters is certified**; seven remain planned. This PR does not close #17 or unlock the certified release soak.

## Current scope (slice 14)

Four sourced cards now have real implementations and focused conformance:

- **Arcane Signet** `{2}` — `{T}: Add one mana of any color in your commander's color identity.`
- **Fellwar Stone** `{2}` — `{T}: Add one mana of any color that a land an opponent controls could produce.`
- **Commander's Sphere** `{3}` — the same commander-identity mana plus its printed `Sacrifice this artifact: Draw a card.`, usable while tapped.
- **Werebear** `{1}{G}` 1/1 Human Bear Druid — real `{T}: Add {G}` with summoning sickness and a **threshold** layer 7c static (+3/+3 at seven or more cards in *your* graveyard, any card type, both directions, stacking with counters and pumps).

Dynamic colors are explicit `ActivateColorManaAbility` legal actions in stable WUBRG order. Forged colors, wrong seats/zones and pending-choice timing are refused before any tap; a refused action leaves the state digest unchanged. Tapped or summoning-sick sources are never offered by `legal_actions`.

Printing fields, immutable printing URLs and retrieval timestamps are recorded in `fixtures/starter-corpus/slice14-sourced.json`; all four were re-read live during this slice.

**Corpus: 126 catalog cards; 459 desired union names = 114 supported + 345 missing.** All eight desired snapshots remain unchanged. Jasmine is still the only certified 100-card starter. Nearest remaining gaps: Ayula 39, Krenko 48, Talrand 51, Elas 52, Tatyova 52, Zada 53, Isamaru 55.

## Runnable evidence and honest limits

Evidence-only masked legal-action demonstrator (explicitly **not** an improved product AI), exact four seats / 40 life / `decks/package-slice14.txt` / seed 1414: terminal P0, **2371 accepted actions, 42 turns, 6665 replay events**, digest `028cc2e33168…`. Actual casts: Arcane Signet 7, Fellwar Stone 7, Commander's Sphere 7, Werebear 18. Mana activations: 51 / 39 / 7 / 82; Sphere sacrifice-draws 7. A separate release-CLI replay and a fresh session-save reload both reproduce the same digest. The default product heuristic trace is retained unchanged at seed 1401 (terminal, digest `fdc9b089f8f9…`, 17 Werebear casts, zero artifact casts or activations): artifacts and nonland mana still require explicit activation, and no AI policy was changed. Both journeys use **40-card package fixtures**, not starter certificates.

## Repairs and review

Two real defects in the first cut were caught by native inspection and repaired with genuine RED/GREEN in this history:

1. `legal_actions` offered colors for **tapped** dynamic artifacts that `validate` then refused;
2. a ready Werebear was **never offered** because `mana_sources` omitted creature mana — direct-submit tests alone had passed.

The evidence driver was not allowed to bypass legal offers; it selects only projected legal actions and exercises all four cards.

Independent same-history delta reviews: **APPROVED_INCREMENT at `472aeba`** (zero blocking findings) and **APPROVED_INCREMENT at `eb516e4`** for the behaviour-neutral cleanup delta (removal of two dead `CardDef` fields the first review flagged, plus corrected counts). Reports: `evidence/issue17/slice14/REVIEW14.md` and `REVIEW14-CLEANUP.md`.

Focused changed-behaviour tests, fmt, warnings-denied Clippy and locked release build passed. **Exact-final-head CI: 10/10 SUCCESS at `c872a82d277353884840785be74417eb11f71d16`; configured required checks pass (2026-10-26T20:00Z).** Failed heads and their findings are retained, not hidden.

## Open release gaps

345 desired names remain missing, with seven whole starter lists unimplemented/uncertified. Legend rule, equipment/Auras, tutors, reanimation, multi-mode text and most tribal payoffs remain explicit gaps; Friendly Teddy and Aetherize are still excluded with recorded reasons. Bounded 40-life commander-combat practice is not the full Commander format. No arbitrary Archidekt compatibility, measured playing strength, native Commander release or v1 acceptance is claimed.

No merge, master push, force push, issue closure or certified downstream activation by this worker.

