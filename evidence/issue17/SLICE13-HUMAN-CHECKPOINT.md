# Issue 17 — slices 12–13 continuation checkpoint

## Current result

Six sourced cards added on the SAME draft PR46 and branch `aperture/issue-17`:
Sol Ring, Mind Stone, Hedron Archive, Sazh's Chocobo, Rampaging Baloths, and
Professor of Zoomancy. Artifact casting/mana/draw abilities, typed landfall
counters, and Beast/Pest token interactions have executable conformance.

Corpus: **122 catalog cards; 459 desired names = 110 supported + 349 missing**.
**Only 1 of 8 desired 100-card starters is certified**. All desired snapshots
remain unchanged. Full #17 is unfinished; do not complete it or unlock #32.
No merge, master push, force push, issue closure or successor activation.

## Heads and gates

- Independently approved code: `1d99b74371f06bdb645a3d4494ddb48e02d81e14`.
- Final documentation/review-receipt head:
  `a4db05f01adcfb2cc8f677d8a33390cd9bfb35e9`.
- Code-head CI: **10/10 SUCCESS**. Final-head CI: **10/10 SUCCESS**, all bound
  to `a4db05f01adcfb2cc8f677d8a33390cd9bfb35e9`, checked 2026-10-26T18:47Z.
  Configured required checks also pass. Local = remote = current PR head;
  PR remains OPEN, draft, MERGEABLE / BEHIND. No merge was performed.
- Reviews: `slice12/REVIEW12-R3.{md,json}` and `slice13/REVIEW13.{md,json}`.
  Final source-left APNAP compatibility repair was adjudicated in slice13.
- Required final check receipt: `slice13/exact-head-ci.json`.
  Read the live final SHA checks before integration; old code-head checks alone
  do not approve a later head.

## Working demonstrations

Both are exact four-seat, 40-life **40-card packages**, not starter certificates.

1. Artifact package, seed1201: terminal P3, 1573 accepted actions, 6981 replay
   events, digest `d3eade58f133…`. Actual casts 7/8/10, mana activations53/7/6,
   stacked draw activations8/9. Explicitly labeled evidence-only masked driver;
   the product heuristic still ignores artifact spells/mana.
2. Landfall package, seed1301: terminal P1,1568 actions,5018 replay events,
   digest `4cbaa34f0de1…`. New casts2/3/4,18 counter events,1 Beast+4 Pest.
   Pest death/life resolution is proven by a fully drained focused test, not by
   the terminal trace.

Separate CLI replays and fresh save reload/replays match. Old slice8 digest is
preserved. Real failed reviews, invalid saves, CI failures and RED evidence are
retained, not rewritten. The initial writer's save-success claim was false and
is explicitly superseded by executed roundtrip repairs.

## Resume without losing continuity

All implementation/review workers stopped. Both full JSONLs and referenced
session artifacts are byte-verified in `evidence/issue17/slice13-final-backup/`.
Original histories remain:
- writer: `evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl`
- reviewer: `evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl`

Use the SAME card, PR46, branch, histories and `.starter-run.yml`. No blank
replacement team. Slice13 used three bounded assignments; the next slice may
continue sourced missing-card implementation under standing authorization.
`slice13/candidate-source.json` contains authoritative retrievals for all 352
names missing before slice13; it is research, not implementation/support.

Preserve the other agent's Rust-cache cleanup. Use only this lane's current
target, focused affected tests, one real journey, required exact-head CI and one
concise independent delta review. Do not rebuild retired worktrees or unchanged
historical matrices; no cleanup or global/shared-target changes here.

Remaining examples: three unsupported target commanders (Ayula, Elas, Zada),
legend/equipment/Aura/tutor/reanimation and hundreds of printed cards.
Commander's Sphere, Friendly Teddy and Aetherize remain excluded. No arbitrary
Archidekt, full Commander, playing-strength or v1 claim.
