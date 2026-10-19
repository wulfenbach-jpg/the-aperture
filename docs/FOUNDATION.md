# Public foundation verification

This records the public-foundation setup, not a Commander release or a security certification.

## Existing Rust baseline

The existing six reachable commits were preserved and initially pushed at `3ad25b4cfafab8f90f7b6cbe68b02f41a6ce5dac`. No history rewrite, squash or force push was used. `git fsck --full` reported no corruption (two unreferenced/dangling historical commits were left untouched).

Fresh local execution before governance/board changes:

* `cargo fmt --check`: exit 0.
* `cargo test --all-targets --locked`: exit 0, **136 passed** across the reported test binaries.
* `cargo clippy --all-targets --locked -- -D warnings`: exit 0.

No Rust runtime source was changed by the public-foundation setup. Three-platform remote CI is configured separately; a configured or queued workflow is not a passing run.

## Publication hygiene

A heuristic scan inspected 246 unique reachable historical objects with paths for credential patterns. Three matches were synthetic credential-bearing URLs in rejection tests (`crates/ai/tests/decider.rs` and historical versions of `crates/import/tests/url_guard.rs`); no actual credential was identified. This limited scan is not a comprehensive security audit. Build/cache/model directories and Python caches are ignored. No downloaded model weights or local provider configuration were added.

Original source retains the workspace's MIT OR Apache-2.0 choice, now backed by both license texts. Third-party content is addressed separately in [ASSETS.md](../ASSETS.md). GitHub private vulnerability reporting was enabled and read back as enabled.

## Roadmap and real browser checks

The idempotent bootstrap's read-back verified **33 ownership-marked issues and six milestones**, including exact acceptance checklists and milestone titles. A second bootstrap created no duplicates. GitHub's issue-list index briefly lagged successful issue creation; a reproduced failing test now covers bounded verification retries rather than assuming writes are visible immediately.

The live board was exercised in an isolated headless Edge browser using a loopback HTTP server, not a fabricated page response. It displayed 33 real GitHub issues in six columns. Verified:

* M1 filtering showed five issues; real browser text input `#6` selected the correct live issue link.
* The 390-pixel mobile layout had no horizontal document overflow.
* Blocking the GitHub API produced a visibly dated real repository snapshot.
* Blocking both API and snapshot produced an explicit no-data state, not a stale success.
* Unblocking networking recovered the live board.
* No JavaScript runtime exceptions were observed.

Reproduction requires an isolated Chromium browser with a loopback CDP port and a loopback server serving `docs/`:

```text
node --test scripts/board.test.cjs
python -m unittest discover -s scripts -p 'test_*.py' -v
node scripts/board-browser-smoke.cjs http://127.0.0.1:9337 33 OUTPUT_DIR
```

The optional browser smoke defaults to `http://127.0.0.1:9876/`; its fifth argument can point to the published board. Node 22 or newer supplies the WebSocket client. Browser screenshots/reports and reviewer records are local verification artifacts, not bundled application assets. The issue-backed board does not claim GitHub Projects authorization, editing support or universal Magic coverage.
