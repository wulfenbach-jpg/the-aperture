# Slice 58 — review348 functional save repairs

**Original #17 is unfinished: one of eight lists certified, Ayula two missing.**
No new card is enabled in this slice: it repairs the two reproduced functional
save-validation defects before the Ring/hideaway work. There is no external blocker.
Native owner authored these repairs and is not their independent reviewer.

## Candidate and boundaries

Same card, branch `aperture/issue-17`, draft PR46; base
`e4308bc8c12296fd995e4187628f6b2d949a6121`; published code and pre-CI evidence head
`2d8d8b3b48ad5e767ffd2d642ee7c55135033010`. Product files match that commit exactly.
No new team or configuration change; inherited OMP history and unrelated reviewer
artifacts remain untouched/unstaged. No merge, master push, closure or dependent unlock.

## What changed

- **R348-1:** the printed ability determines whether a target set is required before
  consulting the saved mode. One shared guard is used by state ingestion and direct
  pick/commit answers. It validates live candidate eligibility through `legal_targets`,
  per-candidate mana value, duplicate candidates, membership/distinctness of selected
  targets and the aggregate total. The ordinary single-target empty-selected rule stays.
- **R348-2:** casting and serialized validation share `MAX_X_VALUE = 20`. Stack entries
  (also the retained entry of a resolving spell), the trigger queue, pending trigger
  references and queued siblings enforce the bound and nonzero-X card/ability association.
  Unrelated activated abilities cannot carry nonzero X. X-spell metadata must agree
  with its actual stack object. No attempt is made to reconstruct a sandbox's mana history.
- The old aggregate validation moved to the shared guard rather than leaving different
  loader and answer rules. No serialized fields, defaults or digest semantics changed.
- Eight new tests in `issue17_slice57.rs` (14 -> 22), plus a normal single-target load
  control in an existing test. Independent duplicate and total-budget probes both use
  the correct X2 and genuinely offered candidates. Old forged cases' wrong-budget
  distractions were also removed. Docs §Slice58 records the scope.

## Executed evidence

Receipts are under `evidence/issue17/slice58/`.

- `red-mode.log`: compiling pre-fix assertion RED, **0 passed / 1 failed**, EXIT101;
  omitted aggregate validated and sandbox-loaded before repair.
- `red-candidates.log`: after the first mode repair but before the shared guard,
  **1 passed / 2 failed**, EXIT101; injected candidates loaded and direct downgraded
  answers were accepted. The earlier mode regression stayed green.
- `red-x.log`: before saved-X repair, **3 passed / 2 failed**, EXIT101; saved X21 and
  non-X Grizzly Bears carrying X1 both validated and sandbox-loaded.
- These five witnessed failing regressions are distinct from the three later
  strengthening/continuation tests; no retrospective all-eight-TDD claim. Earlier RED
  line numbers belong to the pre-rustfmt test revision.
- `gates.log`: `cargo fmt --all --check` 0; focused **7 suites / 79 passed / 0 failed**;
  workspace/all-target Clippy with `-D warnings` 0; whole-field corpus audit 0;
  locked workspace release build 0. The seven suites are card_printings19,
  catalog_pool5, Ruxa15, Ursine4, YaoGuai22, replay_digest6 and starter_corpus8.
- `postcommit.log`: exact committed head, fmt0, YaoGuai22/0, empty product-tree diff.
  These 22 are already included in79, not an extra test total.
- `retained-review348-probe.log`: the original reviewer's unchanged four probes re-run
  by this implementer at the committed head, **4 passed / 0 failed**, exit0. Both
  prior bad states now report validates=false/loads=false; full-state post-pick
  continuation and wrong-seat atomicity still pass. This is retained-probe execution,
  not a new independent verdict, and is separate from79.
- The first gate attempt failed Clippy on an unused import after moving validation;
  `gates-first-unused-import-failure.log` is retained. The import was removed and the
  full focused gate script re-run. That is not a behavioral RED.
- `journey.log`: new actual four-seat/40-life release execution and separate CLI replay,
  WinP2, **1960 actions / 47 turns / 5052 events**, digest
  `8a34da9bb066dd3ba18606043a65e0f4a39a32d60e3a181714f967c0ecb07253`.
  Ten YaoGuai casts, six target sets, thirteen destructions. Fresh non-sandbox save
  round trip succeeds; the separate CLI says the replay reproduces the save, exit0.
  `verified-receipts.log` verifies all three new native JSON artifacts are **byte-identical**
  to slice57. The driver differs only in its output directory and an identifying comment.
  This remains an evidence-only masked-action 40-card package, not product-policy,
  playing-strength, GUI or exact-100-card certification evidence.
- Refusal/continuation fixtures are explicitly **sandbox** SessionSave loads with dummy
  replay decks. They demonstrate structural validation, not bypass of ordinary replay integrity.

## CI and publication

**10/10 check runs completed SUCCESS, all keyed to exact code head
`2d8d8b3b48ad5e767ffd2d642ee7c55135033010`, zero cancelled or failed.**
Receipt `slice58/exact-head-ci-2d8d8b3.json`, observed 2026-10-29T04:12:00Z;
`pr-checks-codehead.log` and `exact-head-workflows.json` independently read back the
push run40815255576 and PR run40815263822, both completed success. No reruns requested.
The lane waited until BOTH workflows completed before making the final evidence push.
Subsequent evidence-only tip carries this checkpoint and final receipts; its own CI
is not claimed. The exact published tip is recorded in the board lifecycle metadata,
separately from the CI-proven code head. No local full-workspace test total is claimed;
exact-head CI supplies that broader gate.

## Still to deliver

Fresh whole-field audit: **164 catalog / 459 union = 151 supported + 308 missing**;
per-list `[55,0,51,48,2,52,52,53]`, one certified. Ayula's missing_cards AND
missing_interactions still name exactly **Dúnedain Rangers** and **Evercoat Ursine**.
No manifest enablement or certification status changed.

Independent concise practical delta review is the next gate, not a self-approval.
After acceptance, same writer/history/card/branch/PR46 continues Ring, then linked
hideaway/free play, then exact100-card four-seat certification and separate replays
once BOTH Ayula gap fields reach zero. Claim two of eight only after actual success.
Children inspected: `t_26a7a4ff` is the scheduled certified-input evaluation wait;
`t_3c2fe51a` is unassigned soak backlog. Neither is this increment's review child;
both remain gated. Do not complete original #17 on a repair approval.
