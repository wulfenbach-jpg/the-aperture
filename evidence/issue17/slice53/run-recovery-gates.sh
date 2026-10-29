#!/usr/bin/env bash
# Recovery-run gates; preserve the interrupted run's original receipts.
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice53/recovery-gates.log
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
step focused cargo test -p aperture-core --locked --test issue17_slice53_ruxa --test issue17_slice46 --test issue17_slice48 --test issue17_slice49 --test card_printings --test starter_corpus --test catalog_pool --test issue17_slice15_corpus
step corpus python evidence/issue17/slice53/corpus-audit.py
step clippy cargo clippy --workspace --all-targets --locked -- -D warnings
step release cargo build --workspace --release --locked
# Uses masked offered actions and a matching separate release-CLI replay.
step journey bash evidence/issue17/slice53/run-journey.sh
exit "$failed"
