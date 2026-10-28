## Current candidate — slice53 Ruxa (2026-10-28)

**Original #17 remains unfinished: 1 of 8 starters certified.** This is the same draft PR46, not a merge or eight-list acceptance request.

### Runnable change

**Ruxa, Patient Professor** (C21 66, complete sourced text) is implemented and enabled: enters/attacks returns a no-abilities creature card from your graveyard, a layer-7c +1/+1 anthem uses the same predicate, and qualifying blocked attackers get the printed assign-as-though-unblocked choice. Typed graveyard targets, return dispatch/commander prediction, pending/action/event/projection, small AI adapters and native pending buttons are included. No import/start UI, search ranking/budget or model/provider change.

Recovery of the timed-out local slice found and fixed a real defect: Surrak's battlefield trample static wrongly excluded vanilla cards in the graveyard from Ruxa's target set. A compiling pre-fix assertion failure is retained. The two-choice test also exercises actual save/load after the first answer and identical second-answer continuation. Initial card implementation was implementation-first; post-hoc controls are not retroactive TDD.

### Scope and verification

- Final code/CI head: **f7e1d419368b8fee1ffd46bd43af78d4207305c0**. Later evidence-only commits claim no CI of their own. Actual exact-head result: `evidence/issue17/slice53/exact-head-ci-f7e1d41.json`.
- Current local gates: 15 Ruxa cases within **8 focused suites / 77 passed / 0 failed**, fmt, workspace Clippy `-D warnings`, locked release, real four-seat game + separate release-CLI replay: all pass.
- Four-seat, 40-life evidence-only package, seed5353: **Win P0 / 1410 actions / 3646 events**, digest `b5cb7a5679b90e2ef4de2a4759e8fec6be840a5feac24d9fef20e86e544eee1a`; 5 Ruxa casts, 11 assignment answers (6 yes / 5 no), 12 returns. Non-sandbox save and replay agree. **Not a 100-card certificate or playing-strength evidence.**
- Catalog **162**; desired union **459 = 149 supported + 310 missing**; per-list gaps `[55,0,51,48,4,52,52,53]`. Whole-field missing_cards AND missing_interactions audit passes.
- Ayula still needs Dúnedain Rangers, Evercoat Ursine, Rampaging Yao Guai and Ursine Monstrosity. Exact desired snapshots and certification statuses are unchanged. Arbitrary Archidekt compatibility remains a non-goal.
- R337-1/2/3 are corrected in opening commit `6d656fb`. Prior approved increments remain preserved.

Independent practical **increment review requested, not claimed**. After approval continue the same card/branch/history with those four names, then exact100-card four-seat certification and separate replays. No merge, master push, issue closure or downstream unlock.

Authoritative handoff: `evidence/issue17/SLICE53-FINAL-HUMAN-CHECKPOINT.md`; structured receipts and disclosures: `evidence/issue17/slice53/RESULT.json`. Historical checkpoints remain in the repository; their old counts and review statuses are not current claims.
