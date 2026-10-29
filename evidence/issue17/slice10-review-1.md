# Slice 10 — independent review and repair

Reviewer: independent subagent with its own fresh context and its own probe
directory (no implementation edits, no branch changes). Scope: the slice-10 diff
`ab9d4f5..8471e7d` plus the new `certify` command, the certificate, the manifest
record and the docs claims. Verdict: **request changes**.

## Findings and disposition

| id | finding | disposition |
|---|---|---|
| S10-R1-F1 | `crates/cli/src/certify_cmd.rs`: the coverage ledger iterated `deck.cards` only, so the separately supplied commander was never counted, while `unique_names_total` already included it. Published `unique_names_observed = 54` was one short (true value 55) and `54 + 6 = 60 ≠ 61`. Propagated to the manifest, docs, HANDOFF, the checkpoint and `certify.log`. No test caught it (the CLI test only asserted `> 0`). | **Repaired.** Coverage is now derived from the same per-card rows the certificate publishes (commander included), so `observed + not_observed == unique_names_total` by construction. Certificate regenerated: **55/61**. Manifest, `docs/issue17-starter-corpus.md`, `HANDOFF.md`, `SLICE10-HUMAN-CHECKPOINT.md` and `certify.log` corrected. A new CLI regression asserts the arithmetic and that the commander appears in the ledger. |
| S10-R1-F2 | `crates/cli/src/main.rs` help text promised exit code 2 for an invalid request, but both refusal paths return exit 1 through `anyhow`. | **Repaired in prose, not by inventing a code**: the help now says exit 0 = certified and any other outcome exits non-zero with the reason on stderr, which is what the binary actually does. Refusal behaviour itself was correct and unchanged. |
| S10-R1-F3 | `docs/issue17-starter-corpus.md` said the exact snapshot "is 99 supported singleton cards with exactly one legendary card", conflating the 99-card library (which holds no legend) with the 100-card snapshot (whose single legend is the commander). | **Repaired.** The prose now distinguishes the exact library from the whole 100-card snapshot; the test already checked the union correctly. |

## What the reviewer verified as true (by execution)

* Recomputed `sha256(decks/starter-corpus/jasmine-vanilla-stompy.txt)` matches the
  manifest, certificate and docs.
* Re-running seeds 1,2,3 reproduces `evidence/issue17/slice10/certification.json`
  **byte-identically**; games are four-seat, 40 life, terminal, replay-verified,
  winners P1/P2/P3 with the recorded steps and digests.
* No bundled fallback: a missing deck path exits non-zero with "refusing to
  substitute a bundled practice deck"; an edited snapshot is refused at parse
  time; no output file is written on failure.
* `crates/core/src`, `crates/ai`, `crates/import/src`, `crates/ui` are untouched
  by the diff; the catalog is unchanged; an independent scrape of the desired
  lists reproduces union 459 = 102 supported + 357 missing; planned lists are
  byte-identical; `main.rs` is additive-only.
* Tests pass: slice-10 core 3, CLI certify 3, `starter_corpus` 8,
  `starter_corpus_planned` 1.
* Reviewer left no tracked modifications (`git status --porcelain` clean apart
  from pre-existing untracked evidence files).

## Reviewer's stated limits

It did not re-query Scryfall for the 47 vanilla printings (the checked-in
fixture and `starter_corpus.rs` were re-run instead), and it noted that the
certificate's provenance rests on byte-exact reproduction rather than a signed
CI artifact — which is why the reproduction command is recorded in the manifest.

## Repair verification

`cargo test -p aperture-core --test issue17_slice10_certification` 3 passed,
`cargo test -p aperture-cli --test certify --release` 3 passed,
`cargo test -p aperture-core --test starter_corpus` 8 passed,
`cargo test -p aperture-import --test starter_corpus_planned` 1 passed,
`cargo fmt --all --check` and
`cargo clippy --workspace --all-targets --locked -- -D warnings` clean.
The full workspace run before the repair was 660 passed / 0 failed; the repair
touched only `certify_cmd.rs`, its test, the manifest coverage count and prose,
so the affected suites above are the delta.

## S10-R1-F4 — found by exact-head CI, not by the reviewer

`rust (windows-latest)` failed at `67eb5e7` on
`the_manifest_certification_record_matches_the_real_snapshot`:

```
left:  "4312ab2a90a94a4f9c9578b330a093dcec7eb1aa253785c97667cb0fab967491"
right: "1fbaf76e10320beeea6b69fb5b2cadcf4437040a63eb954f3b06dce52d53d25c"
```

The test and the command hashed **raw file bytes**, so a checkout that converts
the tracked LF file to CRLF (the Windows runner) produced a different digest.
That is a portability defect in the certificate rule, not in the accepted list:
a certificate must identify the list's content, not the checkout's line endings.

**Repaired.** Both the command and the conformance test now hash the snapshot's
UTF-8 text with CRLF normalized to LF, and the digest basis is recorded in the
manifest (`snapshot_hash_basis`) and the docs. Control:
`evidence/issue17/slice10/crlf-normalization-control.log` certifies a CRLF copy
of the same deck (built on purpose) at the identical hash `4312ab2a…`, and the
LF working tree still certifies unchanged. Linux and macOS CI passed at the
same head, which is consistent with this being a line-ending-only defect.
