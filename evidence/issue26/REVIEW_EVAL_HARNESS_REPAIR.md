# Independent repair adjudication — PR #52 (draft), issue #26 harness

Reviewed head: `c7990ba1489d28906521d9092524ef02e3acef14`

- Local `git -C C:/AI/aperture-worktrees/issue-26 rev-parse HEAD` -> `c7990ba1489d28906521d9092524ef02e3acef14`.
- Remote `gh pr view 52 --repo wulfenbach-jpg/the-aperture --json headRefOid,isDraft` -> same SHA; `isDraft=true`, base `master`, branch `aperture/issue-26`.
- Local == remote. Before this report was written, the worktree had no tracked changes and no untracked files (`git status --porcelain` empty, `git diff --exit-code` 0); the only new untracked file is this report. No tracked file was modified by this review.
- Original report `evidence/issue26/REVIEW_EVAL_HARNESS.md` is preserved unchanged.

Scope reviewed: the repair diff `01111e7..c7990ba` in `crates/eval` plus the regenerated `reports/eval` evidence and the committed repro artifacts. No source changes outside `crates/eval`; no engine/masking/AI/UI/model audit was repeated.

VERDICT: BLOCK

One evidence-integrity defect remains: the committed report pins the wrong driver commit. All originally blocking functional findings (B1, B2, B3, B5) and the deterministic-mode part of S6 are otherwise resolved and independently reproduced.

## B1 — Lint/compilation repair: PASS

Commands:

```
cargo clippy -p aperture-eval --all-targets --locked -- -D warnings   # exit 0
cargo fmt -p aperture-eval -- --check                                # exit 0
gh pr checks 52 --repo wulfenbach-jpg/the-aperture                  # all pass
```

Observed: local clippy and fmt clean; CI `rust (ubuntu-latest)`, `rust (macos-latest)`, `rust (windows-latest)`, `windows package`, and `board and roadmap` all pass on the exact head. The previous `unnecessary_cast` at `runner.rs:299` is gone (`winner` is now recorded directly).

## B2 — Scoring/statistic labels and 0.30 parity proof: PASS

Production path:

- `GameRecord` outcomes are converted to `score - FAIR_ODDS` before entering `diffs_by_seed` (`report.rs`, `summarize`; `FAIR_ODDS = 0.5`).
- Fields are explicitly centered-difference: `paired_mean_diff_fair_odds`, `paired_ci95_low_diff_fair_odds`, `paired_ci95_high_diff_fair_odds`.
- The headline uses `lo > 0.0` on that centered difference; no raw-score branch remains.
- `PairedSample::mean` documents "Callers must pre-subtract 0.5"; the test `raw_scores_below_fair_odds_give_negative_diffs` checks a 0.30 raw score maps negative.
- `a_raw_score_of_point_three_is_never_above_parity` passes: 3 wins / 7 losses on 5 paired seeds; independent probe of the same records gives per-seed mean `-0.2`, CI `[-0.539989, 0.139989]`, `lo < 0`, headline does not contain "above parity".

Observed committed report: `paired_mean_diff_fair_odds = -0.5`, CI `[-0.5, -0.5]`, headline "search did not exceed parity". The raw report no longer exposes a `paired_mean` field that could be read as an above-parity raw score.

Residual (non-blocking): `stats.rs` still describes `PairedSample` as "per-seed mean score" in the struct comment, although the production values are centered differences. The production fields/headline are consistent; clean up the stale comment to remove the last mixed label.

## B3 — Unverified terminal replay exclusion / CLI failure: PASS

- `outcome_for` now accepts `replay_verified` and returns `NotAResult` when `status != "terminal" || !replay_verified` (`report.rs`). Unverified terminal games are excluded from `diffs_by_seed`, `terminal_search_wins/losses/draws`, `wins_by_seat0_strategy`, and seeded units.
- `summarize` withholds the CI outright when `replay_mismatch > 0`; the headline becomes "report invalid ... WITHHELD".
- Focused test: `cargo test -p aperture-eval --lib unverified` -> `unverified_terminal_replays_never_score ... ok` (2 mismatches ignored; 2 verified wins retained; seed 102 excluded; CI `None`; headline contains `WITHHELD`).
- Real CLI probe: tampering one stored replay's `final_digest` and running `cargo run -q --release -p aperture-eval -- verify <tampered>` -> `19 verified, 1 mismatched`, error `digest a160... != recorded 000...`, exit 1.
- `run` now bails after writing the report when `report.summary.replay_mismatch > 0`; the mismatch is a CLI failure, not just stderr.

Residual (non-blocking): when a mismatch exists, the headline says "every scored outcome and the confidence interval are WITHHELD", but `terminal_search_wins/losses` and `paired_mean_diff_fair_odds` remain populated from verified games. The unverified games do not contribute (test confirms), and the CI is withheld, but the headline overstates what was withheld. Either change the headline to "CI/headline withheld; verified counts retained" or null the verified aggregates too. This is a wording/consistency defect, not a scoring leak.

## B5 — Rejected policy action: PASS (with a status-shape note)

- `runner.rs` rejection branch no longer submits `Action::Pass`. It sets `truncated`, `failed = true`, `policy_rejected = true`, and breaks. The game record gets `status="failed"` and `policy_rejected=true`; `summarize` counts `policy_rejected` and excludes the game through `status != "terminal"`.
- Focused test `policy_rejected_games_are_failed_and_never_scored` passes: `failed=1`, `policy_rejected=1`, `terminal=1`, only the verified terminal game scores; headline says "no strength statement".
- `verify` rejects reports where `summary.policy_rejected > summary.failed`.

Note (non-blocking): the distinct marker is the `policy_rejected` boolean/counter; the `status` string remains the generic `"failed"`. This satisfies the original minimum ("failed/policy_rejected or count and exclude"), but if "distinct status" is intended literally, add `"policy_rejected"` as a status value.

## S6 — Deterministic mode / environment pin / regenerated report: BLOCK

Deterministic behavior itself passes:

- `search_max_millis: 0` is mapped to `u64::MAX` in `run_game`; `config.rs`, `main.rs` and `README.md` document that 0 is deterministic node/rollout mode and positive values are wall-clock/non-re-derivable.
- Committed config: `search_max_millis: 0`, `max_millis: 0`, node 1200, rollout 200.
- I copied the committed config with `seeds=[101]` only and ran:

```
cargo run -q --release -p aperture-eval -- run <tmp>/config.json <tmp>/out1
cargo run -q --release -p aperture-eval -- run <tmp>/config.json <tmp>/out2
```

Observed identical results in out1/out2, including replay action arrays:

| seed/rot | status | winner | steps | digest | out1 vs out2 | vs committed |
|---|---|---|---|---|---|---|
| 101/0 | terminal | 1 | 396 | `a160cd1d795e2bd96edb18ddf92b18b5120ff88cc8fece760b5081c0486a4eac` | equal | equal |
| 101/1 | terminal | 0 | 321 | `57bfbb378e9f3f5f6b1710b958a49027fe2bc6eed799ef73b7269c7763f9e91e` | equal | equal |

The committed report's 20/20 replay digests also verify via `aperture-eval verify` (20 verified, 0 mismatched). The committed report summary is internally consistent: 20 terminal, 0 capped, 0 failed, 0 policy rejected, 0 replay mismatches; `paired_design=true`; `paired_units_seeds=10`; `paired_scored_instances=20`; `excluded_seeds=[]`; mean diff -0.5, CI [-0.5, -0.5]; no strength claim. Fixture hashes match the current files; raw config hash `3b8a289e...` matches `config-insha256.txt` and the report's `config_input_sha256`; canonical hash `d701869c...` matches the report's `config_sha256`.

**Blocking finding — the report's commit pin is stale and contradicts the claim of a "full ... commit pin":**

- `git rev-parse HEAD` at the reviewed worktree: `c7990ba1489d28906521d9092524ef02e3acef14`.
- `reports/eval/practice-calibration-paired-seeds-v1/report.json` and `SUMMARY.txt`: `hardware.git_commit = 01111e7274301eb7dd7565e4cc826e7660b37f9a` (and "driver commit: 01111e7...").
- `evidence/issue26/repro/run-A/report.json` pins `01111e7` as well; the `run-A`/`run-B` logs are the corresponding pre-commit runs, and the committed `reports/eval/...` report is run-B's output.
- The report is `schema: 2`, uses `search_max_millis: 0` deterministic mode, and has the new fields; that code does not exist at `01111e7` (the original review head). Reproduction: `git show 01111e7:crates/eval/src/report.rs | grep 'schema: 1'` -> `schema: 1`; `git show 01111e7:crates/eval/src/config.rs | grep 'search_max_millis must be positive'` -> the old validation that rejects 0. My independent runs at `c7990ba` reproduced the exact committed seed-101 digests but recorded `git_commit = c7990ba...`.

Observed vs expected:

- Observed: report/summary pin `01111e7`; actual code that produced it and reproduces it is `c7990ba` (run from a dirty pre-commit tree).
- Expected: the evidence pin should name the code commit that was executed (`c7990ba` or the eventual merge commit) or record an explicit dirty-tree/tree hash. As published, checking out `01111e7` cannot reproduce the report; the metadata is false.

Minimum fix (no production-code change required): regenerate `reports/eval/practice-calibration-paired-seeds-v1` at `c7990ba` (or after the new-master rebase) so `hardware.git_commit` matches, update `SUMMARY.txt` and the repro logs, and commit. To prevent recurrence, add a `git_dirty` flag or `git rev-parse HEAD^{tree}` / dirty tree hash to `HardwarePin`, and refuse or prominently label evidence produced from a dirty tree. Re-run `verify` on the regenerated report. Because this is an evidence-integrity/acceptance claim, it is blocking even though the games themselves reproduce exactly.

## Named smaller corrections

- **Paired rotation validity: PASS.** `paired_design = rotations >= 2`; `single_rotation_is_not_a_paired_design` passes and withholds the CI with an explicit "NOT a paired design" note.
- **Scored denominator / excluded seeds: PASS.** `paired_scored_instances` is the number of scored verified terminal games (20 in the committed run), not `games.len()` including non-results; `excluded_seeds` names declared seeds with no scored game. The committed run has no exclusions. `verify` checks the declared seed x rotation plan and missing-game counts.
- **Typed pending / search-fallback diagnostics: PASS.** The driver no longer synthesizes a pending note; it calls `search()` for the search seat, which returns typed `SearchUnavailable` reasons (including the real pending kind), and records `unavailable.label()` in `fallback_notes`; source labels distinguish search vs `heuristic (search-seat fallback)`. The committed report contains 48 typed `pending kind ...` notes (mulligan, declare attackers/blockers, order card), 20 stack-busy notes, and zero `"eval driver: pending decision"` strings.
- **Latency split: PASS.** `wall_ms` is captured before replay verification; `verify_ms` is separate. Observed committed games have small `verify_ms` values; independent reruns confirm the field is not part of game wall time.
- **README/command: PASS.** `crates/eval/README.md` documents the exact release command from the repository root and the deterministic-mode meaning.
- **Residual non-blocking:** `verify` does not detect duplicate `(seed, rotation)` records (it overwrites the plan map; a duplicate is not rejected). The plan/missing checks otherwise work. Add duplicate detection.
- **Residual non-blocking:** `terminal` counts unverified terminal records, while scored counts exclude them; the mismatch headline wording above is the only place this is confusing. Consider labeling `terminal_unverified` separately.

## Test/verification log (reviewer-run)

- `cargo test -p aperture-eval` -> 10 lib tests passed; 3 integration tests passed (driver + deterministic rerun; 179 s).
- `cargo test -p aperture-eval --lib unverified` -> focused B3 test passed.
- `cargo fmt -p aperture-eval -- --check` -> exit 0.
- `cargo clippy -p aperture-eval --all-targets --locked -- -D warnings` -> exit 0.
- `cargo run -q --release -p aperture-eval -- verify reports/eval/practice-calibration-paired-seeds-v1/report.json` -> 20 verified, 0 mismatched, 20 declared, schema 2, exit 0.
- Tampered replay digest -> 19 verified, 1 mismatched, exit 1.
- One-seed deterministic reruns (release, seeds=[101], committed config otherwise unchanged) -> out1/out2 identical digests and replay actions; both match committed seed-101 lines; `git_commit` in those runs is `c7990ba`.
- `gh pr checks 52 --repo wulfenbach-jpg/the-aperture` -> all checks pass on `c7990ba`; PR remains draft.

## Gate and integration

**This repair does NOT satisfy the original issue-#26 acceptance requirement of at least 1000 paired games/rotations on #17's CERTIFIED eight-list corpus with the certified hardware/budget protocol. The committed evidence is a 20-game practice calibration on two practice decks and is correctly labelled as such. New-master integration/rebase is separately owed before merge.**
