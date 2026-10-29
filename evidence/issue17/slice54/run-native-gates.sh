#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice54/native-gates.log
: > "$LOG"
failed=0
step() {
  name=$1; shift
  printf '\n=== %s ===\ncommand=%s\n' "$name" "$*" >> "$LOG"
  "$@" >> "$LOG" 2>&1
  rc=$?
  printf 'EXIT=%s (%s)\n' "$rc" "$name" >> "$LOG"
  printf '%s exit=%s\n' "$name" "$rc"
  if [ "$rc" != 0 ]; then failed=1; fi
}
printf 'head=%s\n' "$(git rev-parse HEAD)" >> "$LOG"
step fmt cargo fmt --all --check
step focused_core cargo test -p aperture-core --locked --test issue17_slice54 --test issue17_slice54_regressions --test card_printings --test catalog_pool --test issue17_slice15_corpus --test issue13_commander_combat --test replay_digest --test issue17_slice46 --test starter_corpus
step policy_adapter cargo test -p aperture-ai --locked --test issue17_ursine_policy --test heuristic
step corpus python evidence/issue17/slice54/corpus-audit.py
step clippy cargo clippy --workspace --all-targets --locked -- -D warnings
step release cargo build --workspace --release --locked
step journey bash evidence/issue17/slice54/run-native-journey.sh
exit "$failed"
