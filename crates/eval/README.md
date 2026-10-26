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

Nothing leaves the machine: no services, downloads, weights or network
inference. The committed calibration is PRACTICE/PRE-ACCEPTANCE evidence: it
does not satisfy issue #26's gate of at least 1000 paired games on the
certified eight-list corpus (roadmap card #17).
