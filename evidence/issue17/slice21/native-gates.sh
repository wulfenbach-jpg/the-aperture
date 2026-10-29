#!/usr/bin/env bash
set +e
run() { printf '\nCOMMAND:'; printf ' %q' "$@"; printf '\n'; "$@"; code=$?; printf 'EXIT=%s\n' "$code"; if [ "$code" -ne 0 ]; then exit "$code"; fi; }
run cargo test -p aperture-core --test issue17_slice21 --test card_printings --test catalog_pool --test issue17_slice15_corpus --test issue13_commander_combat --test issue17_slice20_privacy --locked
run cargo test -p aperture-core --lib cleanup_expires_regeneration_shields --locked
run cargo fmt --all --check
run cargo clippy --workspace --all-targets --locked -- -D warnings
run cargo build -p aperture-cli --release --locked
run target/release/aperture-cli.exe replay --file evidence/issue17/slice21/demo-replay.json
run target/release/aperture-cli.exe replay --file evidence/issue17/slice20/demo-replay.json
run python evidence/issue17/slice21/corpus-audit.py
printf '\nALL_NATIVE_GATES_PASS\n'
