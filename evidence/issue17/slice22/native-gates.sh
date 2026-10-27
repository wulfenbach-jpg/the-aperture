#!/usr/bin/env bash
# Slice 22 gates. Every step prints its own exit code; this log is the receipt.
cd /c/AI/aperture-worktrees/issue-17 || exit 1
set -o pipefail
echo "=== git head ==="; git rev-parse HEAD; echo "head_exit=$?"
echo "=== fmt ==="; cargo fmt --all --check; echo "fmt_exit=$?"
echo "=== clippy ==="; cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -5; echo "clippy_exit=${PIPESTATUS[0]}"
echo "=== release build ==="; cargo build --workspace --release --locked 2>&1 | tail -3; echo "release_exit=${PIPESTATUS[0]}"
echo "=== slice22 focused ==="; cargo test -p aperture-core --test issue17_slice22 --locked 2>&1 | grep -E "^test result|^test .* FAILED"; echo "slice22_exit=${PIPESTATUS[0]}"
echo "=== pinned/focused suites ==="
cargo test -p aperture-core --test card_printings --locked 2>&1 | grep -E "^test result"; echo "printings_exit=${PIPESTATUS[0]}"
cargo test -p aperture-core --test catalog_pool --locked 2>&1 | grep -E "^test result"; echo "catalog_exit=${PIPESTATUS[0]}"
cargo test -p aperture-core --test issue13_commander_combat --locked 2>&1 | grep -E "^test result"; echo "combat_exit=${PIPESTATUS[0]}"
cargo test -p aperture-core --test issue17_slice15_corpus --locked 2>&1 | grep -E "^test result"; echo "corpus_exit=${PIPESTATUS[0]}"
cargo test -p aperture-core --test abilities --locked 2>&1 | grep -E "^test result"; echo "abilities_exit=${PIPESTATUS[0]}"
cargo test -p aperture-core --test issue17_slice20 --locked 2>&1 | grep -E "^test result"; echo "slice20_exit=${PIPESTATUS[0]}"
cargo test -p aperture-core --test issue17_slice21 --locked 2>&1 | grep -E "^test result"; echo "slice21_exit=${PIPESTATUS[0]}"
cargo test -p aperture-core --test projection --locked 2>&1 | grep -E "^test result"; echo "projection_exit=${PIPESTATUS[0]}"
cargo test -p aperture-core --test timing_mana --locked 2>&1 | grep -E "^test result"; echo "timing_exit=${PIPESTATUS[0]}"
cargo test -p aperture-core --test m1_multiplayer --locked 2>&1 | grep -E "^test result"; echo "m1_exit=${PIPESTATUS[0]}"
echo "=== workspace tests ==="; cargo test --workspace --all-targets --locked --no-fail-fast 2>&1 | grep -E "^test result: FAILED|^error\[|^error:|failures:" | head -20; echo "workspace_exit=${PIPESTATUS[0]}"
echo "=== done ==="
