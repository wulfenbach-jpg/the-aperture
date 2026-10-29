#!/usr/bin/env bash
# Slice 57: one real four-seat, 40-life journey on the exact package snapshot plus a
# separate release-CLI replay of the recorded save. Never a certificate.
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice57/journey.log
: > "$LOG"
printf 'head=%s\n' "$(git rev-parse HEAD)" >> "$LOG"
printf '\ncommand=cargo run --release --locked (slice57-journey-driver)\n' >> "$LOG"
(cd evidence/issue17/slice57-journey-driver && cargo run --release --locked) >> "$LOG" 2>&1
one=$?
printf 'EXIT=%s (driver)\n' "$one" >> "$LOG"
printf '\ncommand=target/release/aperture-cli.exe replay --file evidence/issue17/slice57/native/demo-save.json\n' >> "$LOG"
target/release/aperture-cli.exe replay --file evidence/issue17/slice57/native/demo-save.json >> "$LOG" 2>&1
two=$?
printf 'EXIT=%s (separate_release_cli)\n' "$two" >> "$LOG"
printf 'driver=%s replay=%s\n' "$one" "$two"
exit $((one | two))
