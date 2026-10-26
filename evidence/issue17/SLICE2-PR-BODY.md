## Partial #17 corpus implementation — eight-list gate remains OPEN

This draft continues the original starter-corpus work. It does **not** close #17, certify eight lists, approve #16 integration, claim full Commander, or authorize merge/release.

### Usable candidate scope
- Exact Jasmine Boreal starter: 99 library cards + separately specified commander, 41 basics and 58 singleton nonland library cards. Retained independent first-increment evidence includes an exact four-seat terminal game and matching replay/save.
- Seven further sourced cards: **Krenko, Mob Boss**, **Dragon Fodder**, **Krenko's Command**, **Hordeling Outburst**, **Goblin Instigator**, **Beetleback Chief**, and **Talrand, Sky Summoner**. Catalog: 79 → 86 cards for this continuation.
- Typed Goblin production, Krenko's real tap activation and resolution-time Goblin count (including counts above 255), Instigator/Chief ETB triggers, true token zone movement/death before SBA cessation.
- Talrand's controller-only instant/sorcery cast trigger above its spell, real blue 2/2 flying Drake tokens, and enforced flying-block restrictions with illegal-action refusal.
- Engine-only tokens remain unavailable to deck import. These legends are commander-only while the legend rule remains unsupported.

### What is still NOT implemented/certified
- All eight original exact desired lists are preserved unchanged. **One implemented list, seven planned lists, zero certified lists.**
- Exact corpus union: 459 names including commanders/basic lands; **385 names remain missing**. Missing-card sets are checked against the compiled catalog.
- Krenko and Talrand desired 100-card lists remain unsupported, missing 52 and 54 names respectively.
- `decks/package-krenko-tokens.txt` and `decks/package-talrand-drakes.txt` are explicitly **40-card mechanic-package fixtures**, not replacements for those desired Commander lists.
- Other token types, tribal statics, general keywords, remaining commanders/interactions and broad card support are not claimed. Arbitrary Archidekt/Commander compatibility, native starter setup and playing strength remain unclaimed.

### Executed evidence
- Focused new token/Talrand tests, catalog/printing/commander/starter/import regressions, format, warnings-denied Clippy and release CLI build.
- Krenko package: exact four-seat 40-life terminal journey, 1805 actions/40 turns, separate replay/save match.
- Talrand package: exact four-seat 40-life terminal journey, 1830 actions/39 turns, 11 actual Drake creations, separate replay/save match.
- Independent practical review ran additional exact package games/replays and old Jasmine compatibility. Its initial requested changes and subsequent adjudication are preserved in `evidence/issue17/`.
- Exact-head CI and the final independent verdict are recorded separately; do not interpret this draft description as their approval.

### Process and integration caveats
Assignment1 of this continuation was implementation-first, not genuine test-first; that disclosure is retained. Native-found token-count/death corrections and Talrand/flying have genuine behavioral RED/GREEN evidence. No historical process failure is retroactively certified.

Core token/card/effect hooks overlap the separate #16 integration lane. Integration must reconcile both histories and verify the merged delta independently; this work does not approve that lane. No master push, merge, issue closure or successor activation by this worker.
