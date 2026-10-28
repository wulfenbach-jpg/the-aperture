#!/usr/bin/env bash
set +e
step() { printf '\nCOMMAND:'; printf ' %q' "$@"; printf '\n'; "$@"; code=$?; printf '\nEXIT=%s\n' "$code"; if [ "$code" -ne 0 ]; then exit "$code"; fi; }
step cargo fmt --all
step cargo fmt --manifest-path evidence/issue17/slice44-journey-driver/Cargo.toml
step cargo fmt --all --check
step cargo test -p aperture-core --test issue17_slice44 --test issue17_slice44_search_controls --test issue17_slice44_sources --test issue17_slice41 --test issue17_slice43 --test card_printings --test catalog_pool --test starter_corpus --test issue17_slice15_corpus --test issue13_commander_combat --test ordering_persistence --test replay_digest --locked
step cargo clippy --workspace --all-targets --locked -- -D warnings
step cargo build -p aperture-cli --release --locked
step cargo run --manifest-path evidence/issue17/slice44-journey-driver/Cargo.toml --release --locked
step target/release/aperture-cli.exe replay --file evidence/issue17/slice44/demo-replay.json
step target/release/aperture-cli.exe replay --file evidence/issue17/slice41/demo-replay.json
step python evidence/issue17/slice44/corpus-audit.py
step git diff --check
