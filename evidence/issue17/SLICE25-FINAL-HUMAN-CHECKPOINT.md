# Issue17 — slices24–25 final reviewed handoff (original eight-list contract is unfinished)

## Current candidate

- Same native card `t_67778feb`, checkout `C:/AI/aperture-worktrees/issue-17`, branch `aperture/issue-17`, draft PR46.
- Published and independently reviewed head: `ea106932d649da6876004fdf5c1a2ce6e441d19c`. Code commit `823680d`; native test/docs/evidence follow-up `ea10693` adds no engine change. Local/remote/PR head readbacks match.
- Slice24 independently approved head `3b06249d8bfc3f58cfa454e30ed2be417e23639f` has its own 10/10 exact-head CI SUCCESS and all required contexts PASS. Its full checkpoint is `SLICE24-FINAL-HUMAN-CHECKPOINT.md`.
- **Slice25 final exact-head CI: 10/10 completed SUCCESS**, every head_sha `ea106932d649da6876004fdf5c1a2ce6e441d19c`, captured2026-10-27T09:42:23Z in `slice25/exact-head-ci-ea10693.json` / summary. All four strict required contexts PASS with configured Actions app15368. PR body updated and read back exactly with current scope and this receipt. No inherited green; final checkpoint/review/CI files stay outside the branch tip so CI remains bound to the exact published head.
- No merge, master push, issue closure, downstream activation, new team, provider/global-role change or purchases.

## What became usable

**Surrak and Goreclaw (slice24)**: full printed trample, trample to other controlled creatures, and own other nontoken entry gives that exact entered permanent one +1/+1 counter and haste until cleanup. Reusable static trample and temporary haste, including actual creature-mana offers. First cut was rejected: native tests proved the mana offer missing and a departed recipient deleting the whole mandatory trigger. Both fixed; the trigger remains but its incarnation binding invalidates. Same original independent reviewer approved3b06249. Exact source MOM337 `9c10934d-9016-43c4-a7ab-56cc7d8f671f`, retrieved2026-10-27T08:44:35.433520+00:00; raw, stamp and normalized fixture retained.

**Exuberant Wolfbear (slice25)**: full optional attack trigger targets a Human you control and sets base power/toughness to the source's effective P/T until end of turn. Reusable layer7b setter applied before counters and layer7c modifiers; first-departure source P/T snapshot provides last-known information. Exact IKO151 printing `8c8ff0da-fbe3-4c95-bc39-c975c55b6aea`, oracle `40f5f8b9-5c2c-41d4-8b83-55a2e51ced64`, retrieval stamp in `slice25/source-retrieval.json` (2026-10-27T09:19:21Z). Full response and normalized fixture retained.

Native added three post-implementation GREEN-strengthening tests (not retroactive TDD): real Giant Growth responses after the trigger prove current-resolution P/T, base7/7 plus target counter+pump becomes11/11 and cleanup restores3/2; pumped source bounced with actual Unsummon supplies7/7LKI; multiple own Human offers reject non-Human, opposing Human and wrong-seat submissions unchanged, then accept the valid target. Earlier writer test name claimed a forged-target assertion it did not contain; native renamed it and supplied a real test. No engine repair was needed for these controls. Registration REDs are retained in original histories; no blanket strict-TDD claim.

## Executable verification

- Native `slice25/native-focused.log`: **100 passed**, no failures (abilities13, printings8, catalog5, commander43, corpus2, slice24 10, slice25 7, timing12).
- `slice25/native-gates.sh` / `.log` record each command's own exit: fmt0, workspace Clippy-Dwarnings0, locked release CLI0, rebuilt journey0, separate replay0, independent each-list audit0. The evidence-only driver has an unused-import warning; workspace Clippy is clean.
- Native rebuilt/reran the real exact four-seat/40-life/40-card package `decks/package-slice25.txt`, seed2323: terminal P3, **1760 actions /41turns /4471events**, Wolfbear8casts /9attacktriggers /9base-P/T-setevents. Digest `4d6ea58ee735c74a89e142eca90656dbf22faeaf49c2de0fb33c0ec9952c5d52`; separate release CLI replay matches; fresh save roundtrips. Snapshot SHA256 `1bd229439656be174278cd958eb7083ec495c43be3eefa8a17d4ee6fff881a0d`. Driver uses masked offered actions; Jasmine Boreal is in all four command zones. **Not product AI, playing-strength evidence or a desired 100-card starter certificate.**
- Previous slice24 replay still verifies with a41bbfd2 at the new engine; one representative continuity check, not another historical matrix.
- SAME exact original reviewer history `evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl` returned **APPROVED_INCREMENT at ea10693**, ran7focusedtests and separate release replay; inspected actual delta and native evidence. Report `slice25/REVIEW25-ORIGINAL-HISTORY.md`. Slice24 approval preserved; no blocking finding. Earlier approved1–23 not re-audited.

## Remaining acceptance and limits

Independent native EACH-list audit: **142 catalog /459 unique desired names =130 supported +329 missing**. Per-list missing `55/0/51/48/23/52/52/53`. Only **1 of8** desired 100-card lists is certified (Jasmine); Ayula still23short. Desired deck snapshots and certification statuses unchanged. Full original #17 acceptance remains false; arbitrary Archidekt compatibility remains a non-goal. Bulk processed/candidate lanes remain unintegrated, no automatic enabling.

Slice25 limitations: no reachable mid-optional save/reload, dedicated target departure/recast or source-recast-overrides-LKI regression; these are code-inspected, not newly executed claims. Slice24 pending-reference validation and granted-keyword projection limits stay disclosed. No UI/import/AI-policy change; legacy trigger/stack literal initialization only. No full workspace rerun in these increments; exact-head CI owns the required gate. #26 remains scheduled on certified inputs; #32 remains unassigned/todo/gated; both inspected and untouched.

## Histories and next action

Both original writer/reviewer histories and full referenced directories preserved: initial59files backed up and SHA-verified before work, final **62files** byte-verified in `evidence/issue17/slice25-final-backup`, manifest `slice25/history-backup.json`. All assigned OMP processes exited (verified); no competing writer/reviewer.

Exact-head CI and PR readback are finished. Continue sourced missing mechanics on SAME card/PR/branch/original histories after this reviewed increment checkpoint; preserve approvals1–25 without re-audit. The original eight-list contract is still unmet, so return to implementation rather than completing #17. Do not merge, push master, close issues or unlock certified downstream work. No new human/quota block or replacement team is needed. For later exact-head captures, the reusable command is `python evidence/issue17/slice24/capture-ci.py <40-character-head> <slice-output-directory>`; it fails until all10 checks succeed and required contexts match Actions app15368.
