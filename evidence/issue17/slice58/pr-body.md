## Current candidate — slice58: repair saved aggregate choices and announced X

**Original #17 remains unfinished: 1 of 8 starters certified; Ayula 2 names.** Same draft PR46. This is a functional repair for review348, not a new card, merge request or eight-list acceptance. No merge, master push, issue closure, dependent unlock, new team or global-config change.

### Runnable repairs

- **R348-1:** derive the required aggregate mode from the printed ability before dispatching on saved data. The shared load/pick/commit guard rechecks live candidate eligibility, individual mana values, duplicates, selected membership and the total budget. Clearing the mode or injecting a candidate no longer lets an X1 trigger destroy an MV4 artifact. Direct malformed answers are refused atomically.
- **R348-2:** share the supported X bound `0..=20` between casting and saved-state validation. Stacked/resolving spell entries, stacked/queued/pending triggers and queued siblings reject out-of-range X and nonzero X on unrelated card/ability entries. No reconstruction of sandbox mana history is claimed.
- No serialization/digest shape change, no additional card enablement. Rampaging Yao Guai's sourced text and full-clause support from slice57 remain; the three current journey JSON files reproduce the prior recordings byte-for-byte.

### Exact candidate and evidence

- Code and pre-CI evidence head: **`2d8d8b3b48ad5e767ffd2d642ee7c55135033010`**, **10/10 completed SUCCESS checks, all keyed to this SHA, zero cancelled/failed; no reruns**. Receipt: `evidence/issue17/slice58/exact-head-ci-2d8d8b3.json`; push40815255576 and PR40815263822 both completed success before the final evidence push.
- Any subsequent commit is final evidence only, not another product head; no CI is claimed for that moving evidence tip. The code-head CI completes before the evidence push.
- **79 focused tests across 7 suites, zero failures**, including YaoGuai22 (eight added, 14→22); fmt, workspace/all-target warnings-denied Clippy, locked workspace release and whole-field corpus audit pass. Exact postcommit YaoGuai22 and the retained review348 probes4/4 also pass; these repeats are not added to79. Five new regressions have genuine pre-fix compiling assertion REDs; three additional tests are later strengthening/continuation controls, not retrospectively TDD.
- Actual four-seat/40-life release journey: **WinP2,1960 actions/47 turns/5052 events**, digest `8a34da9bb066dd3ba18606043a65e0f4a39a32d60e3a181714f967c0ecb07253`; ten YaoGuai casts, six target sets, thirteen destructions. Fresh non-sandbox save/load and separate release-CLI replay both pass. The driver is an evidence-only masked-offer demonstrator on an exact40-card package, **not** product AI, a100-card certificate, playing strength or GUI evidence.
- Malformed-save and continuation probes explicitly use **sandbox** SessionSave loads. They demonstrate structural validation, not a bypass of ordinary replay-integrity checks.

### Remaining full contract

Fresh recount: **164 catalog;459 union=151 supported+308 missing**, gaps `[55,0,51,48,2,52,52,53]`, certified **1/8**. Ayula's missing_cards AND missing_interactions still name **Dúnedain Rangers** (Ring) and **Evercoat Ursine** (linked hideaway/free play). Only Jasmine is certified; inherited one-defender-per-declaration combat remains a disclosed limit.

All original gates remain: eight explicitly supported100-card lists spanning different styles; implementation/conformance for every included card and interaction; four-seat games using exact starter snapshots, never bundled replacements; documented per-card/interaction exclusions. Arbitrary Archidekt compatibility remains a non-goal. After both Ayula gap fields reach0, run exact100-card four-seat certification and separate replays; claim2/8 only on actual success. Evaluation/soak children remain gated.

Handoff: `evidence/issue17/SLICE58-HUMAN-CHECKPOINT.md`; structured result: `evidence/issue17/slice58/RESULT.json`. The native lane authored this repair and requests a concise independent practical delta review; it does not self-approve.
