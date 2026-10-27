# Reproduction proof — practice calibration v1 regeneration (repair 3)

Pinned driver commit (hardware.git_commit in the committed report):
`d44d09d8ab49dd0733fc5bdff846391631e79b7c` (tree `9e8580ae587cd57ae1eb8e75f265e2f1f5948bab`,
`git_dirty=false`, clean tracked tree).

Committed evidence reviewed here: `c255939a26096af797a3db318f11619a4086ca53`
(`reports/eval/practice-calibration-paired-seeds-v1/{report.json,SUMMARY.txt,config.json,config-insha256.txt,timings.json}`).

The committed report was produced in the main worktree `C:/AI/aperture-worktrees/issue-26`
in a returned-to-HEAD-clean tracked tree (the report files were restored to HEAD
before the run so the tracked-tree diff was empty at run start), with the exact
documented command:

```
cargo run -q -p aperture-eval --release -- run \
  crates/eval/configs/practice-calibration.json \
  reports/eval/practice-calibration-paired-seeds-v1
```

## Scratch-checkout reproduction

```
git worktree add C:/AI/aperture-worktrees/issue-26-repro-pin d44d09d8ab49dd0733fc5bdff846391631e79b7c
cd C:/AI/aperture-worktrees/issue-26-repro-pin
cargo run -q -p aperture-eval --release -- run crates/eval/configs/practice-calibration.json \
  reports/eval/practice-calibration-paired-seeds-v1
cargo run -q -p aperture-eval --release -- verify reports/eval/practice-calibration-paired-seeds-v1/report.json
cmp reports/eval/practice-calibration-paired-seeds-v1/report.json \
  C:/AI/aperture-worktrees/issue-26/reports/eval/practice-calibration-paired-seeds-v1/report.json
diff -q reports/eval/practice-calibration-paired-seeds-v1/SUMMARY.txt \
  C:/AI/aperture-worktrees/issue-26/reports/eval/practice-calibration-paired-seeds-v1/SUMMARY.txt
```

Observed (full transcript below): run exit 0; `verify: 20 verified, 0 mismatched
of 20 recorded games (20 declared; schema 3, run_id
practice-calibration-paired-seeds-v1)`, verify exit 0;
`cmp` silent (byte-for-byte identical report.json);
`diff -q` silent for SUMMARY.txt and config.json.

## Disclosed working-copy caveat (Windows autocrlf)

This host checks out LF blobs as CRLF (`core.autocrlf=true` is configured
globally). Two consequences, both handled and disclosed rather than hidden:

1. The first scratch run recorded `config_input_sha256 = f7c876b8...` (CRLF
   working copy) instead of the committed pin `3b8a289e...` (LF blob). The
   canonical config digest `d701869c00aac8596c1baa822acd334e963f0b62946262a96b0f3cfd966bf47d`
   was IDENTICAL in both runs, and all 20 replay digests verified — the parsed
   input is the same. For the byte-for-byte proof the config was re-smudged to
   the exact blob bytes (`rm config && git -c core.autocrlf=false checkout --
   crates/eval/configs/practice-calibration.json` -> sha 3b8a289e...).
2. The pre-repair-era committed artifacts at d44d09d are LF blobs; after the
   first scratch run rewrote them, `git status --porcelain -uno` listed them
   as (post-run, expected) modifications, and the second scratch attempt was
   REFUSED BY THE NEW GATE (exit 1, no report written — the dirty-tree refusal
   proving itself in practice). The scratch tree was restored with
   `git -c core.autocrlf=false checkout --force -- reports/eval/...-v1
   crates/eval/configs/practice-calibration.json` before the final accepted
   attempt (status empty at run start).

The latencies sidecar `timings.json` differs between runs BY DESIGN
(committed `a2e873b596c5a9b8...` vs scratch `e593f0b9d23f9d21...` full sha
recorded below); wall/verify/search milliseconds are the only non-reproducible
bytes and are deliberately excluded from report.json (schema 3).

## Final accepted attempt verbatim

```
$ cargo run -q -p aperture-eval --release -- run crates/eval/configs/practice-calibration.json reports/eval/practice-calibration-paired-seeds-v1
practice calibration: search did not exceed parity within disclosed uncertainty (10 calibration units); no strength claim
wrote raw report to reports/eval/practice-calibration-paired-seeds-v1/report.json (20 games), timings sidecar reports/eval/practice-calibration-paired-seeds-v1/timings.json
RUN_EXIT=0
$ cargo run -q -p aperture-eval --release -- verify reports/eval/practice-calibration-paired-seeds-v1/report.json
verify: 20 verified, 0 mismatched of 20 recorded games (20 declared; schema 3, run_id practice-calibration-paired-seeds-v1)
VERIFY_EXIT=0
$ cmp reports/eval/practice-calibration-paired-seeds-v1/report.json C:/AI/aperture-worktrees/issue-26/reports/eval/practice-calibration-paired-seeds-v1/report.json
REPORT_BYTE_IDENTICAL        (cmp exit 0, no difference reported)
$ diff -q .../SUMMARY.txt C:/.../SUMMARY.txt
SUMMARY_BYTE_IDENTICAL       (diff exit 0)
$ diff -q .../config.json C:/.../config.json
CONFIG_BYTE_IDENTICAL        (diff exit 0)
$ git rev-parse HEAD
d44d09d8ab49dd0733fc5bdff846391631e79b7c
scratch timings.json sha256: e593f0b9d23f9d21...     (committed: a2e873b596c5a9b8...)
post-run `git status --porcelain -uno`: " M .../SUMMARY.txt", " M .../report.json"
  (the regenerated scored files — expected post-write state; the run-time pin
  was captured BEFORE any write as clean)
```

Numbers reproduce exactly: 20 recorded / 20 declared, terminal 20 (search
wins 0, losses 20, draws 0), terminal unverified 0, capped 0, failed 0,
policy rejected 0, replay mismatches 0; paired mean diff (fair odds) -0.5,
CI95 [-0.5, -0.5]; headline "search did not exceed parity within disclosed
uncertainty ... no strength claim". No strength claim is made and the
#26 certified-corpus gate (>=1000 paired games, #17 inputs) remains unmet.
