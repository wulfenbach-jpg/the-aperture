#!/usr/bin/env bash
# Reproducibility receipt for the slice-38 journey: run the driver twice and hash
# every artifact both times, then replay the driver's own replay file with a
# separate release CLI build. Any difference is a hard failure.
set -u
cd "$(dirname "$0")/../../.." || exit 1
EVID=evidence/issue17/slice38
DRIVER=evidence/issue17/slice38-journey-driver

{
  echo "== first run =="
  ( cd "$DRIVER" && cargo run --release --locked 2>&1 ) | tail -n 40
  sha256sum "$EVID/driver-journey.json" "$EVID/driver-replay.json" "$EVID/driver-save.json"
} > "$EVID/driver-reproducibility-first.log" 2>&1 || exit 1

{
  echo "== second run =="
  ( cd "$DRIVER" && cargo run --release --locked 2>&1 ) | tail -n 40
  sha256sum "$EVID/driver-journey.json" "$EVID/driver-replay.json" "$EVID/driver-save.json"
} > "$EVID/driver-reproducibility-second.log" 2>&1 || exit 2

if diff -q <(grep -E '^[0-9a-f]{64}  ' "$EVID/driver-reproducibility-first.log") \
           <(grep -E '^[0-9a-f]{64}  ' "$EVID/driver-reproducibility-second.log") > /dev/null; then
  echo "ARTIFACTS_IDENTICAL=yes" | tee "$EVID/driver-reproducibility.log"
else
  echo "ARTIFACTS_IDENTICAL=no" | tee "$EVID/driver-reproducibility.log"
  exit 3
fi
grep -E '^[0-9a-f]{64}  ' "$EVID/driver-reproducibility.log" >> /dev/null 2>&1

echo "== separate release CLI replay ==" >> "$EVID/driver-reproducibility.log"
cargo run --release --locked -p aperture-cli -- replay --file "$EVID/driver-replay.json" \
  > "$EVID/driver-cli-replay.log" 2>&1
echo "cli_replay_exit=$?" >> "$EVID/driver-reproducibility.log"
tail -n 5 "$EVID/driver-cli-replay.log" >> "$EVID/driver-reproducibility.log"
cat "$EVID/driver-reproducibility.log"
