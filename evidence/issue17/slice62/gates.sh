#!/usr/bin/env bash
set +e
log=evidence/issue17/slice62/gates.log
: > "$log"
run() { name=$1; shift; "$@" >>"$log" 2>&1; code=$?; echo "$name exit=$code" | tee -a "$log"; }
run slice62 cargo test -q -p aperture-core --test issue17_slice62
run card_printings cargo test -q -p aperture-core --test card_printings
run catalog_pool cargo test -q -p aperture-core --test catalog_pool
run corpus_pins cargo test -q -p aperture-core --test issue17_slice15_corpus --test issue13_commander_combat --test replay_digest
run fmt cargo fmt --all --check
run clippy cargo clippy -q --workspace --all-targets -- -D warnings
run release cargo build --release --locked --workspace
