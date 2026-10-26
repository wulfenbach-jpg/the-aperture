# Slice 17 assertion strengthening

Head before this change: `bfaa782`. **No product source changed.** This is test-only
evidence; it does not add a new source verdict or a new TDD claim.

## Changes to `crates/core/tests/issue17_slice17.rs`
- No-death control now locates the actual entered Ulvenwald Bear on the battlefield and
  asserts exactly zero +1/+1 counters.
- Positive morbid test places the opposing Runeclaw Bear in the fixture **before** the
  cast, asserts the `Pending::TriggerTarget` offer contains that permanent, answers with
  it, and asserts the same object still exists with exactly two counters. The former
  `target.is_none()` escape is gone.
- New token-only death test kills a Bear token, verifies the nontoken owner count remains
  zero, and verifies the offered target receives exactly two counters.

A Fade-containing convenience test was not added; the three required assertions were
prioritized and the source/package was left untouched.

## Commands (real exit status)
- `cargo test -q -p aperture-core --test issue17_slice17` -> 6 passed, 0 failed.
- `cargo fmt --all` -> applied; `cargo fmt --all --check` -> exit 0 (`FMT_OK`).
- `cargo clippy -q --workspace --all-targets -- -D warnings` -> exit 0 (`CLIPPY_OK`).

No full suite, build or journey was rerun, as instructed.
