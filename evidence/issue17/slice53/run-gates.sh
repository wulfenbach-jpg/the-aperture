#!/usr/bin/env bash
# Slice 53 gates: fmt, the Ruxa focused suites, the separate aperture-ai search
# suite, the corpus audit, clippy -D warnings, the locked release build and the
# whole-workspace test run. Each step records its own output and its own exit
# line, so a claim can always be traced to the command that produced it.
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice53/gates.log
: > "$LOG"
echo "head=$(git rev-parse HEAD)" >> "$LOG"
echo "started=$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$LOG"

step() {
  name=$1
  shift
  echo "=== $name ===" >> "$LOG"
  echo "command=$*" >> "$LOG"
  "$@" >> "$LOG" 2>&1
  code=$?
  echo "EXIT=$code ($name)" >> "$LOG"
  echo "$name exit=$code"
}

step fmt cargo fmt --all --check
step focused_ruxa cargo test -p aperture-core --test issue17_slice53_ruxa --locked
step focused_slice46 cargo test -p aperture-core --test issue17_slice46 --locked
step focused_slice48 cargo test -p aperture-core --test issue17_slice48 --locked
step focused_slice49 cargo test -p aperture-core --test issue17_slice49 --locked
step card_printings cargo test -p aperture-core --test card_printings --locked
step starter_corpus cargo test -p aperture-core --test starter_corpus --locked
step catalog_pool cargo test -p aperture-core --test catalog_pool --locked
step slice15_corpus cargo test -p aperture-core --test issue17_slice15_corpus --locked
# R335-1: the search gate is the real --test search command, never a --lib filter
# that matches zero tests.
step ai_search cargo test -p aperture-ai --test search --locked
step corpus_audit python evidence/issue17/slice53/corpus-audit.py
step clippy cargo clippy --workspace --all-targets --locked -- -D warnings
step locked_release cargo build --workspace --release --locked
step workspace_tests cargo test --workspace --all-targets --locked

echo "finished=$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$LOG"
grep -c "^EXIT=" "$LOG" | sed 's/^/steps_recorded=/'
grep "^EXIT=" "$LOG" | sort | uniq -c
