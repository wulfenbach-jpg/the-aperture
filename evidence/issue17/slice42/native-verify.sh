#!/usr/bin/env bash
# Slice 42 native verification after the coordinator's own controls.
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 1
log=evidence/issue17/slice42/native-verify.log
: > "$log"

run() {
  local name=$1; shift
  "$@" >> "$log" 2>&1
  local code=$?
  printf '%s_exit=%s\n' "$name" "$code" | tee -a "$log"
  return 0
}

run slice42_equip_controls cargo test -p aperture-core --test issue17_slice42_equip_controls --locked
run slice42 cargo test -p aperture-core --test issue17_slice42 --locked
run slice33 cargo test -p aperture-core --test issue17_slice33 --locked
run card_printings cargo test -p aperture-core --test card_printings --locked
run catalog_pool cargo test -p aperture-core --test catalog_pool --locked
run starter_corpus cargo test -p aperture-core --test starter_corpus --locked
run slice15_corpus cargo test -p aperture-core --test issue17_slice15_corpus --locked
run commander_combat cargo test -p aperture-core --test issue13_commander_combat --locked
run replay_digest cargo test -p aperture-core --test replay_digest --locked
run fmt cargo fmt --all -- --check
run clippy cargo clippy --workspace --all-targets --locked -- -D warnings
run release cargo build -p aperture-cli --release --locked
run workspace cargo test --workspace --locked
tail -n 1 "$log"
