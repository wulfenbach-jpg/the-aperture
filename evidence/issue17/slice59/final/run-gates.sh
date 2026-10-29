#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../../../.." || exit 1
LOG=evidence/issue17/slice59/final/gates.log
: > "$LOG"
printf 'head=%s\n' "$(git rev-parse HEAD)" >> "$LOG"
printf 'working_tree_candidate=1\n' >> "$LOG"
failed=0
step() {
    name=$1; shift
    printf '\ncommand=%s\n' "$*" >> "$LOG"
    "$@" >> "$LOG" 2>&1
    code=$?
    printf '%s_exit=%s\n' "$name" "$code" >> "$LOG"
    if [ "$code" -ne 0 ]; then failed=1; fi
}
step fmt cargo fmt --all --check
step focused cargo test -p aperture-core --test issue17_slice59_repairs --test issue17_slice59_ring_repairs --test issue17_slice59 --test card_printings --test catalog_pool --test issue17_slice15_corpus --test issue13_commander_combat --test replay_digest --locked
step corpus python evidence/issue17/slice59/corpus-audit.py
step clippy cargo clippy --workspace --all-targets --locked -- -D warnings
step release cargo build --workspace --release --locked
step workspace cargo test --workspace --all-targets --locked
printf '\naggregate_failure=%s\n' "$failed" >> "$LOG"
exit "$failed"
