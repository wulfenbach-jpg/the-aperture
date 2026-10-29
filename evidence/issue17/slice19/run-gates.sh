#!/usr/bin/env bash
# Slice 19 gate log. Every step records its own exit code; nothing is inherited.
set +e
cd /c/AI/aperture-worktrees/issue-17 || exit 9
LOG=evidence/issue17/slice19/gates.log
: > "$LOG"
{
  echo "commit: $(git rev-parse HEAD)"
  echo "started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "=== cargo fmt --all --check ==="
  cargo fmt --all --check 2>&1
  echo "fmt_exit=$?"
  echo
  echo "=== cargo clippy --workspace --all-targets --locked -- -D warnings ==="
  cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -20
  echo "clippy_exit=${PIPESTATUS[0]}"
  echo
  echo "=== cargo build --workspace --release --locked ==="
  cargo build --workspace --release --locked 2>&1 | tail -5
  echo "release_exit=${PIPESTATUS[0]}"
  echo
  echo "=== cargo test --workspace --all-targets --locked --no-fail-fast ==="
  cargo test --workspace --all-targets --locked --no-fail-fast 2>&1 \
    | grep -E "test result:|^error|FAILED|panicked at"
  echo "test_exit=${PIPESTATUS[0]}"
  echo
  echo "finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} >> "$LOG" 2>&1
echo "GATES WRITTEN"
