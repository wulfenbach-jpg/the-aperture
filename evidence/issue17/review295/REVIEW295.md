# Review 295 — review-294 correction re-adjudication

Reviewed head: **`bfbbceb6e023644d080d980e91d922babaaca60c`** (base `b74815d`).

**Verdict: APPROVED_INCREMENT** — correction accepted; this remains an increment
only, not full #17.

## Checks run

| Check | Result |
|---|---|
| `git diff --stat b74815d..bfbbceb` | evidence/docs only; no `crates/`, `decks/` or fixture bytes |
| `sha256sum -c pre-mutation-sha256.txt` | **exit 0** — `08988a74…d6338` matches committed `engine.rs` |
| `sha256sum -c post-mutation-sha256.txt` | **exit 0** — same `08988a74…` |
| `sha256sum -c post-mutation-ability-sha256.txt` | **exit 0 with GNU sha256sum** — `dff6b60d…a839` matches committed `ability.rs` |
| `pre-mutation-ability-sha256.txt` vs post | both `dff6b60d…a839`; pre/post pairs agree |
| `red-mutation-source-excluded.log` | 4 failed, including both self-target and the two P/T tests |
| `red-mutation-another-spec-dropped.log` | exactly Little Bear’s `little_bear_keeps_the_printed_another_exclusion` fails |
| `cargo test -p aperture-core --test issue17_slice33 --locked` | **12/12 passed** |
| Product delta since `b74815d` | none |
| CLI journey | prior evidence still consistent: 1117 actions / 2967 events / `3dfb14d4ce76…`; no product change |

The local shell’s `sha256sum` builtin reports a false failure for
`ability.rs`; the Git/GNU implementation exits 0 on the same receipt and direct
hash, so this was treated as an environment artifact, not a record defect.

## Record correction

`SLICE34-HUMAN-CHECKPOINT.md` now states that the earlier engine receipt was
written before `cargo fmt` reformatted the file and was stale; it records the
regenerated `pre-`/`post-` hashes (`08988a74…` for `engine.rs`,
`dff6b60d…` for `ability.rs`) and the re-run RED logs. That is an honest
correction of review 294’s finding.

## CI

Live `check-runs` for `bfbbceb` returned **10 runs**: `board and roadmap` twice
completed **success**; `rust (ubuntu-latest)`, `rust (windows-latest)`,
`rust (macos-latest)` and `windows package` are **in_progress** (8 runs). These
are not passes. The previous head `b74815d` has the writer’s completed capture
10/10 SUCCESS keyed to `b74815d`.

## Residual gaps

Original #17 remains unfinished: **1 of 8** certified 100-card lists, **322 of
459** union names unsupported, Ayula 16 short. No full #17 acceptance, merge,
master push, closure or downstream unlock.
