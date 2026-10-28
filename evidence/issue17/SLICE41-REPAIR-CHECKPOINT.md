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
replay_digest 6, issue17_slice12 8, rules_conformance 5; fmt/clippy/release exit 0.
Release CLI replay of slice41 journey exit 0, digest `d646deedfac9`, matches recorded.

## Remaining
- No new forged duplicate eligible/chosen `Pending` validator control; existing
  duplicate-choice action refusals remain.
- No full workspace matrix. Repair certifies no list; Ayula 12 missing.
