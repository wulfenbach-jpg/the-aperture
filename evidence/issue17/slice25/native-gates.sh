#!/usr/bin/env bash
set +e
out=evidence/issue17/slice25
cargo fmt --all --check > "$out/native-fmt.log" 2>&1; a=$?; printf 'fmt_exit=%s\n' "$a"
cargo clippy --workspace --all-targets --locked -- -D warnings > "$out/native-clippy.log" 2>&1; b=$?; printf 'clippy_exit=%s\n' "$b"
cargo build --release -p aperture-cli --locked > "$out/native-release.log" 2>&1; c=$?; printf 'release_exit=%s\n' "$c"
CARGO_TARGET_DIR=C:/AI/aperture-worktrees/issue-17/target cargo run --release --locked --manifest-path evidence/issue17/slice25-journey-driver/Cargo.toml > "$out/native-journey.log" 2>&1; d=$?; printf 'journey_exit=%s\n' "$d"
target/release/aperture-cli.exe replay --file "$out/demo-replay.json" > "$out/native-replay.log" 2>&1; e=$?; printf 'replay_exit=%s\n' "$e"
python "$out/native-corpus-audit.py" > "$out/native-corpus-audit.log" 2>&1; f=$?; printf 'audit_exit=%s\n' "$f"
if (( a || b || c || d || e || f )); then exit 1; fi
