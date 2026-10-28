#!/usr/bin/env bash
# Re-run the committed four-seat witness journey driver at the slice-51 head and
# compare its artifacts with the committed ones after newline normalisation.
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice51/journey.log
: > "$LOG"
OUT=evidence/issue17/slice50/witness
echo "=== build+run driver (release) ===" >> "$LOG"
( cd evidence/issue17/slice50-witness-driver && cargo run --release --locked ) >> "$LOG" 2>&1
echo "DRIVER_EXIT=$?" >> "$LOG"
for f in demo-journey.json demo-replay.json demo-save.json; do
  git show "HEAD:$OUT/$f" | tr -d '\r' > "/tmp/base-$f" 2>/dev/null
  tr -d '\r' < "$OUT/$f" > "/tmp/head-$f"
  if cmp -s "/tmp/base-$f" "/tmp/head-$f"; then
    echo "CONTENT_IDENTICAL_AFTER_NEWLINE_NORMALISATION $f" >> "$LOG"
  else
    echo "DIFFERS $f" >> "$LOG"
    diff "/tmp/base-$f" "/tmp/head-$f" | head -20 >> "$LOG"
  fi
done
grep -E "outcome|steps|events|digest" "$OUT/demo-journey.json" >> "$LOG" 2>/dev/null
echo "JOURNEY_EXIT=$?" >> "$LOG"
