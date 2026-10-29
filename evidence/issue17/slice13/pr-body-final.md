Draft — **partial issue #17 corpus work, not full acceptance**. Exactly **1 of 8 desired starters is certified**; seven remain planned. This PR does not close #17 or unlock the certified release soak.

## Current scope (slices 12–13)

Six sourced missing cards now have real implementations and focused conformance:

- **Sol Ring, Mind Stone, Hedron Archive:** noncreature artifact spells cast from hand onto the battlefield; immediate tap-mana abilities; Mind Stone/Archive's printed generic-mana + tap + sacrifice costs for stacked draw abilities.
- **Sazh's Chocobo:** controller-only landfall adds typed +1/+1 counters to the same battlefield object.
- **Rampaging Baloths:** printed trample and landfall creating a 4/4 green Beast Token.
- **Professor of Zoomancy:** enters creating a 1/1 black and green Pest Token whose death queues its controller's gain-1-life trigger. The life effect resolves later from the stack.

All printing fields and retrieval dates are recorded in `fixtures/starter-corpus/slice12-sourced.json` and `slice13-sourced.json`; native immutable-printing checks matched. Beast/Pest tokens are engine-only, not importable cards.

**Corpus: 122 catalog cards; 459 desired union names = 110 supported + 349 missing.** All eight desired snapshots remain unchanged. Jasmine is still the only certified 100-card starter. Three target commanders remain unsupported: Ayula, Elas and Zada.

## Runnable evidence and honest limits

- Artifact package: exact four-seat 40-life seed 1201, terminal P3, **1573 accepted actions / 6981 replay events**, digest `d3eade58f133…`. Actual artifact casts 7/8/10, mana activations 53/7/6, sacrifice-draw activations 8/9. This uses a clearly labeled evidence-only masked legal-action demonstrator, **not** an improved product AI.
- Landfall package: exact four-seat 40-life seed 1301, terminal P1, **1568 actions / 5018 replay events**, digest `4cbaa34f0de1…`. Actual new casts 2/3/4, 18 counter events, one Beast and four Pest creations. A Pest dies at the terminal mass-death point; that trace does not demonstrate resolution of its life trigger. A fully drained focused test does.
- Separate release-CLI replays and fresh session-save reload/replays match. Old slice-8 replay retains its digest.
- Both new journeys use explicit **40-card package fixtures**, not desired 100-card starter certificates. Artifact mana requires explicit activation; the existing product heuristic does not cast/use the rocks automatically. Commander's Sphere remains excluded because commander-color mana is unimplemented.

## Repairs and review

Real failures were retained: initially uncastable artifact definitions; invalid token archives in terminal saves; overly permissive archive references; and a too-broad guard that rejected legitimate non-token APNAP source-left provenance. The final guard is token-only and rejects live archival references without changing ordinary token cessation or historical replay digests. Import/UI tests that used now-supported Sol Ring as their unsupported sentinel were refreshed test-only, preserving strong refusal/singleton assertions. No production import/UI/AI behavior changed.

Independent practical reviews: slice12 repair3 approved; slice13 **APPROVED_INCREMENT** at code head `1d99b74371f06bdb645a3d4494ddb48e02d81e14`, including the APNAP compatibility repair. Latest `a4db05f01adcfb2cc8f677d8a33390cd9bfb35e9` changes only current documentation and review receipts. Prior approvals are preserved; no repeated historical audit.

Focused changed-behavior tests, fmt, warnings-denied Clippy and release build passed. **Exact-final-head CI: 10/10 SUCCESS at `a4db05f01adcfb2cc8f677d8a33390cd9bfb35e9`; configured required checks pass (2026-10-26T18:47Z).** The previous failing heads are superseded, not hidden.

## Open release gaps

349 desired names remain missing, with seven whole starter lists unimplemented/uncertified. Legend/equipment/Aura/tutor/reanimation and other printed interactions remain explicit gaps; Friendly Teddy and Aetherize are still excluded. Bounded 40-life commander-combat practice is not the full Commander format. No arbitrary Archidekt compatibility, measured playing strength, native Commander release or v1 acceptance is claimed.

No merge, master push, force push, issue closure or certified downstream activation by this worker.
