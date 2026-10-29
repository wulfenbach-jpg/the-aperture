#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice58/gates.log
: > "$LOG"
printf 'base=%s; working tree candidate (committed blob correspondence verified before publication)\n' "$(git rev-parse HEAD)" >> "$LOG"
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
step focused cargo test -p aperture-core --test issue17_slice57 --test issue17_slice54 --test issue17_slice53_ruxa --test card_printings --test catalog_pool --test starter_corpus --test replay_digest --locked
step clippy cargo clippy --workspace --all-targets --locked -- -D warnings
step corpus python evidence/issue17/slice57/corpus-audit.py
step release cargo build --workspace --release --locked
printf '\naggregate_failure=%s\n' "$failed" >> "$LOG"
exit "$failed"
