## Partial #17 corpus implementation — eight-list gate remains OPEN

This draft continues the original starter-corpus work through **slice 5**. It does
**not** close #17, certify eight lists, approve #16 integration, claim full
Commander, or authorize merge/release.

### Current corpus state (recomputed natively this slice)

* supported catalog: **103** real cards (`cargo`/`catalog::pool_size()`);
* exact eight-list union: **459** names including commanders and basic lands;
* **91 supported / 368 missing**;
* **1 implemented list (Jasmine vanilla stompy), 0 certified**, seven planned.
* Evidence: `evidence/issue17/slice5/update_manifest.py` →
  `slice5-corpus-state.json`. Desired list files are never edited.

### What is newly usable in slice 5 (all sourced from live Scryfall this run)

`fixtures/starter-corpus/slice5-sourced.json` records each printing, the full
distinct-text list and the retrieval timestamp.

| card | printing | behaviour |
|---|---|---|
| Stitcher's Supplier | tdc 196 | mill three on entering **and** on dying |
| Ruin Crab | znr 75 | landfall: each opponent mills three |
| Negate | tmt 47 | counter target **noncreature** spell |
| Preordain | tdc 161 | scry 2, then draw a card |
| Pyretic Ritual | m11 153 | add {R}{R}{R} |
| Seething Song | c21 179 | add {R}{R}{R}{R}{R} |

New rules primitives: `Effect::MillSelf` / `Effect::MillEachOpponent` (real
library→graveyard move, one `Milled` event per card, no "dies" trigger, no loss on
an empty library), `TargetSpec::NoncreatureSpell` (creature spells are never
offered and a forged target is refused with `BadTarget`), `Effect::AddMana` (mana
to the pool without tapping, CR 106.4). Each is covered by focused tests in
`crates/core/tests/issue17_slice5.rs`.

Earlier slices on this branch remain as described by their own evidence: the
exact Jasmine starter, the Krenko goblin-token package, Talrand + Drake tokens,
Tatyova/Jaddi landfall, the cross-corpus package (Swamp, Lightning Bolt, Forest
Bear, Striped Bears, Guttersnipe) and slice 4 (Ordinary Bear, Ashcoat Bear with
printed Flash, Owlbear with printed Trample, Soul Warden).

### What is still NOT implemented or certified

* Seven of the eight desired 100-card lists stay planned; missing counts after
  this slice: isamaru 56, ayula 47, talrand 52, elas 57, tatyova 55, zada 55,
  krenko 51.
* **Target-player milling is not implemented** (Hedron Crab is still refused at
  import). Nothing reads the graveyard: no graveyard watchers, threshold,
  reanimation or delve.
* Counters as rules objects, the legend rule, artifacts/equipment, tutoring,
  spell copying, sacrifice outlets and most keywords remain unimplemented.
* `decks/package-*.txt` files (including the new `package-slice5-mill.txt`, exactly
  40 cards) are clearly labelled **mechanic-package fixtures**, never stand-ins for
  the desired 100-card starters.
* Arbitrary Archidekt/Commander compatibility, native starter setup and playing
  strength remain unclaimed.

### Executed evidence for this slice

* focused `issue17_slice5` 10/10; workspace `cargo test --workspace --locked`
  526 passed / 0 failed over 82 suites; `fmt`, `clippy -D warnings` and locked
  release build clean.
* two genuine RED runs: engine hooks disabled (five behaviour tests fail), and
  Preordain's effects swapped (the strengthened ordering test fails). Both logs
  are in `evidence/issue17/slice5/`.
* real executable four-seat 40-life commander-combat-practice journey on
  `decks/package-slice5-mill.txt`, seed 501: terminal, winner P2, 1880 steps /
  45 turns, digest `d0c888f680d7`, 111 mill lines, separate `replay --file`
  verification matching (1880 actions, 4654 events). One seat is eliminated by
  being milled out — the new primitive changes real outcomes.
* exact-head CI 10/10 SUCCESS (`slice5-final-ci-check-runs.json`).
* independent practical review, three rounds: approve on `eb0a6f5` with three
  findings, request_changes on `4c03673` for one documentation defect, approve on
  `a8f10a2`. Reports in `evidence/issue17/slice5/review/`.

### Heads

* code head of record (reviewed + CI-green): **`4c03673`**
* CI-verified head including the review/checkpoint evidence: **`4e77136`** (10/10)
* documentation-only follow-up: `a8f10a2`; trailing evidence commits after
  `4e77136` add the CI receipt and nothing else.

### Process and integration caveats

Core card/effect/event hooks overlap the separate #16 integration lane; integration
must reconcile both histories and verify the merged delta independently — this
draft does not approve that lane. The historical OMP writer/reviewer sessions were
not resumed for this slice: it was implemented natively and reviewed by two
independent review workers running their own probes. No master push, merge, issue
closure or successor activation by this worker.
