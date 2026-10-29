#!/usr/bin/env bash
# Slice 56 gate log: every step records its own exit code.
set -u
cd "$(dirname "$0")/../../.." || exit 1
log=evidence/issue17/slice56/gates.log
: > "$log"

echo "### fmt" >> "$log"
cargo fmt --all --check >> "$log" 2>&1
echo "fmt_exit=$?" >> "$log"

echo "### clippy (workspace, all targets, -D warnings)" >> "$log"
cargo clippy --workspace --all-targets --locked -- -D warnings >> "$log" 2>&1
echo "clippy_exit=$?" >> "$log"

echo "### focused: the suites this delta touches" >> "$log"
cargo test -p aperture-core --test coach_trace --test combat_teaching \
    --test issue17_slice54 --test issue17_slice54_regressions \
    --test issue17_slice55 --test review_regressions --locked >> "$log" 2>&1
echo "focused_exit=$?" >> "$log"

echo "### corpus whole-field audit" >> "$log"
python evidence/issue17/slice54/corpus-audit.py >> "$log" 2>&1
echo "corpus_exit=$?" >> "$log"
