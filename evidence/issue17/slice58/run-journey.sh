#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice58/journey.log
: > "$LOG"
printf 'base=%s; working tree candidate\n' "$(git rev-parse HEAD)" >> "$LOG"
printf '\ncommand=CARGO_TARGET_DIR=<worktree>/target cargo run --release --locked --manifest-path evidence/issue17/slice58-journey-driver/Cargo.toml\n' >> "$LOG"
CARGO_TARGET_DIR='C:/AI/aperture-worktrees/issue-17/target' cargo run --release --locked --manifest-path evidence/issue17/slice58-journey-driver/Cargo.toml >> "$LOG" 2>&1
one=$?
printf 'driver_exit=%s\n' "$one" >> "$LOG"
printf '\ncommand=target/release/aperture-cli.exe replay --file evidence/issue17/slice58/native/demo-save.json\n' >> "$LOG"
target/release/aperture-cli.exe replay --file evidence/issue17/slice58/native/demo-save.json >> "$LOG" 2>&1
two=$?
printf 'separate_release_cli_exit=%s\n' "$two" >> "$LOG"
printf 'driver=%s replay=%s\n' "$one" "$two"
exit $((one | two))
