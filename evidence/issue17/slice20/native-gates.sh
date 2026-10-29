#!/usr/bin/env bash
set +e
run() {
  printf '\nCOMMAND:'; printf ' %q' "$@"; printf '\n'
  "$@"
  code=$?
  printf '\nCOMMAND_EXIT=%s\n' "$code"
  if [ "$code" -ne 0 ]; then exit "$code"; fi
}
run git rev-parse HEAD
run git diff --check 7dc933a..HEAD
run cargo fmt --all --check
run cargo clippy --workspace --all-targets --locked -- -D warnings
run cargo build --release -p aperture-cli --locked
run target/release/aperture-cli.exe replay --file evidence/issue17/slice20/demo-replay.json
run python evidence/issue17/slice20/corpus-audit.py
