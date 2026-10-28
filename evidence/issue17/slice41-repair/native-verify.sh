#!/usr/bin/env bash
set -u
run() { printf '\nCOMMAND:'; printf ' %q' "$@"; printf '\n'; "$@"; code=$?; printf 'EXIT=%s\n' "$code"; if [ "$code" -ne 0 ]; then exit "$code"; fi; }
run git rev-parse HEAD
run cargo test -p aperture-core --locked --test issue17_slice41 --test ordering_persistence --test replay_digest --test issue17_slice12 --test rules_conformance
run cargo fmt --all --check
run cargo clippy --workspace --all-targets --locked -- -D warnings
run cargo build -p aperture-cli --release --locked
run target/release/aperture-cli.exe replay --file evidence/issue17/slice41/demo-replay.json
run git diff --exit-code -- crates/core/src/state.rs crates/core/tests/issue17_slice41.rs
