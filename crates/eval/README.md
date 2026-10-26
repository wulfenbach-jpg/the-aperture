# aperture-eval — paired-seed strength/latency harness

Run the paired seed x rotation benchmark from the repository root; fixture
paths in the config are relative to the CWD:

```
cargo run -q -p aperture-eval --release -- run \
  crates/eval/configs/practice-calibration.json \
  reports/eval/practice-calibration-paired-seeds-v1
# re-verify every stored replay digest:
cargo run -q -p aperture-eval --release -- verify \
  reports/eval/practice-calibration-paired-seeds-v1/report.json
```

Deterministic mode: `search_max_millis: 0` neutralizes the search policy's
wall-clock cutoff (passed as u64::MAX), so node/rollout budgets govern and the
games are a reproducible function of (seed, config, code); positive values make
the policy wall-clock dependent and non-re-derivable.

## Evidence-integrity pins (repair 3)

- **Dirty-tree gate.** A run refuses — before any game executes or any evidence
  is written — whenever the working tree carries tracked modifications
  (`git status --porcelain --untracked-files=no` non-empty), because then
  `hardware.git_commit` could not name the code that reproduces the report.
  Running outside any git repository is also refused (unnameable provenance).
  The explicit escape hatch `--allow-dirty` emits the report but marks it
  "DIRTY TREE - NOT REPRODUCIBLE ... NO value may support any claim" and
  suppresses the paired interval, mean and Wilson readouts; the pin records
  `git_dirty = true` plus the exact `HEAD^{tree}` either way.
- **Full commit + tree pin.** Every report/SUMMARY records the driver commit
  AND `git rev-parse HEAD^{tree}`, plus "clean tracked tree: yes/no". The pin
  is captured at run time from the tree that actually executes, so checking out
  the pinned commit reproduces the report.
- **Byte-reproducible scored report.** Per-game wall/verify/search timings are
  the only non-deterministic bytes a run produces; they are written to the
  sibling `timings.json` sidecar and kept OUT of `report.json` by schema design
  (schema 3). A scored report is therefore a deterministic function of
  (commit, config, fixtures, seeds, hardware identity): checking out the pinned
  commit in a scratch worktree and re-running the exact command yields a
  byte-for-byte-identical `report.json` and `SUMMARY.txt`; `timings.json`
  carries the (expectedly different) latency rows. Raw latency evidence is
  retained — not discarded — in the sidecar.

Reproduction protocol for any committed report:

```
# from any checkout of the pinned commit (e.g. git worktree add <path> <pin>)
cargo run -q -p aperture-eval --release -- run <config> <outdir>
diff <pin>/reports/.../report.json <committed>/reports/.../report.json   # must be empty
```

Nothing leaves the machine: no services, downloads, weights or network
inference. The committed calibration is PRACTICE/PRE-ACCEPTANCE evidence: it
does not satisfy issue #26's gate of at least 1000 paired games on the
certified eight-list corpus (roadmap card #17).
