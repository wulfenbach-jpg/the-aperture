#!/usr/bin/env bash
# Slice 42 final native verification (after coordinator formatting + controls).
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 1
log=evidence/issue17/slice42/native-verify-final.log
: > "$log"

run() {
  local name=$1; shift
  "$@" >> "$log" 2>&1
  local code=$?
  printf '%s_exit=%s\n' "$name" "$code" | tee -a "$log"
  return 0
}

run fmt cargo fmt --all -- --check
run clippy cargo clippy --workspace --all-targets --locked -- -D warnings
run slice42_equip_controls cargo test -p aperture-core --test issue17_slice42_equip_controls --locked
run slice42 cargo test -p aperture-core --test issue17_slice42 --locked

# Regenerate the four-seat journey from the corrected engine and verify it.
cd evidence/issue17/slice42-journey-driver || exit 1
cargo run --release --locked >> "../../slice42/native-verify-final.log" 2>&1
printf 'journey_exit=%s\n' "$?" | tee -a "../../slice42/native-verify-final.log"
cd ../../.. || exit 1
run replay cargo run -p aperture-cli --release --locked -- replay --file evidence/issue17/slice42/demo-replay.json
