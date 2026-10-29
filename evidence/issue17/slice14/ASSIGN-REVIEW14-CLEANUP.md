# Slice14 cleanup delta confirmation — same reviewer history
Tiny 5min adjudication of ONLY 472aeba..eb516e4 in same checkout; you already APPROVED_INCREMENT 472aeba and listed its nonblocking items. Writer/code state: four focused changes:
- Removed dead public CardDef::{mana_choices,creature_mana}; runtime behavior unchanged because all consumers use CardId accessors and constructors only set None.
- docs/issue17-starter-corpus.md: replaced stale Commander's-Sphere-unsupported paragraph with the four slice14 mechanic description; corrected Elas missing count53->52.
- README supported-pool count103->126 and catalog.rs module header106->126 to match the measured catalog.
- Slice14 handoff prose corrected (10 tests, Sphere implemented).
No logic/refusal/timing change. `cargo test` slice14 10, slice13 4, tokens10, catalog5 all pass; fmt/clippy-deny-warnings/releaseCLI exit0; diff-check clean. Exact new head eb516e457df97a56a6113b1abbb54a11f839d905; CI pending. Confirm no behavior regression / no accidental deletion of live metadata in ONE short paragraph appended as evidence/issue17/slice14/REVIEW14-CLEANUP.md with APPROVED_INCREMENT or REQUEST_CHANGES and exact head. Do not re-run full matrices or edit code/tests/docs; no merge/closure/full#17 acceptance.
