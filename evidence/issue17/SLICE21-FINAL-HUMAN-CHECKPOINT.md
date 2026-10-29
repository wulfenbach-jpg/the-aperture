# Issue17 slice21 — final reviewed rolling checkpoint

Same card t_67778feb / branch aperture/issue-17 / existing draft PR46.
Base cdfc32e; implementation8088ba5, accounting/citation repair412f63a,
native evidence d9d2c94, identity repair **f59d29be91b77a9f234d268d438f42039915213d**.
Independent SAME original-history reviewer APPROVED_INCREMENT at f59d29b
(`slice21/REVIEW21-REPAIR.md`); earlier CHANGES_REQUESTED at d9d2c94
(`slice21/REVIEW21.md`) preserved. Final evidence-only tip must have its OWN
exact-head CI capture; do not inherit green from a prior commit.
No merge/master push/closure/certified-downstream unlock. Full #17 UNFINISHED.

## Usable addition
**Dragon-Scarred Bear**, DTK183, {2}{G}3/2 Bear. Full printed Formidable
{1}{G} regeneration, repeatable/no tapcost, current controlled creature power>=8
checked at activation only. Typed destruction-replacement shield, not damage
prevention/indestructibility: next destruction removes damage, taps and removes
from combat; each shield used once; expiry at cleanup. Sacrifice, exile, bounce,
zero toughness are not destruction. Shields end on actual battlefield departure.
Immutable source689a6390-894a-4d31-87c2-f9cec3b187a7, oracle2f6f49a4-f2ea-4147-a6d3-69cf875c302e,
retrieved2026-10-27T04:44:50.249896+00:00; raw+retrieval in slice21/,
normalized fixtures/starter-corpus/slice21-sourced.json. Native fetched source;
printing tests match exact cost/type/P/T/text.

## What changed after inspection/review
- Wrong Formidable citation702.102a (Fuse) corrected to207.2c/602.1b/602.5.
- Driver preferred-branch counter claimed9activations; actual authoritative events
  prove25. Corrected accounting:25activations/25shield creations/1regeneration.
- Old death order and no-regeneration event wording preserved, old replay verifies.
- **Real blocking defect found independently:** noncommander ObjectIds persist
  through bounce/recast, so id-keyed shields transferred to a new incarnation.
  Initial synthetic-new-object test missed it. Review RED retained at
  slice21-review-probe.log. Repair invalidates shields on canonical actual BFexit;
  real offered SAME-card recast+Shock now dies, not regenerated. Failed head was
  never approved. No global identity rewrite or extra serialized field.
- Initial registration RED preceded production (original transcript extracted in
  slice21/registration-red-original-history.json). Most later tests were GREEN
  strengthening, not blanket strictTDD. Native's earlier mistaken chronology
  inference explicitly corrected on card. No dedicated queuedsourceleft test:
  that guard is code-inspected only, despite writer stdout overclaim.

## Executed evidence
Native initial **68passed/0failed** across affected suites, fmt/workspace
Clippy-Dwarnings/locked release, separate CLI replay, corpus audit: all8command
exits0 (`native-gates.log`). Native repair rerun8slice21+1cleanup and replay pass
(`native-identity-green.log`). Reviewer initial57focused+1cleanup; repaired9focused,
new+old release replays, no blocking finding. Reuse unaffected evidence, no repeated
whole-workspace local matrix. Empty-skipped state field; authentic slice20 replay
still72cd69fb1143 at repaired code.
Native REBUILT/RAN actual4seat/40life package, seed1919:
**1906actions/45turns/4909events**, terminalP3, digest
0d20c7597183ecb764f7b858dd87ca9aa654e3a4ac6984bada01e9df27926520.
Bear cast7/activated25/shields25/regenerated1; separate replay+fresh save match.
Exact package SHA256f48f7a9011ac005858ce26b71e9313751694d0773797c815c5158480fb880003.
Evidence-only offered-action demonstrator with public battlefield/shield
instrumentation, no hidden zones; NOT productAI/strength. JasmineBoreal injected
all4seats; **40-card package is NOT another100-card starter certificate**.

## Original acceptance still open
**138catalog;459union=126supported+333missing;Ayula27missing.**
Per-list55/0/51/48/27/52/52/53; all desired snapshots unchanged.
**Only1of8desired starters certified.** Equipment/soulbond/hideaway/saddle/Ring,
other library/cost/type primitives remain; arbitraryArchidekt explicitly non-goal.
AI touches search.rs:727 and tests/common.rs:83 are mechanical initialization only,
**no policy change**. No UI/import/AI-policy changes. Bulk lanes remain unintegrated
processed/candidates,0release-enabled/0certified. Children #26scheduled/#32todo,
still gated, inspected/untouched. No fullCommander/v1/release claim.

## Continue SAME work, no human/quota block
writer evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl
reviewer evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl
Preserve approved1–21 and both complete trees/untracked artifacts; backup verified
before run(slice21-backup) and final(slice21-final-backup). No competing worker.
Existing enabled15min supervisor a60a00413961 owns next SAMEcard/PR/history renewal.
Next: sourced missing Ayula mechanics toward genuine whole100card certificate;
no re-audit of approved increments, no replacement bulk team or autoenabled packs.
