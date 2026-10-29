#!/usr/bin/env bash
# #28 integration gates (task t_cf764bc5).
#
# Every step prints its own exit code, so the log is evidence and not a claim.
# Run from the worktree root with one cargo invocation at a time; concurrent
# cargo runs against one CARGO_TARGET_DIR produce spurious LNK1104 failures.
set -u
cd "$(dirname "$0")/../.." || exit 1

echo "gates: $(git rev-parse HEAD) on $(git branch --show-current)"
echo "rustc: $(rustc --version)"
echo "cargo: $(cargo --version)"
echo "started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

echo "=== cargo fmt --all -- --check ==="
cargo fmt --all -- --check
fmt_exit=$?
echo "fmt_exit=$fmt_exit"

echo
echo "=== cargo clippy --workspace --all-targets --locked -- -D warnings ==="
cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit=$?
echo "clippy_exit=$clippy_exit"

echo
echo "=== cargo test --all-targets --locked ==="
cargo test --all-targets --locked
test_exit=$?
echo "test_exit=$test_exit"

echo
echo "=== cargo build --release --locked ==="
cargo build --release --locked
release_exit=$?
echo "release_exit=$release_exit"

echo
echo "finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "SUMMARY fmt=$fmt_exit clippy=$clippy_exit tests=$test_exit release=$release_exit"
if [ "$fmt_exit" -eq 0 ] && [ "$clippy_exit" -eq 0 ] && [ "$test_exit" -eq 0 ] && [ "$release_exit" -eq 0 ]; then
    echo "GATES_PASS"
    exit 0
fi
echo "GATES_FAIL"
exit 1
