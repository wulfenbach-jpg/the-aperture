# Slice 53 — Ruxa shipped on draft PR46; original #17 remains unfinished

**One new supported card, not a third record-only slice.** Ruxa, Patient Professor
now has all three printed clauses implemented. Ayula has four unsupported names
left; only Jasmine is certified, so the corpus remains **1 of 8 certified**.
Independent review of this increment is requested, not claimed.

## Candidate and recovery

Same card, writer checkout, branch `aperture/issue-17`, and draft PR46.

- Base: `36eabc34bff449ec43a8cddbceeffb691c226a6e`.
- Opening record corrections: `6d656fb035f0db05b74b551d7de002f9ad497b06`.
- Initial Ruxa product: `fa3c7bbdd82c29682c5c276a045c4c24cc49f7e0`.
- **Final product / exact CI-receipt head:** `f7e1d419368b8fee1ffd46bd43af78d4207305c0`.
- This report belongs to a later **evidence-only** commit. Its actual published
  SHA is recorded in the native task's post-commit handoff. No CI pass is claimed
  for that evidence tip; CI is keyed to the full final product hash above.

The previous run timed out after committing the card locally. Recovery preserved
that work, exercised it, fixed one concrete defect, and published to the same PR.
`SLICE53-HUMAN-CHECKPOINT.md` was an unpublished pre-recovery draft; this document
and `slice53/RESULT.json` supersede it.

## The printed card, implemented end to end

**C21 66**, `{2}{G}{G}`, legendary 4/4 Bear Druid. Source and original retrieval
stamp: `fixtures/starter-corpus/slice53-sourced.json`, 2026-10-28. Recovery re-fetched
the exact printing at `2026-10-28T23:33:42.385931+00:00`: HTTP 200, all checked
fixture fields equal, including the complete Oracle text. See
`slice53/printing-recheck.json` and the retained raw `ruxa-printing-live.json`.

1. **Enters/attacks recursion:** typed graveyard `Target::Card`, the exact
   no-abilities creature-card target specification, a return-to-hand effect
   through the zone-change path (never draw), and commander hand-replacement
   destination prediction from the resolving ability. Enters and attacks have
   separate typed trigger entries.
2. **No-abilities anthem:** additive layer-7c static, using the same predicate as
   the target filter and permission. Existing subtype-anthem plumbing is reused.
3. **Assign as though unblocked:** typed combat-damage pending, one answer per
   qualifying blocked attacker, answer action, public pending projection, event,
   AI compatibility/answer handling and native buttons. Damage is applied once
   after the last answer; blockers still deal their own damage.

`issue17_slice53_ruxa` has **15 passing tests**: registration, both trigger events,
no-abilities target filtering/refusals, anthem recipients, accepting/declining the
permission, two-choice single-batch assignment, excluded attackers, malformed
pending validation **and real SessionSave load** refusals, commander destination,
and the recovery regression below. `card_printings` pins the sourced card and
implemented shape. These are named tested cases, not a claim that every possible
malformed state has been exhausted.

### A real defect caught during recovery

The shared no-abilities predicate applied Surrak's battlefield-only trample static
to vanilla creature **cards in the graveyard**, disqualifying legitimate Ruxa
returns. The regression compiled and failed on the pre-fix predicate:
`slice53/red-graveyard-grant.log`, **0 passed / 1 failed, EXIT=101**.

The final guard restricts this grant to battlefield recipients. The regression
keeps a live Bear ineligible while offering two graveyard Bears, chooses one and
proves its actual return to hand. An intermediate fixture wrongly expected a
prompt for a sole legal target, which the engine correctly auto-selects; that
harness mistake is retained in `recovery-fixture-failure.log` and
`recovery-gates-FIRST-FAILED.log`, not passed off as another product defect.

The two-attacker test now saves **after answer 1 of 2**, validates and really
reloads, then compares second-answer events and the complete direct/restored
states. This strengthens after-implementation coverage, not retroactive TDD.
Fixture saves are sandbox-marked with dummy replay decks; they prove the real
load/continuation path, not fixture-history provenance. The journey below is a
separate non-sandbox game with a real recorded history.

## Whole corpus fields, not just the removed entry

`missing_cards` and `missing_interactions` both remove Ruxa. The fresh
`slice53/whole-field-audit.log` recounts all lists from desired deck files and
catalog names, compares every missing-card field, scans every interaction field
for stale exact parenthesized supported-card citations, and requires Ayula's
four cited names to equal its four missing cards. **AUDIT OK, EXIT=0.** Runtime
catalog consumers also pass in the focused command.

- Catalog **162**; union **459 = 149 supported + 310 missing**.
- Per-list gaps: **55 / 0 / 51 / 48 / 4 / 52 / 52 / 53**.
- Certified: **1 of 8**, unchanged.
- Ayula remaining: **Dúnedain Rangers, Evercoat Ursine, Rampaging Yao Guai,
  Ursine Monstrosity**.

Review337's R337-1/2/3 rode in opening commit `6d656fb`: corrected engine-vs-state
anchors and overbroad prerequisite claims; distinguished slice52's product,
gates, CI and evidence heads; removed the false committed-CRLF/fresh-LF claim.
No approved earlier product increment was reworked.

## Real four-seat game and separate replay

`slice53-ruxa-driver` uses engine-projected legal actions with an evidence-only
policy. Four seats, 40 life, seed 5353, exact `decks/package-slice53.txt` snapshots,
Jasmine in the command zones. This **40-card package is not a starter certificate
or product-AI/strength evidence**.

At final code head: **Win P0**, **1410 actions / 33 turns / 3646 events**;
digest `b5cb7a5679b90e2ef4de2a4759e8fec6be840a5feac24d9fef20e86e544eee1a`.
Five Ruxa casts, 11 assignment answers (6 accept / 5 decline), 12 graveyard-to-hand
returns. In-driver replay and fresh non-sandbox save load match. The separate
release CLI reproduces the save: stored == recomputed, 1410 actions / 3646 events,
**EXIT=0**. Receipt: `slice53/journey.log`, both own exit lines.

## Gates and honest carryforward

Current final-head receipts: `slice53/recovery-gates.log`:

- fmt, **8 focused suites / 77 passed / 0 failed**, corpus audit, workspace
  all-targets warnings-denied Clippy, locked workspace release, journey: all exit 0.
- Exact-head CI: `slice53/exact-head-ci-f7e1d41.json` and `RESULT.json.ci` contain
  the actual keyed check runs. Do not substitute another commit's green status.

Retained pre-recovery evidence at initial product `fa3c7bb`:

- True `aperture-ai --test search`: **9 passed**, own EXIT=0 in `gates.log`;
  **not part of the current 77**.
- `workspace-tests.log` and `workspace-tests-rerun.log`: each **134 result lines /
  962 passed / 0 failed / 1 ignored / EXIT=0**. No doc-test headers. Not claimed
  re-run locally after the recovery repair; exact final-head CI supplies that gate.
- `gates.log` also retains a failed linker invocation, EXIT=101, not a green gate.
- Original 14-test predicate/assignment mutation controls at `fa3c7bb` are post-hoc
  discrimination, **not TDD**. Initial implementation was implementation-first;
  only the recovery predicate repair has the newly retained pre-fix assertion RED.

## Boundaries and next gate

- Native authored the recovery repair and is not its independent reviewer.
- Narrow AI enum/copy/heuristic-answer adapters and native pending buttons were
  necessary for the new choice. No search ranking/budget/model/provider or
  import/start UI change; no new manual GUI interaction claim.
- Other combat assignment remains the disclosed automatic lethal-first
  simplification. The shared predicate enumerates current ability sources;
  future ability-grant mechanisms must extend it.
- A prior broad EOL pass damaged two **untracked** review309 PNG archive copies.
  Recovery proved exact transformed-byte equality with that archive's own
  `e6babc6` blobs and restored those exact bytes (not current similarly named
  images). Receipt: `restore-archive-images.log`. Other inherited artifacts and
  the sessions JSONL edit are excluded from this publication.
- Children inspected: scheduled evaluation wait `t_26a7a4ff` and unassigned soak
  backlog `t_3c2fe51a`; neither is a review child. Both remain gated.

**Next:** one concise independent practical review of this slice, then the SAME
writer/history/card/branch/PR46 implements the remaining four Ayula names. Run
exact 100-card four-seat certification and separate replays once both gap fields
are genuinely empty; report 2 of 8 only if it passes. Original #17 is not complete.
No merge, master push, issue closure, dependent unlock, new team or global config
change was performed or requested.
