#!/usr/bin/env bash
set +e
log=evidence/issue17/slice67/final-native/gates.log
: > "$log"
fail=0
run() { name=$1; shift; "$@" >>"$log" 2>&1; code=$?; echo "$name exit=$code" | tee -a "$log"; [ "$code" -eq 0 ] || fail=1; }
run checkpoint cargo test --locked -q -p aperture-core --test issue17_slice67_checkpoint
run cards cargo test --locked -q -p aperture-core --test issue17_slice67_cards
run ordering cargo test --locked -q -p aperture-core --test issue17_slice66
run slice62 cargo test --locked -q -p aperture-core --test issue17_slice62
run slice63 cargo test --locked -q -p aperture-core --test issue17_slice63
run printings cargo test --locked -q -p aperture-core --test card_printings
run catalog cargo test --locked -q -p aperture-core --test catalog_pool
run corpus cargo test --locked -q -p aperture-core --test issue17_slice15_corpus --test issue13_commander_combat --test replay_digest
run import cargo test --locked -q -p aperture-import --test starter_corpus_planned --test starter_corpus_analyzer
run fmt cargo fmt --all --check
run clippy cargo clippy --locked -q --workspace --all-targets -- -D warnings
run release cargo build --locked -q --release --workspace
echo "aggregate exit=$fail" | tee -a "$log"
exit "$fail"
