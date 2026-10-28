#!/usr/bin/env bash
set -u
cargo fmt --all
printf 'fmt_apply_exit=%s\n' "$?"
cargo fmt --all --check
printf 'fmt_check_exit=%s\n' "$?"
cargo test -p aperture-core --locked --test issue17_slice45 --test issue17_slice45_native --test issue17_slice45_public --test issue17_slice41 --test issue17_slice44 --test issue17_slice44_search_controls --test card_printings --test catalog_pool --test starter_corpus --test issue17_slice15_corpus --test issue13_commander_combat --test ordering_persistence --test replay_digest --test projection
printf 'focused_exit=%s\n' "$?"
cargo clippy --workspace --all-targets --locked -- -D warnings
printf 'clippy_exit=%s\n' "$?"
cargo build -p aperture-cli --release --locked
printf 'release_exit=%s\n' "$?"
cargo run --release --locked --manifest-path evidence/issue17/slice45-journey-driver/Cargo.toml
printf 'journey_exit=%s\n' "$?"
target/release/aperture-cli.exe replay --file evidence/issue17/slice45/demo-replay.json
printf 'separate_replay_exit=%s\n' "$?"
target/release/aperture-cli.exe replay --file evidence/issue17/slice44/demo-replay.json
printf 'prior_snow_replay_exit=%s\n' "$?"
python evidence/issue17/slice45/corpus-audit.py
printf 'audit_exit=%s\n' "$?"
