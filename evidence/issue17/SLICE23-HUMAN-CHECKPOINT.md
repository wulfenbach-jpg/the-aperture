# Issue17 slice23 — rolling checkpoint

Same card t_67778feb / checkout `C:/AI/aperture-worktrees/issue-17` / branch
`aperture/issue-17` / draft PR46. **Continuation, not acceptance.**

## Head of record

Committed this slice: `SLICE23_HEAD` (see the commit subject
"Issue 17 slice 23: Goreclaw cost reduction and granted trample").
Local == remote after the push readback recorded in
`evidence/issue17/slice23/push-readback.txt`.
**No merge, no master push, no issue closure, no certified-downstream unlock.**

## Usable addition

**Goreclaw, Terror of Qal Sisma**, `{3}{G}` Legendary Creature — Bear 4/3.
Scryfall **CMM 293**, immutable `ee47f23a-3ba9-4615-b170-c89d8ab99d78`, oracle
`befb211f-37ca-4083-98d4-9ff1f28be3f2`, retrieved `2026-10-27T07:38:22Z`. Raw
collection bytes `evidence/issue17/slice23/raw/cards-collection.json`, normalized
record `fixtures/starter-corpus/slice23-sourced.json`, catalog oracle text
byte-equal to the printing, `card_printings` chains the new fixture.

Both printed abilities implemented in full:

1. "Creature spells you cast with power 4 or greater cost {2} less to cast."
   `StaticAbility::CreatureSpellsYouCastWithPowerCostLess`, applied inside
   `effective_cost` after commander tax, to the **generic** part only and never
   below zero (CR 601.2f, 118.7a). Only the controller's own creature spells
   qualify; a noncreature spell and an opponent's creature spell are unaffected.
2. "Whenever Goreclaw attacks, each creature you control with power 4 or greater
   gets +1/+1 and gains trample until end of turn." The `Attacks` trigger
   (slice 19) stages it; resolution reads each battlefield creature's **current**
   power. The pump is a normal layer 7c `ContinuousEffect`; the trample is the
   pool's first `ContinuousKind::GrantsTrample` in a new `Layer::Ability`
   (CR 613.4d), read by combat damage assignment through `Game::has_trample`.

Both halves expire in cleanup (CR 514.2) and are dropped when their target
leaves the battlefield, because they are ordinary `ContinuousEffect` rows rather
than a second per-object grant table — the slice-21 stale-shield defect is not
reproduced.

## Evidence (all commands actually run this slice)

* `slice23/mutation-reds.py` + three retained logs — discriminating mutation
  REDs, engine restored byte-for-byte (sha256 before == after):
  * `red-mutation-cost-reduction-disabled.log` — exactly the two cost tests fail
    (9 passed / 2 failed).
  * `red-mutation-threshold-ignored.log` — exactly the qualifying-creature test
    fails (10/1).
  * `red-mutation-trample-grant-ignored.log` — exactly the trample combat test
    fails (10/1).
* `slice23/focused-tests.log` (from `native-gates.sh`) — `issue17_slice23`
  11/11 pass.
* `slice23/native-gates.log` — every step's own exit code: `fmt_exit=0`,
  `clippy_exit=0`, `release_exit=0`, `slice23_exit=0`, pinned suites
  `card_printings`, `catalog_pool`, `issue13_commander_combat`,
  `issue17_slice15_corpus`, `abilities`, `slice20`, `slice21`, `slice22`,
  `projection`, `timing_mana`, `m1_multiplayer`, plus `workspace_exit=0`.
* `slice23/native-journey.log`, `demo-journey.json`, `demo-replay.json`,
  `demo-save.json` — real four-seat / 40-life / 40-card package journey, seed
  2323, exact `decks/package-slice23.txt` snapshot
  `5f933f49cec503bcbebb8d35c4eb3609f13c84d6fe22dcefd82d2dd7c2e71fb7`: terminal
  **Win{P3}**, **1322 actions / 32 turns / 3495 replay events**, digest
  `e2ccc113bcebfcdbe60e7d0bf875bd88076e3b83961a9476978384aef30d130e`, separate
  replay reproduces it, fresh save roundtrips; Goreclaw cast 5×, its trigger
  staged 12×, 45 trample-grant and 45 pump continuous effects started. Evidence-
  only masked demonstrator, **not** product AI; Jasmine Boreal injected into all
  four command zones; **not** a 100-card certificate.
* `slice22/demo-replay.json` (and slices 17–21) still replay to their recorded
  digests through the release CLI — the new serde variant and layer do not move
  any previously recorded digest, because a state that never grants trample
  serializes exactly as before.
* `slice23/corpus-audit.log` — `AUDIT OK: catalog 140, union 459 = 128 supported
  + 331 missing, Ayula 25, 1 certified`, per-list `55/0/51/48/25/52/52/53`.

## Correction disclosed (test expectations, engine was right)

My first cut of the focused tests asserted Craw Wurm as a 6/6 and predicted 35
life after the trample combat. The card is a printed **6/4** (so the pumped
attacker is 7/5), and **both** attackers qualify and trample (Goreclaw 5/4
tramples 3 over its blocker, the Wurm 5), giving 32 life. The engine behaviour
was correct; the tests were corrected to the printed characteristics and the
arithmetic re-derived from the combat log in `native-journey`/debug output. No
engine change was made for this.

## Known gaps (unchanged by this slice)

* **Still 1 of 8** desired 100-card starters certified (jasmine only).
* Ayula still 25 names short; per-list `55/0/51/48/25/52/52/53`.
* The cost reduction counts one static per permanent (correct for one Goreclaw;
  the pool has no second copy to test stacking).
* Printed power is what the reduction reads; no supported effect changes a
  card's power outside the battlefield, so there is no hidden-zone recalculation.
* `view`/projection does not display a granted keyword; combat enforces it, and
  no UI change is made in this lane.
* Full original #17 acceptance remains open.

## Next

Continue the same card / PR / branch / histories. Nearest Ayula work after this
slice: the trample-grant machinery now exists, so `Drover Grizzly` (saddle +
trample grant) and `Surrak and Goreclaw` (static trample to other creatures,
enters-trigger counter + haste) are the cheap follow-ons; `Circle of the Moon
Druid`/`Stuffed Bear` still need the layer 4/5/7b subsystem and are not cheap.
