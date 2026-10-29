# Review 286 — slice 29, contract lens

Verdict: **APPROVED_INCREMENT; ORIGINAL ISSUE #17 INCOMPLETE**.
Reviewed head: `de3ba07c0a89360a4f86422417790ddf6677a328` (code `99785d783f991918b541fef7793a3e6a1bb48e75`). Delta begins at previously accepted `6f58f75`. No blocking delta defect found. No implementation edits by this reviewer; only this untracked review directory, local build products and the review skill's reusable pitfall note were written. Existing dirty historical evidence was preserved.

## Independently executed

- `cargo test -p aperture-core --locked --test issue17_slice29 --test issue17_slice28 --test issue17_slice26 --test issue17_slice24 --test issue17_slice25 --test timing_mana --test issue17_slice13`: exit 0, **53 passed / 0 failed** across seven binaries (9 tests in the new slice29 suite). `focused.log`.
- `cargo fmt --all --check`, `git diff --check -- crates`: both exit 0. `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0 (`clippy.log`). Did not repeat the unchanged whole-workspace test matrix under DELIVERY_FIRST.
- Exported the committed slice29 driver with `git show`, changed only Cargo path dependencies to absolute local paths, copied the committed exact package deck, and rebuilt/run it in `review286/repro`. The source bytes equal the committed driver. Exit 0: **four seats, 40 starting life, seed 2929, terminal Win P3, 1912 actions / 44 turns / 5311 events**, digest `ac16e510ca4691a69981beafcb577f3aa8f01bd6973710f86b89275899030024`. Four Stuffed Bear casts / 119 animation starts; 1301 animated and 3741 plain invariant sightings. All three regenerated artifacts equal their committed bytes. `journey.log`.
- Separate release `aperture-cli replay --file <own regenerated demo-replay.json>`: exit 0, recorded/replayed digest matches, 5311 events. `cli-replay.log`.
- Additional uniquely named instrumented probe runs the same legal game and captures/loads a **real SessionSave during the first active animation**, not merely raw GameState serde or a terminal save: marker `MID_ANIMATION_SESSION_VALIDATION_PASS steps=408`, exact state equality, then the same terminal digest. `mid-animation.log`. Initial copied-package invocation reused an uninstrumented binary; it was not counted as this verification. The unique package name forced the instrumented binary and marker. The initial preparation attempt also had an incorrect path depth and a no-User-Agent Scryfall request that failed; both are reviewer harness errors, not candidate failures.
- Refreshed both MID/259 and DBL/259 directly from the Scryfall API using explicit User-Agent and Accept headers. Both texts/types/costs equal fixture/catalog/ability: Artifact, {2}, complete self-animation ability. `mid-259-live.json`, `dbl-259-live.json`.
- Re-ran the earlier independent reviewer audit against CURRENT catalog + all eight exact lists and commanders, cross-checking missing-card set equality with the manifest: **145 catalog / 459 union = 133 supported + 326 missing**, per-list **55/0/51/48/20/52/52/53**; Ayula 20 missing; **1 of 8 certified**. `corpus-audit.json`.
- Live PR readback: OPEN/draft, exact tip de3ba07. All **eight required check contexts across push and pull_request pass**, `gh pr checks --required` exit 0 (`required-checks.txt`). Last full API capture (`live-checks.json`): **9/10 completed success, 1 optional Windows package job still in progress**, zero failures, every head_sha equals de3ba07. This report does NOT claim 10/10 complete at tip.

## Contract audit

1. Eight supported 100-card lists: **NOT MET**. Eight desired lists exist but only Jasmine is certified; Ayula remains 20 names short.
2. Every included card/interaction implemented and covered: **NOT MET for desired corpus**. 326 union names still unsupported. Stuffed Bear itself now has a real cast/resolve path, correct layered type/colour/base P/T, target offers, combat/sickness, cleanup, creature-death watcher and save coverage. New tests assert behavior, not only catalog strings.
3. Four-seat exact snapshots: **PASS for bounded increment only**. This real journey is a 40-card package with injected Jasmine commanders, not a desired 100-card starter certificate. No substituted deck and no strength claim.
4. Exclusions/non-goal: documented current 145/133/326/Ayula20 figures match independent audit; arbitrary Archidekt compatibility remains excluded. Prior approved increments were not reopened. The layer/haste documentation repairs remain consistent with the current mechanics. New delta touches no crates/ai or crates/ui and changes no planned desired deck snapshot.

## Bounded caveats, not a polish hold

- Product AI still reads the printed Object::is_creature baseline for its belief model; the evidence demonstrator, not AI strength, is what this journey proves.
- Effective type/colour queries cover this supported pool, not arbitrary layer ordering/dependency interactions. Only green colour setting is exercised. Existing layer7a/keyword limitations remain.
- The new save unit test uses raw state serde; the independent probe above additionally validates one actual mid-animation SessionSave. It is not a claim about all pending/stack continuations.
- The death test stops at the expected life total rather than explicitly draining every pending/queued item; no duplicate trigger defect was observed, but its assertion could be strengthened when next touched.
- Historical uncommitted evidence appends are pre-existing and untouched. Product-file diff is empty.

## Routing

Return **the same card / draft PR46 / branch aperture/issue-17 / original saved histories** to sourced implementation, preserving accepted increments 1–29 without re-audit. Work toward genuine whole-list certification from the sourced remaining Ayula demand rather than another evidence-polish slice. Full issue remains unaccepted. Both children were inspected: #26 scheduled dependency wait, #32 todo/unassigned, neither a precreated review lane; neither may be unlocked. No merge, master push, issue closure or successor activation performed.
