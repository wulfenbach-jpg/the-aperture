#!/usr/bin/env bash
# Re-run the committed four-seat 40-life witness journey driver and compare its artifacts with the
# committed ones. Correction from review 337 R337-3: the earlier "committed CRLF vs fresh LF" note was
# false - all three committed artifacts carry zero CR bytes in blob and worktree, so the comparison below
# still normalises newlines defensively but the artifacts are byte-identical at this head.
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice52/journey.log
TMP="${TMPDIR:-/tmp}/slice52-journey"
mkdir -p "$TMP"
: > "$LOG"
OUT=evidence/issue17/slice50/witness
echo "head=$(git rev-parse HEAD)" >> "$LOG"
echo "=== build+run driver (release, --locked) ===" >> "$LOG"
( cd evidence/issue17/slice50-witness-driver && cargo run --release --locked ) >> "$LOG" 2>&1
echo "DRIVER_EXIT=$?" >> "$LOG"
for f in demo-journey.json demo-replay.json demo-save.json; do
  git show "HEAD:$OUT/$f" | tr -d '\r' > "$TMP/base-$f"
  tr -d '\r' < "$OUT/$f" > "$TMP/head-$f"
  if cmp -s "$TMP/base-$f" "$TMP/head-$f"; then
    echo "CONTENT_IDENTICAL_AFTER_NEWLINE_NORMALISATION $f" >> "$LOG"
  else
    echo "DIFFERS $f" >> "$LOG"
    diff "$TMP/base-$f" "$TMP/head-$f" | head -20 >> "$LOG"
  fi
done
grep -E "outcome|steps|events|digest" "$OUT/demo-journey.json" >> "$LOG" 2>/dev/null
echo "=== separate CLI replay ===" >> "$LOG"
cargo run --release --locked -p aperture-cli -- replay --file "$OUT/demo-save.json" >> "$LOG" 2>&1
echo "SEPARATE_REPLAY_EXIT=$?" >> "$LOG"
