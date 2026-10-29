#!/usr/bin/env bash
set +e
run() {
  printf '\nCOMMAND:'; printf ' %q' "$@"; printf '\n'
  "$@"
  result=$?
  printf 'EXIT=%s\n' "$result"
  if [ "$result" != 0 ]; then exit "$result"; fi
}
run python evidence/issue17/slice49/setup-journey.py
run cargo fmt --all
run cargo fmt --all --check
run cargo test -p aperture-core --locked --test issue17_slice49 --test issue17_slice48 --test issue17_slice45_native --test issue17_slice41 --test issue17_slice44_search_controls --test issue17_slice19 --test ordering_persistence --test replay_digest --test starter_corpus --test card_printings
run cargo clippy --workspace --all-targets --locked -- -D warnings
run cargo build -p aperture-cli --release --locked
run cargo run --release --locked --manifest-path evidence/issue17/slice49-journey-driver/Cargo.toml
run target/release/aperture-cli.exe replay --file evidence/issue17/slice49/cub-replay.json
run git diff --check
