# Issue17 — slice20 FINAL reviewed checkpoint

Supersedes SLICE20-HUMAN-CHECKPOINT.md's provisional records; historical failures retained.
Same card `t_67778feb`, branch `aperture/issue-17`, existing **draft PR46**.
Base `7dc933a`; package reviewed at `f4a02a2a05bce7ea6b22a66c60568256327f7ef5`,
then privacy repair independently approved at
`41d55ee87bbd1de200b40df92ea06d80aecf9516`. Final evidence-only head and its own
exact-head CI are recorded in the attached compact CI capture and card handoff,
not inherited from an older commit. No merge/master push/closure/successor unlock.

## What is new
**Vivien's Grizzly** — `{2}{G}`, Bear Spirit2/3, exact `{3}{G}` ability: look at top
card; may reveal/take a creature or planeswalker; otherwise bottom. Immutable printing
`37f2571b-6756-462c-9e71-3121fa458160` (WAR182), Oracle id
`6dbd0764-4bfe-4a58-905d-aeee70ceb798`, retrieved `2026-10-27T03:25:27.464708+00:00`.
Raw `slice20/source-raw.json`; normalized `fixtures/starter-corpus/slice20-sourced.json`.
Native live readback matches recorded text, characteristics, ids and legality
(`slice20/native-verification.json`).

Reusable `Effect::LookTopTakeOrBottom` / anchored `Pending::TopCardChoice`: card stays
in its library list; no new GameState field or removed holding. Owner-only viewed
identity for eligible AND ineligible cards, eligible optional take/bottom, ineligible
mandatory bottom acknowledgement. Forged land-take refused unchanged. Only taking
reveals publicly. Empty-library look is not a draw/loss; save/reload resumes exactly once.
Stack validation counts real spell containers, not ability-source references, allowing
multiple same-source abilities without admitting malformed spell containers.

## First cut was NOT accepted
Native independent probe submitted two offered/paid activations at `cb7a18a` and
Game::from_state rejected that legal state. Actual RED101:
`slice20/native-two-activations-red.log`; probe `native-two-activations-probe.rs`;
repaired conformance test AND independent probe GREEN. Native also found ineligible
looked-at identity was discarded without the owner's look. Writer repaired it but left
that engine hunk UNCOMMITTED after reporting `2acef3d` committed; native caught and
committed the already-tested hunk in `37f7cdb` before publication/review. No dirty-head
approval is claimed. Tautological library assertion replaced with exact order,
hand zone/ownership and fully drained resolution. Native corrected README/docs current
counts. Unsupported-registration RED and retrospective strengthening are distinct;
not blanket strict-TDD compliance.

## Verified by execution
- Native **125passed/0failed in8focused suites**, including9slice20 plus separate
  discriminating double-activation probe (`slice20/native-focused.log`).
- Native command-owned exits0: fmt, workspace all-targets Clippy-Dwarnings, locked
  release CLI, separate release-CLI replay, corpus audit (`native-gates.sh`/`.log`).
  Initial evidence-log trailing-blank check failure retained locally, then corrected.
- Native REBUILT AND RAN exact4seat/40life/40card package driver: terminalP3,
  **2064actions/46turns/5281events**, digest
  `72cd69fb1143763fcd8fe1387b9a625c896538bdd389f33958d8705d8c7cdd48`;
  Vivien cast6/activated22/take5/voluntarybottom5/mandatorybottom16. Fresh save/replay
  assertions in driver; separate releaseCLI reproduces digest. Rerun leaves all three
  generated artifacts unchanged. Raw `slice20/native-journey.log`.
- Package `decks/package-slice20.txt` SHA256
  `12e6d1e92cdfb56e159e7b96465db2d03a1f511337f2dbca31371521e554916f`.
  Evidence-only MASKED demonstrator, NOT product AI/strength; Jasmine Boreal injected
  all4seats. NOT a desired100card certificate.
- ONE independent practical review on TRUE ORIGINAL reviewer history:
  `slice20/REVIEW20.md`, **APPROVED_INCREMENT** at f4a02a2, no blocking findings.
  Reviewer ran focused tests, release replay, saved-game replay, snapshot hash/fmt/diff.
  No repeated historical/full workspace matrix locally; final exact-headCI remains owed
  until attached final capture is green.

## Post-review privacy correction — earlier approval superseded
After the first review, native proved a real hidden-information leak at78dea97:
although card identities were masked, the opponent's pending description revealed
whether the looked-at card was eligible (creature/planeswalker vsland). Unequal hidden
library fixtures produced different opposing fullPlayerViews and masked digests.
Actual RED101 retained at `slice20/native-eligibility-privacy-red.log`.

Repair `41d55ee` makes TopCardChoice pending details owner-only, alongside the existing
APNAP private-choice filter. Permanent `issue17_slice20_privacy.rs` compares entire
opposing projections/digests, not just empty identity fields. Native reran9slice20+1
privacy test GREEN; writer additionally projection4/replay6/fmt/Clippy and reran the
journey with unchanged72cd69fb digest. Product change is only3view lines; no engine,
card, UI, AI-policy or save-shape change. Actual committed hunk and clean product diff
verified before review. SAME original reviewer re-adjudicated exact repaired delta:
`slice20/REVIEW20-PRIVACY.md`, APPROVED_INCREMENT at41d55ee,14focused tests
(privacy1/slice20nine/projection4), unchanged release replay and fmt/diff pass.
This supersedes the earlier review's missed leak.
Do not treat f4a02a2/78dea97 alone as final acceptance. This checkpoint's older125test
receipt remains historical; the privacy repair has its own10focused-test receipt.

## Original gate still open
Native audit **137catalog;459union=125supported+334missing**. Per-list gaps
`55/0/51/48/28/52/52/53`; all desired libraries99cards+named commander, desired snapshots
unchanged, manifest gap metadata only. **1of8desired starters certified**, Ayula28missing.
No planeswalker in pool, so that eligibility branch is code-inspected only. Reuses
AnswerTriggerChoice/pending-kind with correct legal labels. `crates/ai/tests/common.rs`
new-pending exhaustive arm ONLY, **no policy change**. Slice19 correction now discloses
`crates/ai/src/search.rs:724` clone / `crates/ai/tests/common.rs:80` Default init,
**no policy change**. No UI/import/search/model-policy changes, arbitrary Archidekt,
full Commander or v1 claim.
Bulk registry: all3isolated workers exited; processed/candidate outputs UNINTEGRATED,
zero release-enabled/certified cards. Do not recruit replacement team/auto-enable packs.
Children inspected: #26scheduled, #32todo/unassigned; still gated/untouched.

## Resume the SAME unfinished work
- writer `evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl`
- reviewer `evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl`
Complete trees backed up before run (`slice20-backup`) and after review
(`slice20-final-backup`) with byte-equality verification; all workers settled.
Preserve approved1–20, no re-audit/newteam. Current-lane target only/no retired builds,
cleanup or provider-global changes. Next sourced remaining Ayula/mechanic package toward
one whole genuine100card certificate (e.g. temporary artifact animation or conditional
regeneration); library-shell cards beyond Grizzly remain unsupported.
Existing continuous supervisor owns SAMEcard/PR/branch/history renewal. Checkpoint is NOT
human/quota block. Full8list acceptance and certified downstream gates remain mandatory.
