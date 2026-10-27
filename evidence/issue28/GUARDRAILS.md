# #28 guardrails: false certainty and blunder labelling

Card: `Add guardrails against false certainty and blunder labelling` (task `t_df6dd2e0`).
Spec: `docs/design/move-review-spec.md` §6 (spec 1.0), with the payload from §3.4 and §4.
Branch: `wt/t_df6dd2e0` (worktree, master untouched).

## What landed

| path | what it is |
| --- | --- |
| `crates/ai/src/review.rs` | The review payload as data (§4.1–§4.8): `RankedReview`, `RankedCandidate`, `ReasonTag`, `EvidenceRef`, `ReviewText`/`TextKind`, `Uncertainty`, `CandidateLine`, `BudgetRecord`/`StopReason`, `Preference`, and the `SPEC_ID`/`MAX_LINE_PLIES`/`MIN_ORDERING_SAMPLES` constants. No ranking derivation and no presentation. |
| `crates/ai/src/review/honesty.rs` | The shared validation and formatting module: `audit_text`, `audit_review`, `audit_review_against`, `audit_sections`, the closed `HonestyViolation` set, the §4.6 reason registry plus required-evidence checks, and the single formatter (`format_reason`, `format_outcome`, `format_budget_line`). Its module header documents how to add a reason tag without breaking the boundary. |
| `crates/ai/tests/review_honesty.rs` | The CI-checkable guardrail suite: 27 tests across text, payload corpus, section structure, formatter output and a string-literal copy scan. |
| `crates/ai/tests/fixtures/review/*.json` | Representative payload corpus: 3 honest payloads, 7 known-bad payloads, each bad one naming a single defect. |
| `crates/core/src/coach.rs` | One-line additive change: `ExplanationKind` also derives `Deserialize` so `EvidenceRef::CoachExplanation` can round-trip through a fixture. No behaviour change. |

The review path is data + audit only. It has no UI dependency and no `search` dependency, which is
what lets the same module guard the panel, the evidence harness and the future corpus run.

## Why the boundary is enforced in three places

A scanner alone is easy to satisfy by accident, and a typed payload alone cannot see prose. The
suite therefore checks all three levels:

1. **Text** (`audit_text`): split on `[.!?;]`; percentages `\d+(\.\d+)?\s*%` are a violation with
   no negation excuse; probability/strength vocabulary and verdict labels are violations unless
   the same sentence carries a negation marker from `{no, not, never, none, nothing, neither}`.
2. **Payload** (`audit_review` / `audit_review_against`): every reason tag is in the §4.6 registry
   and its required evidence is present, every `EvidenceRef` resolves inside the payload (offer
   index exists, counts equal the fields they describe, the CR number is pinned and appears in the
   source rules, the line step exists in the recorded line), every printed score carries a sample
   count, the uncertainty state matches the sample count, a preference cannot contradict the
   scores, and a truncated analysis cannot claim reproducibility.
3. **Copy** (`review_facing_copy_strings_are_honest`): every string literal under `crates/ai/src`
   (plus `crates/core/src/coach.rs`, plus `crates/ui/src/review.rs` when the presentation task
   lands it) is audited, so a hard-coded "72% chance this is best" in a formatter fails CI.

## Acceptance criteria

| id | how it is met |
| --- | --- |
| AC-2 no fabricated probabilities | `percent_figure`/`probability_claim` in `audit_text`; fixture `bad_percent.json` (`72% chance this is best`) fails with both codes; percentages have no negation excuse; the budget line is asserted never to contain `%`. |
| AC-3 no blunder verdicts | `verdict_label` with the offending word named; fixture `bad_blunder.json` (`forced blunder`) fails; the two shipped disclaimers are pinned as *allowed* so a future tightening cannot delete them. |
| AC-4 every claim traceable | `untraceable_claim` for a sentence with no evidence, a tag whose evidence the payload lacks, an ordering claimed on one sample, a line on an unscored row, a preference that contradicts the scores, rule text with no CR citation, and a `reproducible: true` on a wall-clock stop. Fixtures `bad_no_evidence.json`, `bad_fake_tie.json`, `bad_reproducible.json`, `bad_unregistered_reason.json` cover the first, fifth, sixth and (with sources) the positional-vocabulary case. |
| AC-11 rules vs policy separate (structural half) | `TextKind` is a closed enum with a total mapping to a section; `audit_sections` rejects rule text in the policy section, policy/uncertainty/strategy text in the rules section, identical titles or source labels, and any payload sentence the presentation never placed. The rules section is produced only from `ExplanationKind::Rule`/`Simplification`; a `Strategy` record is never quoted as a rule (`a_strategy_record_may_not_be_quoted_as_a_rule`). |

Also enforced, because the spec's §4.6 procedure requires it: the enum and the spec table cannot
drift (`the_reason_registry_matches_the_spec_table` parses `docs/design/move-review-spec.md` §4.6
and compares it with the registry).

## How to run

```sh
cargo test -p aperture-ai --test review_honesty          # the guardrail suite
cargo test --workspace --all-targets                     # whole tree (guardrails included)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Demonstrated failure (RED), then green

`evidence/issue28/guardrails-red.log` records two probes applied to a passing tree and removed
again:

1. The honest fixture `honest_ranked.json` gained the acceptance sentence `72% chance this is
   best`:

   ```
   an honest payload must produce zero violations, got: [PercentFigure { sentence: "72% chance this is best" },
                                                          ProbabilityClaim { sentence: "72% chance this is best" }]
   exit=101
   ```

2. A review-facing source literal gained a verdict label, proving the copy scan is not decorative:

   ```
   review copy violates the honesty boundary:
   crates/ai/src/review.rs:369: verdict_label: verdict word "blunder" asserted in
   "forced blunder: the search had to pick something"
   exit=101
   ```

`evidence/issue28/guardrails-green.log` is the same suite after both probes were removed
(`27 passed; exit=0`).

## Adding a reason tag (short form; the module header has the full procedure)

One commit, five parts, or the suite fails:

1. a row in spec §4.6 with the required evidence and permitted text;
2. the enum variant in `crates/ai/src/review.rs` (the enum is closed: no `Other(String)`);
3. `required_evidence` and `required_evidence_present` arms;
4. the `format_reason` arm, using the spec's wording verbatim;
5. a positive *and* a negative fixture in `crates/ai/tests/fixtures/review/`.

Positional vocabulary (hanging piece, tempo, card advantage, board development, mana efficiency,
"punishes a mistake") stays unavailable until an engine trace establishes it — a new trace is its
own card. `EngineRuleApplies` is the only tag that quotes engine prose, and only a
`Rule`/`Simplification` explanation with a pinned CR number.

## Boundary decisions a reviewer should check

- `Preference` is defined here because §4.1 names the type but never describes it, while AC-9
  requires `preference: None` on a refusal. It carries exactly the two states §5.3 needs (unique
  maximum, real tie) and lives in an `Option`.
- Rule citations are `String`, not `&'static str`: §4.1 requires the payload to `Deserialize`, and
  a borrowed `'static` field cannot be deserialised without leaking fixture strings. The property
  the audit needs (a citation is the engine's own pinned rule number) is enforced by resolving
  against `coach::rule_note` and the source `LegalAction::rules`/`Explanation::rules` instead.
- The payload does not carry the reviewed offer. Engine-explanation evidence resolves through
  `audit_review_against(review, &AuditSources { offer, explanations })`; with empty sources only
  the refs that live inside the payload can be proven, and that limit is documented on the type.
- `chance` (whole word) is rejected in addition to the spec's `chance of winning`: the acceptance
  fixture depends on the shorter form and it is the same claim.
- The section audit needs the presentation's four label constants and the texts it actually placed,
  so AC-11 is completed where the panel is built; the guardrail owns the property, the UI owns the
  values (`SectionLabels`, `ReviewSections`).

## Known limits, stated rather than hidden

- The negation rule has a hole by construction: one sentence carrying both a negation and a label
  (`"this is not a blunder, it is the best move"`) passes. The mitigation is that `format_reason`
  is the only producer of review copy and emits no verdict vocabulary; the suite pins both halves.
- The copy scan covers review-facing sources only. It skips `crates/ai/src/review/honesty.rs`
  (the detector's own pattern table and negative fixtures live there) and audits the formatter's
  output instead; it does not scan `crates/ui/src/app.rs`, whose legitimate text-zoom copy contains
  percentages.
- `crates/ui/src/review.rs` does not exist on this branch yet; the scan picks it up automatically
  once the presentation task lands it.
- `audit_review` without `AuditSources` cannot prove engine-explanation references or reject a
  positional topic. Fixtures and the corpus harness must use `audit_review_against`.
