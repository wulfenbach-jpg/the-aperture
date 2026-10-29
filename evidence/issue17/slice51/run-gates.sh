#!/usr/bin/env bash
# Slice 51 gates: review-333 record corrections + public-view projection of
# CR 509.1g unpaired blockers. Every command writes its own exit code.
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice51/gates.log
: > "$LOG"
run() {
  local name="$1"; shift
  echo "=== $name ===" >> "$LOG"
  "$@" >> "$LOG" 2>&1
  local code=$?
  echo "EXIT=$code ($name)" >> "$LOG"
  return $code
}
run fmt cargo fmt --all --check
run slice46 cargo test -p aperture-core --test issue17_slice46 --locked
run slice48 cargo test -p aperture-core --test issue17_slice48 --locked
run slice49 cargo test -p aperture-core --test issue17_slice49 --locked
run starter_corpus cargo test -p aperture-core --test starter_corpus --locked
run catalog_pool cargo test -p aperture-core --test catalog_pool --locked
run slice15_corpus cargo test -p aperture-core --test issue17_slice15_corpus --locked
run ai_search cargo test -p aperture-ai --lib search --locked
run corpus_audit python evidence/issue17/slice50/corpus-audit.py
run clippy cargo clippy --workspace --all-targets --locked -- -D warnings
run release cargo build --workspace --release --locked
run workspace cargo test --workspace --all-targets --locked
echo "=== SUMMARY ===" >> "$LOG"
grep -E "^EXIT=" "$LOG" >> "$LOG"
