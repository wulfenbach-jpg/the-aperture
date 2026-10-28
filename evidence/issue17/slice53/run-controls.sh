#!/usr/bin/env bash
# Discriminating controls for the slice-53 Ruxa surface.
#
# Both controls mutate ONE thing in the product source, re-run the focused suite,
# and then restore the file from git. A control that leaves the suite green means
# the corresponding assertion does not actually test what it claims, so each log
# must show failures. This is a post-hoc discrimination check on an
# implementation-first slice, not TDD, and the logs are labelled that way.
set -u
cd "$(dirname "$0")/../../.." || exit 1
OUT=evidence/issue17/slice53

control() {
  name=$1
  file=$2
  echo "=== $name ==="
  python evidence/issue17/slice53/mutate.py "$name"
  cargo test -p aperture-core --test issue17_slice53_ruxa --locked > "$OUT/control-$name.log" 2>&1
  code=$?
  echo "EXIT=$code ($name)"
  git checkout -- "$file"
}

control no_abilities_predicate crates/core/src/state.rs
control assignment_ignored crates/core/src/engine.rs

echo "=== restored suite ==="
cargo test -p aperture-core --test issue17_slice53_ruxa --locked > "$OUT/control-restored.log" 2>&1
echo "EXIT=$? (restored)"
git status --short crates/core/src/state.rs crates/core/src/engine.rs
