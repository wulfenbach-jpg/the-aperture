# Independent integration review — CHANGES REQUESTED

Candidate: `5f9422e2f7b5a6bb68e4b270b22062d7c3f91245`, PR56. This is a review of the combined integration, not full #28 acceptance. No implementation files edited, no push/merge/issue closure.

## Verified controls

- Working tree initially clean; local HEAD, remote `aperture/issue-28`, and PR head agree. PR OPEN/draft, mergedAt null; remote master remains f3e8bb16a173726247313319980616c7bd502985.
- Fresh `gh pr checks 56`: 10 pass, none pending/failing, across runs 40471618150 and 40471625044.
- `cargo test -p aperture-ai --test review --test review_honesty --locked`: exit 0; ranking 16 passed/1 ignored, honesty 27 passed.
- `cargo test -p aperture-ui review --locked`: exit 0; 27 library tests and 1 study integration test passed. Includes renderer, stale-view, no-mutation, and rendered viewport journey checks.
- Recounted `#[test]` in tracked Rust files under crates/: 48e3334=586, HEAD=636. `git diff b77603b HEAD -- crates` empty: retained gate code matches candidate. `git diff --check 48e3334 HEAD` exit 0.

## Required repairs

1. **Lattice ties rejected by integrated honesty audit.** `crates/ai/src/review/honesty.rs:1229` demands exact score equality for AmongEquals, but canonical rank uses §4.5 lattice inseparability. Feed rank rows `(offer, mean, samples) = [(0,1.0,2),(1,0.75,4)]`, exactly the existing sub-lattice control. It produces AmongEquals [0,1]; audit_review rejects offer 1 because its score is 0.75 rather than 1.0. Reconcile the audit with canonical lattice semantics without changing rank into exact-equality-only ties. Require actual rank output to pass audit for exact ties, unequal lattice ties, separated and single-sample controls. Update §4.10's exact-equality wording accordingly. The UI/notes also call these unequal means 'equally scored'; distinguish effective ties from exact equal scores while retaining a positive regression for the tie qualifier.

2. **Renderer drops interleaved single-sample rows.** `crates/ui/src/review.rs:384-405` forms a noncontiguous tie group then advances index to its last member, skipping intervening nonmembers. Rows `[(0,1.0,2),(1,1.0,1),(2,0.75,4)]` produce tie 0↔2 and SingleSample 1. The rendered candidate list contains offers 0 and 2, but no `[offer 1]` row. Render every scored candidate exactly once, preserving the explicit single-sample/not-ordered disclosure and its line/reasons.

3. **Overlapping ties duplicate a row.** The same loop assumes lattice ties partition the list, but pairwise inseparability is not transitive. Rows `[(0,1.0,4),(1,2.0/3.0,3),(2,0.5,4)]` produce 0↔1, 1↔2, not 0↔2. Renderer prints `[offer 1]` twice. Handle overlapping ties without omission/duplication or claiming the separated endpoints are pairwise equal; retain all pairwise uncertainty facts. The means in these probes are achievable sums of {-1,0,+1} samples.

## Reproduction and evidence

Reviewer-only crate/source/lockfile and raw verified RED transcript are retained in `review-round1-repro.zip` beside this report. The probe copies the candidate's existing ranking tests (including their fixture and synthetic_analysis helpers), then appends three behavioral assertions. Product source is unchanged. Extract under `target/issue28-review-probe/` in this checkout; run:

```bash
CARGO_TARGET_DIR='C:/AI/aperture-worktrees/issue-28/target' cargo test --offline --locked --manifest-path target/issue28-review-probe/Cargo.toml reviewer_ -- --nocapture --test-threads=1
```

Actual exit: 101. Three assertions failed: canonical rank output rejected by audit, interleaved candidate omitted, overlapping tie member printed twice. The initial hand-selected-rustc harness failed to compile due to incompatible cached crate variants; that is NOT RED evidence. The final Cargo harness compiled and all three failures above are behavioral. Its isolated lock resolution differs from the workspace lock; regular workspace tests above used `--locked` and remain the ordinary controls.

Criterion 4 remains blocked on #17; full_issue28_accepted=false. Interactive latency and real-game tie/single-sample journeys remain unverified, not newly claimed. Correct these integration defects on the same card/branch/PR, rerun focused controls and exact-head CI, then return to review. No new PR or merge authorized.
