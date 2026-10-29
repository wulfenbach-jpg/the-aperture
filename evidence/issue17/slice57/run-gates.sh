#!/usr/bin/env bash
# Slice 57 gate script: every step writes its own output and its own EXIT line, so
# a citation always points at the receipt of the command it names.
set -u
cd "$(dirname "$0")/../../.." || exit 1
LOG=evidence/issue17/slice57/gates.log
: > "$LOG"
printf 'head=%s\n' "$(git rev-parse HEAD)" >> "$LOG"

step() {
    name=$1
    shift
    printf '\ncommand=%s\n' "$*" >> "$LOG"
    "$@" >> "$LOG" 2>&1
    code=$?
    printf '%s_exit=%s\n' "$name" "$code" >> "$LOG"
    return $code
}

step fmt cargo fmt --all --check
step clippy cargo clippy --workspace --all-targets --locked -- -D warnings
step focused_slice57 cargo test -p aperture-core --test issue17_slice57 --locked
step focused_neighbours cargo test -p aperture-core --test issue17_slice55 --test issue17_slice54 --test issue17_slice53_ruxa --test card_printings --test catalog_pool --test issue17_slice15_corpus --locked
step ai_search cargo test -p aperture-ai --test search --locked
step corpus python evidence/issue17/slice57/corpus-audit.py
step release_build cargo build --workspace --release --locked

printf '\nall gates recorded\n' >> "$LOG"
