#!/usr/bin/env bash
# Gates for the #28 presentation layer (card t_808ff701).
#
# Every step prints its own exit status; the output is committed as evidence, so
# a reader can see the commands and their codes rather than a hand-written
# "all green". Long runs: invoke with background=true from the Hermes terminal
# and poll the process — piping cargo into grep block-buffers the log.
set -u

cd "$(dirname "$0")/.." || exit 1

echo "# move-review render gates"
echo "# utc:    $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "# head:   $(git rev-parse HEAD)"
echo "# branch: $(git rev-parse --abbrev-ref HEAD)"
echo "# rustc:  $(rustc --version)"
echo

echo "## cargo fmt --all -- --check"
cargo fmt --all -- --check
echo "fmt_exit=$?"
echo

echo "## cargo clippy --all-targets --locked -- -D warnings"
cargo clippy --all-targets --locked -- -D warnings
echo "clippy_exit=$?"
echo

echo "## cargo test -p aperture-ui --all-targets --locked"
cargo test -p aperture-ui --all-targets --locked
echo "ui_tests_exit=$?"
echo

echo "## cargo test --all-targets --locked"
cargo test --all-targets --locked
echo "workspace_tests_exit=$?"
