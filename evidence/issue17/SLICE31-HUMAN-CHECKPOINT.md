# Slice 31 — Beorn the Fierce (HOB 119)

Date: 2026-10-27. Card: **Beorn the Fierce** (`{3}{G}{G}`, 6/6 Legendary Creature — Bear Shapeshifter Warrior).
Printing: Scryfall **HOB 119**, immutable `367d5f8b-77ee-47f7-bc71-972d62c280a9`, oracle
`30ca8e92-5955-46a1-86c1-094a873f518f`, retrieved **2026-10-27T15:33:31Z**, recorded in
`fixtures/starter-corpus/slice31-sourced.json` and consumed by the provenance guard
(`crates/core/tests/card_printings.rs`, new `every_slice31_card_matches_its_recorded_printing_and_trample`).

Printed text (verbatim):

```
Trample
Other Bears you control get +2/+2.
At the beginning of combat on your turn, put a trample counter on up to one target creature you control. It becomes a Bear in addition to its other types. Then if you control three or more Bears, draw two cards.
```

## What is new and reusable

* `TriggerEvent::BeginningOfCombat` (CR 507.1) dispatched once at the start of the beginning-of-combat
  step, only for the active player's permanents, before priority is granted.
* `TargetSpec::UpToOneCreatureYouControl` + `Action::DeclineTriggerTarget` (CR 115.6): the engine now asks
  for the target whenever one exists — including exactly one — and always offers "choose no target".
  Declining still resolves the ability, so the conditional draw is unaffected.
* `CounterKind::Trample`, the pool's first **keyword counter** (CR 122.1b). `Game::has_trample` reads it,
  cleanup never removes it, and the creature keeps trample after the turn ends.
* `Effect::AddTargetCreatureType(SubtypeLine::Bear)` → layer 4 *additive* continuous effect on the chosen
  target (CR 613.1d), until end of turn because no duration is printed (CR 611.2a).
* `StaticAbility::OtherBearsYouControlGet` — a layer 7c subtype anthem whose recipients are computed from
  the **effective** subtype (so a layer 4 Bear counts) and which never pumps its own source.
* `Effect::DrawIfControlsBears { minimum, cards }` — the Bear count is taken after the earlier
  instructions of the same resolution (CR 608.2c), via the effective subtype.

## Verification actually run (all logs in `evidence/issue17/slice31/`)

* `issue17_slice31` — **8/8** focused tests: registration/printing, the up-to-one offer including declining,
  counter + Bear + anthem after choosing, three-Bears draw two, two-Bears no draw, real trample combat over
  a blocking 1/1 (life 20→17, blocker dies), cleanup keeps the counter but ends the subtype, and the
  trigger firing only on its controller's turn.
* Four discriminating mutation REDs (`run-mutations.py`): trample-counter read dropped, Bear-count gate
  ignored, optional target made mandatory, subtype anthem disabled. Each fails exactly the named tests and
  `engine.rs` is restored **byte-for-byte** (`pre-`/`post-mutation-sha256.txt` both
  `8fe51710e795eea457fa9718b5b62596c630ca92dd8caae60684998c1b7030d2`).
* Gates (committed logs carry their own exit codes): `fmt.log` `FMT_EXIT=0`, `clippy.log`
  `CLIPPY_EXIT=0`, `workspace.log` `WORKSPACE_EXIT=0` with **117 suites / 827 passed / 0 failed**,
  `release-build.log` `RELEASE_EXIT=0`.
* Four-seat / 40-life exact-snapshot journey from `evidence/issue17/slice31-journey-driver`
  (`decks/package-slice31.txt`, seed 3131, Jasmine Boreal injected for all four seats by the demonstrator):
  terminal **Win{P2}**, **1917 actions**, 46 turns, **4930 replay events**, digest
  `36e181859f398146b2cac38278a712db4ed0cddc4372f9e23642ce57a51da79f`, snapshot sha256
  `370cfe3bcd9095671f600e55c6563614bad8a77b3223909e1d7bf8433efbc22a`, 7 Beorn casts, 13 beginning-of-combat
  triggers staged, 13 trample counters placed, 13 Bear additions, 8 Bear-count gates correctly declined.
  A separate release CLI replay of the recorded replay matches the digest (`cli-replay.log`).
* Independent corpus audit (`corpus-audit.py`, recomputed from `catalog.rs` + the eight desired decks and
  asserted equal to every manifest `missing_cards` entry): **147 catalog / 459 union = 135 supported +
  324 missing**, per-list `55/0/51/48/18/52/52/53`, **1 of 8 certified**.
* Exact-head CI at `e53b987`: **10/10 check runs completed SUCCESS, every `head_sha` = e53b987**
  (`exact-head-ci-e53b987.json`, capture exit 0 in `ci-capture.log`).
* Independent review: **APPROVED_INCREMENT**, no functional defect and no reviewer edits
  (`evidence/issue17/review287/REVIEW287.md`). Its non-blocking notes: the PR body had to be refreshed
  (done — `pr-body-readback.md`), the slice-15/16 replay fixtures are stale at the previous head too
  (pre-existing, verified identical at 073fd19), layer 7c additive statics only apply at recompute points
  (already disclosed), and the Surrak gap should become a tracked follow-up.

## Disclosures

* The journey is a 40-card package, **not** a 100-card starter certificate, and the driver is an
  evidence-only masked demonstrator, not product AI or strength evidence.
* The anthem is recomputed only at the engine's existing layer-recompute points; no supported effect
  changes a subtype without such a point.
* **Newly discovered, not fixed here:** `Surrak and Goreclaw` prints its own `Trample`, but its catalog row
  is a plain `card!` (trample `false`) and its static grants trample only to other creatures, so Surrak
  itself does not trample. Repairing an approved card's behavior mid-slice would also move the digests of the
  journeys that used it, so this is recorded (also in `docs/issue17-starter-corpus.md`) for a separately
  reviewed increment.
* Untimed network helpers were the cause of the run-287 wedge: `slice30/capture-ci.py` and
  `slice30/capture-ci2.py` now pass `timeout=120` to every `gh` call and exit **124** on a timeout; the
  slice-31 capture helper is written with the same bound from the start.

## State after this slice

Original issue #17 remains **unfinished**: 1 of 8 desired 100-card lists certified, Ayula still 18 names
short. No merge, no master push, no issue closure, no downstream unlock (`t_26a7a4ff`, `t_3c2fe51a` stay
parked). Next sourced card: Kamahl's Summons (ONS 269) per the supervisor's slice-31 order, which needs a
per-player arbitrary-hand-subset reveal choice resolved APNAP.
