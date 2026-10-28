#!/usr/bin/env bash
# Slice 52 gates: review-335 record corrections (R335-1 search receipt, R335-2 sweep statement,
# NEXT-CARDS describe_target attribution). Every command writes its own exit code.
#
# Difference from slice51/run-gates.sh: the separate aperture-ai search gate runs the REAL command
# (`--test search`). Slice 51's script ran `--lib search`, which matched zero test names while
# exiting 0; that zero-match attempt is kept here as its own labelled step so the receipt shows both
# and no reader can mistake the empty run for the nine-test gate (review 335 R335-1).
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice52/gates.log
: > "$LOG"
echo "head=$(git rev-parse HEAD)" >> "$LOG"
echo "command_set=slice52/run-gates.sh" >> "$LOG"
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
# R335-1: the true search gate first, then the historical zero-match attempt, explicitly labelled.
run ai_search cargo test -p aperture-ai --test search --locked
run ai_search_lib_zero_match_attempt cargo test -p aperture-ai --lib search --locked
run corpus_audit python evidence/issue17/slice50/corpus-audit.py
run clippy cargo clippy --workspace --all-targets --locked -- -D warnings
run release cargo build --workspace --release --locked
run workspace cargo test --workspace --all-targets --locked
echo "=== SUMMARY ===" >> "$LOG"
grep -E "^EXIT=" "$LOG" >> "$LOG"
