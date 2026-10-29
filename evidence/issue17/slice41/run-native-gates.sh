#!/usr/bin/env bash
# Slice 41 native gates: writes every command's own exit status.
set -u
cd "$(dirname "$0")/../../.." || exit 1
log=evidence/issue17/slice41/native-gates.log
: > "$log"

cargo fmt --all --check >> "$log" 2>&1
fmt=$?
echo "fmt_exit=$fmt" >> "$log"

cargo test -p aperture-core --test issue17_slice41 --locked >> "$log" 2>&1
slice41=$?
echo "slice41_exit=$slice41" >> "$log"

for suite in card_printings catalog_pool issue17_slice15_corpus issue13_commander_combat replay_digest counters_tokens_library abilities; do
  cargo test -p aperture-core --test "$suite" --locked >> "$log" 2>&1
  echo "${suite}_exit=$?" >> "$log"
done

cargo clippy --workspace --all-targets --locked -- -D warnings >> "$log" 2>&1
clippy=$?
echo "clippy_exit=$clippy" >> "$log"

cargo test --workspace --locked >> "$log" 2>&1
workspace=$?
echo "workspace_exit=$workspace" >> "$log"

cargo build --workspace --release --locked >> "$log" 2>&1
release=$?
echo "release_exit=$release" >> "$log"

tail -n 8 "$log"
