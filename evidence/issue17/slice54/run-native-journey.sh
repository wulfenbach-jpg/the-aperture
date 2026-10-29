#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice54/native-journey.log
: > "$LOG"
printf 'head=%s\n' "$(git rev-parse HEAD)" >> "$LOG"
printf '\ncommand=cargo run --release --locked (slice54-journey-driver)\n' >> "$LOG"
(cd evidence/issue17/slice54-journey-driver && cargo run --release --locked) >> "$LOG" 2>&1
one=$?
printf 'EXIT=%s (driver)\n' "$one" >> "$LOG"
printf '\ncommand=target/release/aperture-cli.exe replay --file evidence/issue17/slice54/native/demo-save.json\n' >> "$LOG"
target/release/aperture-cli.exe replay --file evidence/issue17/slice54/native/demo-save.json >> "$LOG" 2>&1
two=$?
printf 'EXIT=%s (separate_release_cli)\n' "$two" >> "$LOG"
printf 'driver=%s replay=%s\n' "$one" "$two"
exit $((one | two))
