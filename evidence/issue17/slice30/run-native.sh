set +e
cargo fmt --all --check
printf 'FMT_EXIT=%s\n' "$?"
cargo test -p aperture-core --locked --test issue17_slice30 --test issue17_slice29 --test issue17_slice28 --test issue17_slice24 --test timing_mana --test replay_digest --test card_printings --test catalog_pool
printf 'FOCUSED_EXIT=%s\n' "$?"
cargo clippy --workspace --all-targets --locked -- -D warnings
printf 'CLIPPY_EXIT=%s\n' "$?"
cargo build -p aperture-cli --release --locked
printf 'RELEASE_EXIT=%s\n' "$?"
cargo run --release --locked --manifest-path evidence/issue17/slice30-journey-driver/Cargo.toml
printf 'JOURNEY_EXIT=%s\n' "$?"
./target/release/aperture-cli.exe replay --file evidence/issue17/slice30/demo-replay.json
printf 'REPLAY_EXIT=%s\n' "$?"
python evidence/issue17/slice30/corpus-audit.py
printf 'AUDIT_EXIT=%s\n' "$?"
