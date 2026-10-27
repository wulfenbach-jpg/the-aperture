# Review 294 — review-290 repair adjudication

Reviewed head: **`b74815d6690d76a7d838bc491fd37b1d791328d0`** (code `5b60c2c`; base `cff319d`).

**Verdict: CHANGES_REQUESTED** — the behavioural repair is correct, but one
verification receipt in the repair evidence is false. This is not a product
regression; it is a record-integrity defect of the kind review 290 required to
be corrected.

## Behavioural repair: confirmed

The delta is exactly the target-spec split:

- `TargetSpec::CreatureYouControl` now includes every controlled creature,
  including the ability source when it is a creature.
- `TargetSpec::AnotherCreatureYouControl` is the only spec that excludes the
  source; Little Bear uses it.
- Beorn's Hospitality uses the plain spec. The animated source is offered and
  auto-chosen as its own landfall target; the unanimated enchantment is not a
  creature target; a lone unanimated watcher has no legal target and is removed
  with `CR 603.3d`.
- The two existing slice-33 tests were updated for the legitimate landfall
  counter, not weakened.

## Checks run

| Check | Result |
|---|---|
| `cargo test -p aperture-core --test issue17_slice33 --locked` | **12/12 passed** |
| `cargo test -p aperture-core --test card_printings --locked` | **12/12 passed** |
| `cargo fmt --all -- --check` | exit 0 |
| `git diff cff319d..HEAD -- crates/` | exactly the spec split + label; no other rules change |
| Mutation receipts | source-exclusion mutation fails 4 tests; plain-spec-for-Little-Bear fails 1 — see defect below |
| Release CLI replay `slice33/demo-replay.json` | 1117 actions, 2967 events, digest `3dfb14d4ce76…` matches |
| Corpus audit | 149 catalog / 459 = 137 supported + 322 missing; per list `55/0/51/48/16/52/52/53`; 1/8 certified |
| `crates/ai`, `crates/ui`, deck snapshots | untouched by the delta |

`gates-final.log` reports workspace 846 passed / 0 failed / 1 ignored; I did not
repeat the full workspace matrix. The clippy/fmt logs were inspected, not
independently rerun beyond the fmt check above.

## Blocking record finding

`evidence/issue17/slice34/pre-mutation-sha256.txt` claims:

```text
b61dd29d78cfc23b347bd48ff7ac245394703b40d7c563bc31c3b9749c2b81e4 *crates/core/src/engine.rs
```

The actual committed/worktree engine at this head is:

```text
08988a7466a5fb53a57addc0c9c9f130256ee721f0034d9939a0b214782d6338  crates/core/src/engine.rs
```

`sha256sum -c evidence/issue17/slice34/pre-mutation-sha256.txt` exits 1 with
`FAILED`. `SLICE34-HUMAN-CHECKPOINT.md:41-42` therefore falsely states that the
mutation was restored byte-for-byte and that the receipt is clean. The current
worktree is not modified and the committed repair is correct, but the claimed
restore proof is not true for `engine.rs`.

Required correction: rerun the source-excluded mutation from the actual
pre-mutation copy, restore it, and record matching before/after hashes (or
remove the false `sha256sum -c` claim and state exactly what was compared).
No production or test code change is indicated by this finding.

## CI

Live `check-runs` for `b74815d` returned **10 runs**: `board and roadmap` twice
completed **success**; `rust (ubuntu-latest)`, `rust (windows-latest)`,
`rust (macos-latest)` and `windows package` are currently **in_progress**. These
in-flight runs are not passes.

## Residual scope

Original issue #17 remains unfinished: **1 of 8** certified 100-card lists and
**322 of 459** union names unsupported; Ayula is 16 names short. This is an
increment adjudication only — no full #17 acceptance, merge, master push,
closure, or downstream unlock.
