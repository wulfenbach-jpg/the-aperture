#!/usr/bin/env bash
# Independent reproduction of the slice-38 journey from the record alone: export
# the committed tree with `git archive` into a fresh out-of-tree directory and run
# the evidence driver there, then compare its artifacts with the committed blobs.
set -u
SRC=/c/AI/aperture-worktrees/issue-17
OUT=/c/AI/r300
STAMP=$(date +%s)

mkdir -p "$OUT/$STAMP"
cd "$SRC" || exit 1
git archive HEAD | tar -x -C "$OUT/$STAMP" || exit 1
cd "$OUT/$STAMP/evidence/issue17/slice38-journey-driver" || exit 1
cargo run --release --locked > "$OUT/driver-$STAMP.log" 2>&1
echo "driver_exit=$?" >> "$OUT/driver-$STAMP.log"
cd "$OUT/$STAMP" || exit 1
sha256sum evidence/issue17/slice38/driver-journey.json \
          evidence/issue17/slice38/driver-replay.json \
          evidence/issue17/slice38/driver-save.json >> "$OUT/driver-$STAMP.log" 2>&1
echo "=== committed blobs ===" >> "$OUT/driver-$STAMP.log"
cd "$SRC" || exit 1
for f in driver-journey driver-replay driver-save; do
  git show "HEAD:evidence/issue17/slice38/$f.json" | sha256sum
done >> "$OUT/driver-$STAMP.log" 2>&1
tail -n 12 "$OUT/driver-$STAMP.log"
