# Issue17 — slice 5 checkpoint (mill / noncreature-counter / ritual package)

Full issue17 is **UNFINISHED**. This is a bounded implementation increment on the
existing draft PR46, not eight certified starter lists. No merge, master push,
issue closure or successor activation was performed.

## Published heads

* draft PR46: https://github.com/wulfenbach-jpg/the-aperture/pull/46
* **code head of record (reviewed + CI): `4c03673`** — the six-card package plus
  the three review repairs.
* **approved head: `a8f10a2`** — documentation only (`git show --stat a8f10a2`
  touches `HANDOFF.md` and nothing else), confirmed by the round-3 review.
* earlier candidate `eb0a6f5` was independently approved (round 1,
  `approve_increment`, three findings) and is superseded by `4c03673`.
* PR OPEN, MERGEABLE, still draft; branch is behind master by other lanes'
  commits (no engine drift).
* Independent practical review (two independent review workers, three rounds):
  * round 1 on `eb0a6f5`: **approve_increment**, findings S5-R1-F1 (deck file
    claimed 40 cards but had 50), S5-R1-F2 (Preordain test could not tell
    scry-then-draw from draw-then-scry), S5-R1-F3 (info: each-opponent effects
    iterate seat order, not APNAP);
  * round 2 on `4c03673`: **request_changes** for one documentation defect —
    the HANDOFF note wrongly attributed the seat-order deviation to Crack the
    Earth, whose sacrifice really does walk APNAP;
  * round 3 on `a8f10a2`: **approve_increment**, documentation now matches the
    code clause by clause.

## What is usable now (real behaviour, sourced this run)

| card | printing | new behaviour |
|---|---|---|
| Stitcher's Supplier | tdc 196 | mill three on entering **and** on dying (one printed ability modelled as two trigger events) |
| Ruin Crab | znr 75 | landfall: each opponent mills three |
| Negate | tmt 47 | counter target **noncreature** spell |
| Preordain | tdc 161 | scry 2, then draw a card |
| Pyretic Ritual | m11 153 | add {R}{R}{R} to the pool |
| Seething Song | c21 179 | add {R}{R}{R}{R}{R} to the pool |

Provenance: `fixtures/starter-corpus/slice5-sourced.json` records, per card, the
printing (set/collector/release date), `oracle_text`, power/toughness, keywords,
`printings_checked`, the list of distinct printed texts and the retrieval
timestamp (`2026-10-26T…Z`). Seething Song has ten printings, one of which (sos
109) is a prepare-layout double-faced card whose top-level `oracle_text` is empty;
every single-face printing carries the identical text.

New rules primitives, each with focused coverage:

* `Effect::MillSelf` / `Effect::MillEachOpponent` — a real library→graveyard zone
  move, one `Event::Milled` per card, no "dies" trigger (a milled card never
  entered the battlefield), no loss on an empty library (CR 701.13a; CR 704.5b
  is a *draw* rule), and the controller never mills for "each opponent".
* `TargetSpec::NoncreatureSpell` — a creature spell on the stack is never
  offered as a Negate target and a forged `Target::Spell` on a creature spell is
  refused with `BadTarget` and no state change (CR 205.2a).
* `Effect::AddMana` — mana added to the pool without tapping a permanent
  (CR 106.4), with a truthful `Event::ManaAdded.pool_total`.

## Corpus movement (native recomputation)

`evidence/issue17/slice5/update_manifest.py` recomputes every desired list's
`missing_cards` from the compiled catalog and asserts each desired deck file is
still exactly 99 cards:

* catalog **97 → 103** real cards;
* eight-list union **459** exact names (unchanged — desired lists untouched);
* supported **85 → 91**, missing **374 → 368**;
* per list: talrand 54→52, elas 58→57, tatyova 56→55, zada 57→55; isamaru 56 and
  ayula 47 unchanged;
* **1 implemented / 0 certified** starters; the seven planned lists stay planned;
* every new card sits in a real desired list (no orphan cards).

Evidence: `evidence/issue17/slice5/slice5-corpus-state.json`.

## Verification actually run

* `crates/core/tests/issue17_slice5.rs` — **10** focused tests.
* **Genuine RED (round 1)**: with the three engine hooks disabled (mill, opponent
  mill, mana addition) and the noncreature filter opened up, exactly the five
  behaviour tests fail while the provenance/structure/control tests pass —
  `evidence/issue17/slice5/slice5-red-before-engine.log`; the engine diff is
  preserved as `engine-slice5.diff`. Restored afterwards: all green.
* **Genuine RED (F2)**: with the Preordain effects swapped to draw-then-scry the
  strengthened test fails —
  `evidence/issue17/slice5/slice5-red-F2-order.log`. Restored afterwards.
* `cargo test --workspace --locked` → **526 passed, 0 failed** over 82 suites
  (`slice5-workspace-tests.log`).
* `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked --
  -D warnings`, `cargo build --workspace --release --locked` → all exit 0
  (`slice5-fmt.log`, `slice5-clippy.log`).
* Real executable four-seat journey (`decks/package-slice5-mill.txt`, exactly 40
  cards, four seats, 40 life, commander-combat practice, seed 501): **terminal**,
  winner P2, 1880 steps / 45 turns, digest `d0c888f680d7…`, **111 mill lines** in
  the trace, and a separate `replay --file` verification: 1880 actions, 4654
  events, digest matches (`slice5-journey.json`, `.replay.json`, `-replay.log`).
  P3 is eliminated by being milled out and then drawing from an empty library —
  the new primitive changes real game outcomes.
* Provenance guards extended: `every_slice5_card_matches_its_recorded_printing`
  (catalog string, cost, type line, P/T and the distinct printed texts against
  the live record) and a pool guard that now accepts "counter target spell" and
  "counter target noncreature spell" while still refusing ability-only effects in
  spell text.

## Honest limitations

* **Target-player milling is NOT implemented** (Hedron Crab): the engine has no
  player target yet. `DeckList::parse_practice` still refuses the card, and a
  focused test asserts that refusal.
* Milling a commander out of a library puts it into the graveyard and then the
  existing CR 903.9a state-based action offers its owner the return; no
  mill-specific replacement is claimed.
* Each-opponent effects (`MillEachOpponent`, `DamageEachOpponent`) iterate seat
  order rather than APNAP order; unobservable in state, only visible in log
  order. Crack the Earth's sacrifice does walk APNAP.
* Nothing reads the graveyard yet: no "whenever a card is put into a graveyard"
  trigger, no threshold, no reanimation, no delve. Stitcher's Supplier and Ruin
  Crab therefore only *fill* the graveyard.
* The CLI heuristic never casts a ritual or a Negate in this journey (0 casts of
  each); the primitives are exercised by the focused tests and by the reviewer's
  own probes, not by that heuristic trace.
* Counters are still display-only, keywords other than Flash/Trample/flying/
  defender are unmodelled, the legend rule, artifacts/equipment, tutoring,
  copying and sacrifice outlets are unimplemented — the eight desired lists
  cannot be certified.
* Shared core hooks (`card.rs`, `catalog.rs`, `engine.rs`, `event.rs`, `view.rs`)
  remain an integration hotspot with the separate #16 lane; this work does **not**
  approve #16 integration.
* Reporter pointer still targets completed issue20 and was not changed here.
* The historical OMP writer/reviewer sessions were not resumed in this run: this
  slice was implemented natively and reviewed by two independent review workers
  that ran their own probes. The saved OMP histories and backups listed in the
  slice-4 checkpoint are untouched.

## Exact resume state (next slice)

* Branch `aperture/issue-17`, PR46 still draft. No merge, no closure.
* Reviewer artefacts: `evidence/issue17/slice5/review/` (reviews 1–3, probe
  crate, Scryfall re-check, corpus re-check, workspace/journey logs).
* Reviewer assignment: `evidence/issue17/SLICE5-ASSIGN-REVIEW-1.md`.
* Next cheapest packages from the recomputed manifest: `Hedron Crab` needs
  target-player mill (player target + resolution); `Stitcher's Supplier` unlocks
  a "creature card put into a graveyard" trigger family (Syr Konrad, Altar of
  Dementia) once graveyard watchers exist; Isamaru's remaining cards need
  artifacts/equipment and the legend rule; Ayula's need counters as rules
  objects, which stays the #16 hotspot and must be integrated serially.
