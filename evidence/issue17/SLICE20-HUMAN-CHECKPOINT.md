# Issue17 — slice20 human checkpoint

- Branch `aperture/issue-17`, base `7dc933a` (slice19). One local commit for this slice
  follows; see final receipt for exact head.
- **Full #17 unfinished; 1 of 8 certified.** No push/merge/closure/PR creation.

## Implemented
**Vivien's Grizzly** `{2}{G}`, Creature — Bear Spirit 2/3, exact printed activated ability
`{3}{G}`. Source: Scryfall `37f2571b-6756-462c-9e71-3121fa458160`, WAR 182, retrieved
`2026-10-27T03:25:27.464708+00:00`; raw and normalized fixtures under
`evidence/issue17/slice20/source-raw.json` and
`fixtures/starter-corpus/slice20-sourced.json`.

Reusable primitive: anchored top-card look. The card stays in its library list; no
temporary removed holding and no new GameState field. A creature/planeswalker card gets a
private take-or-bottom pending; a nonqualifying card is bottomed without reveal; empty
library does not draw or lose. Take reveals publicly only then. The pending is serialized
and validated as the exact eligible top card. Scry/surveil and optional triggers are
unchanged.

## RED retained
1. Unsupported prior to implementation: `cargo test -p aperture-core --test issue17_slice20`
   failed `vivien_grizzly_is_sourced_and_supported` before the card existed.
2. Native two-activation probe RED at `cb7a18a`:
   `evidence/issue17/slice20/native-two-activations-red.log` — a source with two legal
   activated abilities was rejected by the old `stacked <= 1` zone-container guard.
3. The ineligible-card test originally encoded auto-bottom with no owner look; it was
   strengthened to require a private mandatory-bottom acknowledgement.
4. The take test's `library.first() == library[0]` tautology was removed; exact remaining
   order, hand zone/ownership and `AbilityResolved` are asserted.

## Focused green (this run)
- issue17_slice20: 9 passed (real offered cast/activation, take/reveal/hand, decline/bottom
  without reveal, ineligible private mandatory bottom, empty library no draw/loss, wrong
  seat/action untouched, save/reload midpending parity, two stacked activations saved at
  the top pending and both resolved exactly once, insufficient-mana refusal).
- Repair: stack-container validation now counts only actual spell containers. Ability
  source references may number zero or more and are not zone containers; malformed spell
  containers still fail exactly once. No persistence version change.
- command_zone_setup 44, abilities 13, rules_conformance 5, issue17_slice17 6,
  card_printings 8, catalog_pool 5, slice15 corpus 2, issue13 commander 43.
- fmt --check exit 0; workspace clippy -D warnings exit 0; release CLI build exit 0.

## Journey
Evidence-only masked demonstrator, NOT product AI. `decks/package-slice20.txt`, exact
4 seats / 40 life / seed 1919, Jasmine Boreal injected each command zone. Terminal
`Win{P3}`, **2064 decisions, 46 turns**, snapshot SHA256
`12e6d1e92cdfb56e159e7b96465db2d03a1f511337f2dbca31371521e554916f`; Vivien cast 6,
ability activated 22 times, take 5 / voluntary bottom 5 / mandatory ineligible bottom
acknowledgements 16; 2064 decisions, 46 turns; replay digest matched, fresh save
round-trips. Digest `72cd69fb1143763fcd8fe1387b9a625c896538bdd389f33958d8705d8c7cdd48`.

## Deviations / gaps
- The take/bottom choice reuses `AnswerTriggerChoice` boolean and maps to the existing
  TriggerChoice pending kind; legal offers carry the Vivien-specific labels. No new action
  enum, so no UI action-surface edits.
- A planeswalker card is not in the pool; eligibility uses the printed type-line predicate
  (no fake planeswalker card was added).
- Fresh corpus audit: 137 catalog / union 459 = 125 supported + 334 missing; Ayula 28
  missing; desired snapshots unchanged; 1 of 8 certified.
- No full workspace matrix rerun; exact-head CI owns it. Cargo serialized on this lane.
- Disclosures: `state.rs` gained the ability-source/spell-container validator repair (not a
  new state shape). `crates/ai/tests/common.rs` gained only the new pending match arm; no
  AI policy/search edit. Preserved slice19 disclosure about AI initialization-only changes
  (`search.rs:724`, `common.rs:80`). Look/reveal labels use pinned 701.20a/701.20e.
