#!/usr/bin/env bash
# Rebuild and re-run the slice-53 Ruxa journey driver, then replay its save with
# the separate release CLI. Both steps log their own command and exit code; the
# driver writes evidence/issue17/slice53/ruxa/{demo-save,demo-replay,demo-journey}.json.
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice53/journey.log
: > "$LOG"
echo "head=$(git rev-parse HEAD)" >> "$LOG"

echo "=== driver ===" >> "$LOG"
( cd evidence/issue17/slice53-ruxa-driver && cargo run --release --locked ) >> "$LOG" 2>&1
driver=$?
echo "EXIT=$driver (driver)" >> "$LOG"

echo "=== separate release CLI replay ===" >> "$LOG"
cargo run --release --locked -p aperture-cli -- replay --file evidence/issue17/slice53/ruxa/demo-save.json >> "$LOG" 2>&1
replay=$?
echo "EXIT=$replay (cli_replay)" >> "$LOG"

echo "driver=$driver replay=$replay"
exit $(( driver | replay ))
