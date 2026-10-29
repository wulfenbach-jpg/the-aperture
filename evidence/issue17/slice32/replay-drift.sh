#!/usr/bin/env bash
# Re-run every committed evidence-only journey driver after the slice-32 change
# and print its live digest next to the digest recorded at HEAD, so the record
# can state exactly which journeys moved.
#
# Caveat (raised by review 289): these runs use the WORKING TREE's driver
# Cargo.lock files. slice22's lock is not committed and slice26-journey-driver
# is untracked, so this script reproduces from this checkout, not from a clean
# `git archive` of every slice.
set -u
cd "$(dirname "$0")/../../.." || exit 1
for dir in evidence/issue17/slice*-journey-driver; do
  slice=$(basename "$dir" | sed 's/-journey-driver//')
  committed=$(git show "HEAD:evidence/issue17/${slice}/demo-journey.json" 2>/dev/null | sed -n 's/.*"digest": "\([0-9a-f]*\)".*/\1/p')
  if [ -z "$committed" ]; then
    echo "${slice}: no committed digest"
    continue
  fi
  live=$(cd "$dir" && cargo run --release --locked 2>/dev/null | sed -n 's/.*"digest": "\([0-9a-f]*\)".*/\1/p')
  if [ "$live" = "$committed" ]; then
    echo "${slice}: UNCHANGED ${live}"
  else
    echo "${slice}: CHANGED committed=${committed} live=${live}"
  fi
done
