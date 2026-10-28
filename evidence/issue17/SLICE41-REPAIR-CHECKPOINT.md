# Slice41 repair checkpoint

- Preserved independent BLOCK: `evidence/issue17/review309-recovery/REVIEW.md`.
- Base head `e6babc6e...`; product before repair `ddd99f0`; repair local head in final receipt.
- No push/merge/closure. Original #17 remains unfinished, 1 of 8 certified, Ayula 12 short.

## Repair
`SearchLibrary` validator no longer conflates eligible and chosen membership. It validates
uniqueness **within eligible** and **within chosen** separately, while preserving live-owner,
active-resolution, library zone/owner/basic-land, eligible-set and duplicate-chosen checks.
Legitimate chosen∈eligible overlap is accepted.

## RED (actual behavioral assertion)
New `midsearch_after_one_pick_save_reload_continues_identically` compiled and failed before
the fix at `g.state.validate()`:
`InvalidState { reason: "library search chose #4 twice" }`.

## GREEN
`evidence/issue17/slice41-repair/gates.log`: issue17_slice41 10 passed; ordering_persistence 2,
replay_digest 6, issue17_slice12 11, rules_conformance 5; fmt/clippy/release exit 0.
Release CLI replay of slice41 journey exit 0, digest `d646deedfac9`, matches recorded.

## Negative validator controls (added)
`forged_search_pending_duplicates_and_subset_are_rejected_by_validate_and_save` starts from
a legal post-pick state, confirms the control validates, then forges duplicate `chosen`,
duplicate `eligible`, and chosen-outside-eligible. Each is rejected by both
`GameState::validate` and real `SessionSave` JSON loading. issue17_slice41 is 11 passed;
fmt/clippy exit 0 (`slice41-repair/negative-gates.log`).

## Native coordinator verification (not independent review)

Product fix `c3e939872f229a459a367460cfc6481f1a37a681`; negative controls
`5af167e761d8013b5601f4f73f8daf3ea0fa9721`. Native coordinator executed
`slice41-repair/native-verify.sh` at `c7f7dedb0d5e1eabb630540099745e92961f5cbb`:
issue17_slice41 11, issue17_slice12 11, ordering_persistence 2, replay_digest 6,
rules_conformance 5; fmt, workspace all-target warnings-denied Clippy, locked release CLI,
and separate journey replay all exit0. Every command/exit is retained in native-verify.log.
Writer RED extracted verbatim from the exact preserved session to writer-red-extracted.log;
extract-red.py identifies the actual failing tool result. No new RED claimed for negative controls.
Native coordinator corrected the writer's stale slice12 count (8 -> 11), and added receipts only;
all product code/tests remain authored by the saved writer. Independent review is still required.
The earlier no-push checkpoint above is the writer's exit state, not the publication receipt.

## Remaining
- No full workspace matrix. Repair certifies no list; Ayula 12 missing.
