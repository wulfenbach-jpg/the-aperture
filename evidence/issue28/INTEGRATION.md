# #28 integration record — ranking + honesty + presentation onto draft PR #56

Task `t_cf764bc5` (aperture-supervisor order, 2026-10-27T07:05Z). This is an **integration**, not
acceptance of #28: `full_issue28_accepted=false`. Criterion 4 (teaching-quality validation on a
reviewed corpus) remains blocked on #17's certified starter lists (1 of 8) and is not in this
slice. Nothing here claims AI strength, playable Commander, supported cards, latency or a release
percentage.

## Where the work went

- Checkout: `C:/AI/aperture-worktrees/issue-28`, branch `aperture/issue-28`.
- Base: `48e3334` (slice 1, independently APPROVED, exact-head CI 10/10 — that approval covers
  slice 1 only).
- Draft PR: https://github.com/wulfenbach-jpg/the-aperture/pull/56 (OPEN, draft, base `master`
  `f3e8bb1`). One feature, one PR: no second PR, no branch as the deliverable, no merge, no
  un-draft, no issue closure, `master` untouched.

## Source heads folded in

| lane | branch | head | what was taken |
|---|---|---|---|
| ranking | `wt/t_f4baa891` | `b9f8f50` | `crates/ai/src/review.rs` (1123 lines, spec-1.2 payload + `rank`/`rank_outcome`/`refused`/`reason_text`), `crates/ai/src/search.rs` bounded rollout recording, `crates/ai/tests/review.rs`, `docs/design/move-review-spec.md` v1.2 |
| guardrails | `wt/t_df6dd2e0` | `013277f` | `crates/ai/src/review/honesty.rs` (audit + closed `HonestyViolation` + formatters + `audit_sections`), `crates/ai/tests/review_honesty.rs`, the 10-payload fixture corpus |
| presentation | `wt/t_808ff701` | `f4c0814` | `crates/ui/src/review.rs` (renderer over `aperture_ai::review`), `crates/ui/src/app.rs`, `crates/ui/src/session.rs`, `crates/ui/src/lib.rs` |

`f4c0814`'s `crates/ai` and `docs` content is byte-identical to `b9f8f50`'s (verified with
`git diff b9f8f50 f4c0814 -- crates/ai crates/core docs`, empty), so the presentation lane's copy of
the crate was not a fourth variant. All three branches carried the **same** one-line
`crates/core/src/coach.rs` change (`serde::Deserialize` on `ExplanationKind`); it landed once.

## What is canonical, and why

`wt/t_f4baa891:crates/ai/src/review.rs` is the canonical payload: it is the newest spec version
(1.2), it is what `search` actually feeds through `rank_outcome`, and `rank`/`rank_outcome`/
`refused` are the derivation layer the UI already calls. The guardrails branch's second
`review.rs` (spec-1.0 shape) is retired, not merged: two payloads for one contract is exactly the
drift the spec's §10 warns about.

The two implementations differ in more than naming. Recorded as §4.10 of the spec (informative,
within 1.2 — no decision D-1…D-11 and no criterion AC-1…AC-17 changes):

1. `StopReason` is owned by `search` (the guardrails draft declared its own copy); `MAX_LINE_PLIES`
   likewise.
2. `Preference` is `Unique` / `AmongEquals { chosen, offer_indices }` / `NoOrder { chosen }`
   (the draft had `Unique` / `Tied { offer_indexes }` only, so a scored single-sampled head had no
   honest shape).
3. `ReasonTag::EngineRuleApplies.rule`, `EvidenceRef::RuleCitation.number` and
   `EvidenceRef::Budget.field` are `Cow<'static, str>` (the draft used `String`).
4. `RankedCandidate` carries `rollouts_attempted`; `RankedReview.spec` is `Cow<'static, str>`.
5. The payload enums serialise `snake_case` (the draft's enums carried `#[serde(rename_all)]`; the
   canonical enums did not, while the fixture corpus is JSON). Added to `StopReason`, `LineScope`,
   `Uncertainty`, `Preference`, `ReasonTag`, `TextKind`, `EvidenceRef`.

## Adaptations, by name

### `crates/ai/src/review.rs` (canonical, extended — no derivation rewritten)

- `pub mod honesty;` — the audit lives beside the types it audits.
- `RankedReview::candidate(offer_index)` and `RankedReview::is_refusal()` — accessors the audit and
  the UI both need; the draft declared them, the canonical module did not.
- `Preference::offer_indices()` and `Preference::chosen()` — total over the three shapes.
- `BudgetRecord::reproducible_flag_is_consistent()` — §3.3's consistency check for a loaded
  payload.
- `#[serde(rename_all = "snake_case")]` on the five payload enums (see §4.10.3 above).
- The module header said "spec 1.1" while `SPEC_ID` was already `move-review/1.2`; corrected, and
  the header now names the honesty half.

### `crates/ai/src/search.rs`

- `StopReason::is_reproducible(cancelled)` is now the **single** definition of §3.3's rule; the
  analysis builds its flag with it and `BudgetRecord` checks against it. Behaviour unchanged (same
  expression), but the rule is no longer duplicated in two crates' worth of code paths.
- `#[serde(rename_all = "snake_case")]` on `StopReason` so the fixture corpus can round-trip.

### `crates/ai/src/review/honesty.rs` (ported, algorithm intact)

- Imports re-pointed: `StopReason`, `MAX_LINE_PLIES` from `crate::search`; `Preference` from the
  canonical module.
- `explanation_cites(&Explanation, &str)` replaces three `rules.contains(&rule.as_str())` sites:
  `Explanation::rules` is `Vec<&'static str>` and the payload's citations are `Cow<'static, str>`,
  so the comparison is by content (`iter().copied().any(...)`, which also satisfies
  `clippy::manual_contains`).
- `EvidenceRef::Budget { field }` literals became `"stop".into()`, `"cancelled".into()` (the field
  is `Cow`); the resolve arm coerces `&Cow` to `&str` explicitly.
- `audit_preference` now audits all three shapes: `Unique` requires the named row to carry at least
  `MIN_ORDERING_SAMPLES` samples and no other row to match or beat it; `AmongEquals` requires each
  named member to be `EffectivelyTied`, level with the top, with `chosen` inside the group and the
  row itself marked `chosen`; `NoOrder` checks only the recorded selection.
  `NoOrder { chosen: None }` is legitimate and is not reported as "names no candidate".
- Everything else is the ported module verbatim: `audit_text` (percent figures with no negation
  excuse; probability/verdict vocabulary only inside a `{no, not, never, none, nothing, neither}`
  sentence), `score_without_sample_count`, `required_evidence_present`, `resolve`,
  `format_reason`/`format_outcome`/`format_budget_line`, `payload_texts[_with]`,
  `audit_review[_against]`, `AuditSources`, `audit_sections`, `SectionLabels`, `ReviewSections`,
  the closed `HonestyViolation` set and `HonestyViolation::code()`.

### Fixture corpus (`crates/ai/tests/fixtures/review/`, 10 payloads)

Kept in full; adapted by name only, no payload deleted:

- `spec` → `move-review/1.2` in all ten.
- each candidate gained `rollouts_attempted` (= scored + without-result + refused), a canonical
  field the draft payload lacked.
- `preference: { tied: { offer_indexes } }` → `among_equals: { chosen, offer_indices }` in seven.
- `honest_single_offer.json`: `unique` → `no_order: { chosen: 0 }`, because §4.9.1 makes a
  single-sampled head a `NoOrder`, and the fixture's own note already said "not a preference".

### `crates/ai/tests/review_honesty.rs` (27 tests, all kept)

- `ReasonTag::EngineRuleApplies { rule: ... }` literals became `"117.1a".into()` / `"999.9".into()`
  (`Cow`), the only textual change. The source-copy scan now also covers the landed
  `crates/ui/src/review.rs`, which it was written to do when the panel arrived.

### `crates/ui/src/**` (presentation, folded onto slice 1)

`f4c0814`'s `app.rs`/`session.rs` are a strict evolution of slice 1's (same tests, adapted to the
payload; plus the masked-view-digest discard test and the priority/pending invariance assertions),
so the presentation versions replaced slice 1's rather than being merged hunk by hunk. Verified by
reading the full `48e3334 → f4c0814` diff of both files: nothing slice 1 asserted was dropped.

Two changes on top of `f4c0814`:

- `crates/ui/src/review.rs`, `FORBIDDEN_FRAGMENTS`: the test-local banned-phrase table held the
  literal `"chance of"`, which the ported honesty copy scan rejects as a `probability_claim` in its
  own right. It is now `"chanc"` with a comment saying why. (This is exactly the kind of collision
  that only shows up once the two lanes live in one tree: the presentation branch never ran the
  guardrail suite.)
- No production behaviour was changed in the renderer.

### `docs/design/move-review-spec.md`

v1.2 from `b9f8f50` (735 lines) plus an informative **§4.10 Integration resolutions** section. The
guards branch's 1.0 copy is not shipped. One spec file in the tree.

## Slice-1 non-blocking finding: closed, with retained RED

The finding was that deleting the tie qualifier (`ties_top = false`) still passed the slice-1
suite, because the assertion only required the substring "the search's own choice".

- The renderer now asserts `"the search's own choice among equally scored offers"` **positively** in
  `tied_rows_render_as_one_group_that_is_not_ordered_inside`, while
  `a_single_sample_row_says_it_is_not_ordered` asserts the qualifier is *absent* for a single
  sample.
- Proof that the assertion has teeth: `row_facts` was reduced to always print
  `" · the search's own choice"` in a scratch edit, and that test failed
  (`test result: FAILED. 0 passed; 1 failed`), then the file was restored byte-identically before
  the gate run. Retained transcript: `evidence/issue28/mutation-red-tie-qualifier.log`.

## Tests: counted from the diff, not from memory

`#[test]` occurrences, `git show 48e3334:<file>` versus the pushed head:

| file | base | head | delta |
|---|---|---|---|
| `crates/ai/tests/review.rs` | — | 17 (16 run, 1 `#[ignore]`d subprocess test) | +17 |
| `crates/ai/tests/review_honesty.rs` | — | 27 | +27 |
| `crates/ui/src/review.rs` | 10 | 15 | +5 |
| `crates/ui/src/session.rs` | 7 | 8 | +1 |
| **workspace total** | **586** | **636** | **+50** |

The ranking handoff's "16 tests" is the count of **executing** tests in that file; it declares 17.

## Gates at the code head (`b77603b`)

`evidence/issue28/gates-b77603b.log`, produced by `evidence/issue28/run-gates.sh`, run from the
worktree root with one cargo invocation at a time. Each command's own exit code is in the log:

| step | command | exit |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | `fmt_exit=0` |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | `clippy_exit=0` |
| tests | `cargo test --all-targets --locked` | `test_exit=0` |
| release | `cargo build --release --locked` | `release_exit=0` |

`SUMMARY fmt=0 clippy=0 tests=0 release=0` / `GATES_PASS`. The run reports
**634 passed, 0 failed, 2 ignored** across 85 test binaries. The ignored pair is this feature's
`subprocess_emit_ranking` (out-of-process determinism, `crates/ai/tests/review.rs`, because it
re-executes the test binary) and the pre-existing network test
`a_real_archidekt_commander_deck_is_certified_with_exact_unsupported_cards` in `crates/import`;
636 `#[test]` declarations minus those two is the executed total.

Two runner facts worth recording, because both produced failed logs before this one:

- a killed gate run leaves its `cargo`/`rustc` children linking into `target/`, so the next run
  dies with the spurious `LNK1104: cannot open file ... search-<hash>.exe`. The failed attempt is
  not a build break; the run above was taken with no other cargo process in this checkout.
- `cargo fmt --check` needs the `--all` form used here: a single-crate fix after the first attempt
  left a chain rustfmt wanted broken, and the first gate log correctly caught it
  (`fmt_exit=1`).

## Exact-head CI

Recorded in the summary below and in the card comment: the pushed head is `b77603b`'s successor
`<evidence head>`, whose only difference from `b77603b` is this file and the gate log. CI at the
code head is the run that certifies the code; no green was inherited from `48e3334`.

## Open items

1. **Criterion 4 is not met and is not claimed.** Teaching-quality validation needs #17's certified
   starter lists; 1 of 8 exist. `full_issue28_accepted=false`.
2. **The renderer does not read `RankedReview::preference`.** It takes each row's ordering claim
   from `Uncertainty` (the payload's own per-row statement, §4.3) and numbers rows only where that
   permits. The top-level `preference` is checked against the rows by `audit_preference`, so the two
   cannot diverge in a payload that passes the audit — but a renderer test that pins that agreement
   does not exist yet. Recorded in spec §4.10 rather than silently assumed.
3. **Interactive latency is unmeasured.** The panel's own render journey runs in the viewport
   tests; no timing claim is made, and none should be read into this record.
4. **One-sample and tie coverage is partial on real games.** The ranking suite exercises both
   states; a real-game journey that lands on a genuine tie or a single-scored-sample head was not
   produced here. The fixture corpus and the renderer tests are the evidence for those paths.
5. **Claude/codex-independent review of this combined head has not run yet.** Slice 1's approval
   does not cover this code; a reviewer must be bound to the exact head SHA reported on the card.
6. The honours/adjudication record for the 1.0-vs-1.2 payload decision is this document plus spec
   §4.10; the guardrails branch's `evidence/issue28/GUARDRAILS.md` was **not** imported, because its
   counts and file references describe a tree that no longer exists.
