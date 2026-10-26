# Independent review — PR #52 (draft), issue #26 evaluation-harness increment

Reviewed head: `01111e7274301eb7dd7565e4cc826e7660b37f9a`

- Local: `git -C C:/AI/aperture-worktrees/issue-26 rev-parse HEAD` -> `01111e7274301eb7dd7565e4cc826e7660b37f9a`.
- Remote PR head: `gh pr view 52 --repo wulfenbach-jpg/the-aperture --json headRefOid,isDraft` -> same SHA, `isDraft=true`, base `master`, branch `aperture/issue-26`.
- Local == remote. No tracked file was modified. The worktree already contained untracked run artifacts under `reports/eval/` (config/replays/_retired/base64); they are not part of the reviewed commit and were not used as evidence.

Scope: driver/report/replay code quality and honesty of the "practice calibration" label. This is not issue-#26 acceptance.

VERDICT: BLOCK

## 1. Scope — PASS

`git diff --name-only 1d8d439af69d7a066db35ff109c03a4257bccfb7...01111e7` is exactly 13 files:

- `Cargo.toml` (one workspace member), `Cargo.lock` (new package)
- `crates/eval/**` (new crate: manifest, config, lib, main, report, runner, stats, one integration test, one committed config)
- `reports/eval/practice-calibration-paired-seeds-v1/{SUMMARY.txt,report.json}`

No changes to `crates/core`, `crates/ai`, `crates/ui`, `crates/import`, `crates/cli`, packaging, CI workflows, or global config. The AI/search code consumed by the driver is the unchanged base revision. Scope is confined as requested.

## 2. Paired driver — PARTIAL; one blocking integrity gap

What is correct:

- `run_session` executes exactly `seeds x rotations` real `Game::new(seed, decks)` games (`crates/eval/src/runner.rs:78-86`), no retry of a run.
- Rotation 0 puts search on seat 0, rotation 1 on seat 1, so each policy plays both deck seats (`search_seat`, `strategy_for`, `runner.rs:30-40`).
- Terminal/capped/failed are separate (`runner.rs:272-278`). I verified the capped path with a throwaway config (`max_steps=5`, one seed): report had `status=capped`, `terminal=0`, `terminal_search_wins=0`, `paired_mean=null`, headline "no terminal games; no strength claim". Capped games are not counted as wins.
- The driver does not silently omit declared seeds/rotations: `games_declared = seeds.len() * rotations`, and the calibration test asserts `games_recorded == games_declared`.

Blocking gap:

- **B5. A rejected policy action is converted into `Action::Pass` and the game remains eligible to finish as `terminal`.** `runner.rs` (`Err(err) =>` branch, around lines 213-235) pushes a `Truncated` event into the local log and a note into `fallback_notes`, submits `Pass`, and never sets `failed`. The game can then be scored as a search win/loss/draw. For a policy benchmark, an illegal policy action is a policy failure, not a pass. The note is capped at five and is not aggregated in `EvalSummary`.
  - Observed in the committed run: zero such notes (so the committed numbers are not affected), but the code path is live.
  - Minimum acceptable: set a distinct `failed`/`policy_rejected` status (or count and exclude it from `scores_by_seed`), and add an aggregate counter to the summary.

Non-blocking driver observations:

- `REPEAT_LIMIT=3` stops on the third occurrence of a state digest (`runner.rs:245-253`). This is labelled `capped`, not a result; acceptable, but it is a heuristic that can end a legitimate non-looping game.
- `run_session` aborts the whole run on the first game error (`?`), producing no partial report. Fail-fast is honest but loses all evidence for a flaky game.
- `rotations=1` is allowed by `EvalConfig::validate`, which disables the seat swap while still reporting a "paired" mean. The committed config uses 2; validation should require 2 for a paired design or state the limitation in the report.

## 3. Policy separation — PASS with two labelling caveats

- The search seat selects only from `view.legal` (`crates/ai/src/search.rs`, candidate construction) and caps the list with `max_candidates` in offer order. It never invents an action.
- Pending points, busy stack, inconsistent observations, and refused reconstructions are typed `SearchUnavailable` reasons. The driver falls back to `aperture_ai::fallback(&view)`, which is `HeuristicPolicy::new().decide(view)` — the same frozen heuristic used on the heuristic seat. No heuristic or search code was modified by this PR.
- `Knowledge { own_deck }` is the only knowledge passed to search (`runner.rs:131-133`). The search reconstruction reads only view-visible object ids plus public/bookkeeping state; opponent hands and both libraries are rebuilt from the neutral belief deck/counts, and the viewer's own future draws are a deterministic shuffle of the declared list minus visible cards. The real future order is not read (`search.rs` `reconstruct`/`own_remaining_draws`).
- The heuristic seat calls only `fallback(&view)` (`runner.rs:179-181`). No hidden hands or future library order.

Caveats (not blocking):

- For pending decisions the driver fabricates the note `"eval driver: pending decision"` instead of using the actual `PendingKind` (`runner.rs:163-168`). The committed report therefore contains the generic note `pending kind eval driver: pending decision`; it is honest that fallback happened, but loses the specific reason.
- When `search()` returns `Ok` with a heuristic decision because no candidate reached a result, the driver discards `outcome.analysis` and records only the source label `"heuristic (search-seat fallback)"`; no typed per-decision reason is added to `fallback_notes`. The per-game source counts still expose it.

## 4. Replays — PARTIAL; one blocking contract violation

Verified positively:

- `Replay::run()` replays every recorded step and compares `final_digest` exactly (`crates/core/src/replay.rs:313-350`). The driver sets `final_digest` before verification (`runner.rs:280`), so the check is real, not a no-op.
- `cargo run -q -p aperture-eval -- verify reports/eval/practice-calibration-paired-seeds-v1/report.json` -> `verify: 20 verified, 0 mismatched of 20 recorded games`.
- Tampering with one replay's `final_digest` in a copy of the report produced `seed 101 rot 0: ... digest 4d49... != recorded 000...`, `19 verified, 1 mismatched`, exit 1. The CLI verifier detects and surfaces mismatches.

Blocking gap:

- **B3. A replay mismatch is not excluded from the scored outcomes.** `OutcomeClass::NotAResult` is documented as "capped, failed or replay mismatch" (`report.rs:65-69`), but `outcome_for` receives only `status` and `winner` (`report.rs:71`) and `summarize` calls it without `replay_verified` (`report.rs:159`). A game with `status="terminal"` and `replay_verified=false` is counted in `terminal_search_wins/losses/draws`, in `scores_by_seed`, and therefore in `paired_mean`/CI/headline. `run` also exits 0 after printing a stderr mismatch and writes the report.
  - Observed in the committed run: 0 mismatches, so the committed numbers are not currently false; the defect is latent and contradicts the code's own invariant.
  - Minimum acceptable: pass `replay_verified` into `outcome_for` (or skip unverified games in `summarize`), set them to `NotAResult`, and withhold the headline/CI (or fail the run) when `replay_mismatch > 0`; add a regression test with a fabricated unverified terminal record. The PR body says terminal outcomes are reported "separately from capped/failed/replay-mismatch outcomes"; the counter is separate, but the scored outcomes are not.

## 5. Statistics honesty — BLOCK (wrong parity statistic)

**B2. `paired_mean` is a mean raw score, not a mean difference from fair odds, and the parity test uses the wrong threshold.**

- `Go` scores are win=1.0, draw=0.5, loss=0.0. Fair odds in this symmetric two-seat design is 0.5 per game.
- `stats.rs:3-8` says the per-seed paired difference is the "search-minus-fair-odds" score, and `stats.rs:35-38` says "A mean above 0 means the search policy scored better than fair odds". The implementation (`PairedSample::mean`, `stats.rs:39`) averages the raw scores; it never subtracts 0.5.
- `report.rs:212` decides parity with `lo > 0.0`, not `lo > 0.5` (or `lo - 0.5 > 0`).
- In the committed report: `paired_mean = 0.1`, CI `[-0.0508, 0.2508]`. The honest paired difference is `0.1 - 0.5 = -0.4` with CI `[-0.5508, -0.2492]`; search is 40 points below fair odds, not 10 points above it.
- The committed headline happens to say "did not exceed parity", but the raw JSON field and its documentation invite the opposite reading. A future run with mean 0.30 and sd 0 (CI `[0.30, 0.30]`) would satisfy `lo > 0` and produce "paired interval lower bound above parity" even though 0.30 is far below 0.5.
  - Minimum acceptable: either score `go - 0.5` per game and keep the `> 0` test, or keep raw scores and test `lo > 0.5`, with field docs renamed; add a unit test proving a 0.30 score is not called above parity.

Other statistical findings:

- Sample size and CI method are stated (`n=10` seed units; pinned t, df=9; Wilson secondary). The t interval over per-seed means is a defensible simple correlated-unit method for this design, and the Wilson note explicitly disclaims correlation. Good.
- `paired_instances` is `games.len()` (all 20 recorded games), while the CI uses only terminal rotation games; the per-seed mean drops non-terminal games and silently drops seeds with no terminal game from `n_units` (`report.rs:184-197`). With capped/failed games, "instances=20" overstates the scored denominator and the dropped seeds are not named. The denominator note should report scored terminal instances and excluded seeds.
- `wilson_95(search_wins, terminal)` uses wins/terminal, so draws are neither wins nor half-wins; it is correctly described as a secondary win-rate readout, not the released score interval.
- Practice labelling: `PRACTICE_DISCLOSURE` is attached verbatim to the summary/report, and the headline says "practice calibration ... no strength claim". I found no place where this run is presented as an issue-#26 result. The defect is the `paired_mean`/parity semantics, not a false issue-#26 claim.

## 6. Reproducibility — BLOCK (evidence cannot be re-derived)

Committed inputs do reproduce the *stored* replays: the report embeds full replay JSON, `verify` re-runs all 20 and checks the final digest (20/20 verified), and the fixture hashes match the current files (`70aa...` Temur, `51dc...` Izzet). `config_sha256 = 131c...` is the canonical compact re-serialization of the committed config and can be recomputed.

However, the calibration itself is not reproducible:

- I created a throwaway copy of the committed config with `seeds=[101]` (all other fields unchanged) and ran the committed code twice in separate output directories:
  - `cargo run -q -p aperture-eval -- run <tmp>/config.json <tmp>/out2`
  - `cargo run -q -p aperture-eval -- run <tmp>/config.json <tmp>/out3`
- Results for seed 101, same host (12 logical CPUs, Windows, dev profile), same config/seeds:

| run | rot | winner | steps | digest | wall_ms |
|---|---|---|---|---|---|
| committed | 0 | 1 | 449 | `4d495d67...` | 39,183 |
| out2 | 0 | 0 | 514 | `55ef9cb1...` | 49,974 |
| out3 | 0 | 1 | 557 | `a781a6ca...` | 57,888 |
| committed | 1 | 0 | 365 | `d83b72d2...` | 30,473 |
| out2 | 1 | 0 | 369 | `758572e4...` | 33,459 |
| out3 | 1 | 0 | 369 | `758572e4...` | 32,257 |

Two clean identical runs disagree on seed 101 rotation 0: `out2` is a search win in 514 steps, `out3` is a search loss in 557 steps. The committed rotation-0 game is a third, different line. Rotation 1 happened to converge to the same line in `out2`/`out3`, which does not rescue the design.

The configured search budget includes a wall-clock cutoff (`search_max_millis: 250` in the committed config), and both `search` rounds and `run_rollout` stop on `Instant::elapsed`. With a fixed seed, wall-clock-dependent rollout completion changes the chosen action, so the reported games are a sample of a timing-dependent policy, not a deterministic function of `(seed, config, code)`. The report pins only OS, logical CPU count, and build profile (`report.rs` `detect_hardware`), not CPU model, compiler version, or commit; even the pinned fields match my host and the lines still differ.

- Minimum acceptable: either (a) use a deterministic budget for the calibration (set `search_max_millis` high enough that the node budget governs, or add an explicit deterministic mode that ignores wall clock and records cancellations), rerun seed 101 twice, and show identical digests; or (b) explicitly document that the search policy is wall-clock nondeterministic, stop claiming exact re-derivation, and record the full environment (CPU model, Rust version, commit, build flags) with every report. Regenerate and re-commit the practice calibration under the chosen regime. The PR body calls this a "reproducible paired-seed/rotation driver"; that claim is not true for the committed calibration as it stands.

There is also no committed README/evidence command. `main.rs` documents `aperture-eval run <config.json> <outdir>`, and the config is committed, but the exact invocation/profile used for `reports/eval/...` is not recorded in the evidence. Add the exact command and working directory.

## Blocking-finding summary

| ID | Finding | Reproduce | Observed | Expected / minimum |
|---|---|---|---|---|
| B1 | Required CI check red at exact head | `cargo clippy -p aperture-eval --all-targets --locked -- -D warnings`; `gh pr checks 52 --repo wulfenbach-jpg/the-aperture` | Local exit 101; all three `rust` jobs fail on `unnecessary_cast` at `crates/eval/src/runner.rs:299` (`winner.map(|p| p as u8)`; `PlayerId` is already `u8`). Ubuntu/macOS/Windows logs all show the same clippy failure. | Clippy exits 0 and the `rust` checks pass on `01111e7`. Remove the unnecessary cast (and any other warnings). |
| B2 | `paired_mean`/parity statistic is offset by fair odds | Read `stats.rs:3-8,35-39`, `report.rs:212`, and `python -c "..."` on the committed report | `paired_mean=0.1`, CI `[-0.0508,0.2508]`; true score-minus-fair-odds is `-0.4`, CI `[-0.5508,-0.2492]`. `lo > 0` would call any mean with lower bound >0 "above parity", including 0.30 < 0.5. | Score difference from 0.5 and test `>0`, or test `lo > 0.5`; rename docs/fields; add a 0.30-score regression test. |
| B3 | Replay mismatch is still scored | `report.rs:65-71,159`; fabricate a terminal `replay_verified=false` record | `OutcomeClass` docs say mismatch is `NotAResult`, but `outcome_for` never sees `replay_verified`; mismatch increments `replay_mismatch` and also `search_wins/losses`/paired scores. Committed run has 0 mismatches. | Pass `replay_verified`; exclude unverified games from all scores/CI; withhold headline or fail when `replay_mismatch>0`; add a test. |
| B4 | Committed calibration lines cannot be re-derived; two clean reruns disagree | Temp config = committed config with `seeds=[101]`; run twice: `cargo run -q -p aperture-eval -- run <cfg> <out2>` and `... <out3>` | seed 101 rot 0: committed winner 1/449 steps/`4d49...`; out2 winner 0/514/`55ef...`; out3 winner 1/557/`a781...`. Wall-clock search budget (`search_max_millis=250`) is a non-deterministic input. | Deterministic budget/mode; show identical digests on two reruns; regenerate the committed report; or document the nondeterminism and stop claiming exact re-derivation. Record CPU model/compiler/commit. |
| B5 | Illegal policy actions are converted to passes and can still score | `runner.rs` rejected-action branch (~lines 213-235) | On `game.submit` error the driver submits `Pass`, keeps `failed=false`, may end `terminal`, and scores the game; only a capped-note list records it. Zero occurrences in the committed run. | Mark the game `failed`/`policy_rejected` or count and exclude it; add an aggregate counter and test. |

## Non-blocking findings

- **N1. Denominator disclosure.** `paired_instances = games.len()` includes capped/failed games that do not enter the CI; `n_units` drops seeds with no terminal game without naming them. Report scored terminal instances and excluded seeds.
- **N2. Config hash label.** `EvalReport::build` hashes canonical `serde_json::to_string(&config)` (`report.rs`, `config_sha256`), while `main.rs` writes the raw file hash to untracked `config-insha256.txt`; `SUMMARY.txt` labels the canonical hash "source config sha256". Observed: raw file `3a2ed4ab...`, canonical `131c4145...`. Name both or commit the raw hash.
- **N3. Pending note is synthesized.** The driver's `SearchUnavailable::PendingDecision` uses `kind: "eval driver: pending decision"` instead of `pending_kind_name(kind)`, so the report loses the actual pending kind.
- **N4. Latency includes replay verification.** `started` is taken at `run_game` entry, but `wall_ms: started.elapsed()` is evaluated when the `GameRecord` is built after replay verification. The number is total harness time, not game-only wall clock; split the fields or document it.
- **N5. Environment/provenance pin is thin.** Hardware pin has OS, logical CPU count, and dev/release only; `engine: "aperture-core 0.1.0"` has no git commit, Rust compiler, or CPU model. Add these for acceptance evidence.
- **N6. No committed docs/command.** Only the CLI usage comment; add the exact calibration command/profile/cwd to the evidence or crate README.
- **N7. `verify` scope.** It replays stored games but does not cross-check the declared seed x rotation plan, `games_declared`, or summary counts; a report with omitted games would still verify its remaining replays.

## Test/verification log (reviewer-run)

- `cargo test -p aperture-eval` -> 5 lib tests passed; 2 integration tests passed (driver test took ~65 s). Exit 0.
- `cargo fmt -p aperture-eval -- --check` -> exit 0.
- `cargo clippy -p aperture-eval --all-targets --locked -- -D warnings` -> exit 101, B1.
- `cargo run -q -p aperture-eval -- verify reports/eval/practice-calibration-paired-seeds-v1/report.json` -> 20 verified, 0 mismatched, exit 0.
- Tampered copy of the report -> 19 verified, 1 mismatched, exit 1.
- Capped-path throwaway config -> `capped`, `terminal=0`, `terminal_search_wins=0`, headline "no terminal games".
- Seed-101 reruns -> B4 table above.

## Acceptance gate

**This review does not satisfy, replace, or advance the issue-#26 acceptance gate: at least 1000 paired games/rotations on the certified #17 corpus with pinned hardware/budgets, seat/deck bias handling, terminal-vs-capped separation, wall-clock latency, confidence intervals, stored raw reports/configs, and verified replay digests. The committed evidence is a practice calibration of 20 terminal games on two practice decks; it is correctly labelled as such and is not strength evidence.**
