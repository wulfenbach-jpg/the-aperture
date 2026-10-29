#!/usr/bin/env bash
# Slice 28 native gates. Every command's exit status is recorded; the script
# exits non-zero if any gate failed.
set -u
cd "$(dirname "$0")/../../.." || exit 1
OUT=evidence/issue17/slice28
mkdir -p "$OUT"
status=0
run() {
  name="$1"; shift
  echo "=== $name: $* ==="
  "$@" >"$OUT/$name.log" 2>&1
  code=$?
  echo "$name exit=$code" | tee -a "$OUT/gates-summary.log"
  [ $code -ne 0 ] && status=1
  return 0
}
: >"$OUT/gates-summary.log"
run fmt cargo fmt --all --check
run clippy cargo clippy --workspace --all-targets --locked -- -D warnings
run release-build cargo build --workspace --release --locked
run focused cargo test -p aperture-core --locked \
  --test issue17_slice28 --test issue17_slice26 --test issue17_slice25 \
  --test issue17_slice24 --test issue17_slice23 --test issue17_slice22 \
  --test issue17_slice21 --test issue17_slice20 --test issue17_slice19 \
  --test issue17_slice15_corpus --test issue13_commander_combat \
  --test rules_conformance --test catalog_pool --test card_printings \
  --test abilities --test timing_mana --test sba_cleanup --test replay_digest
run workspace cargo test --workspace --locked
echo "overall exit=$status" | tee -a "$OUT/gates-summary.log"
exit $status
