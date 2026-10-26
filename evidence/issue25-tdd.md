# Issue 25 TDD evidence — local model boundary

Executed in `C:/AI/aperture-worktrees/issue-25`, branch `aperture/issue-25`,
base `03f4802`. No model service was started and no weights were used. Loopback
responders are protocol fixtures (fixed typed-readout documents), never model
inference.

## Slice 1 — masked observation completeness (true RED, then GREEN)

Regression: `crates/ai/tests/decider_observation.rs`

RED command:

```
cargo test -p aperture-ai --test decider_observation
```

Actual RED (2 failed):

```
the_request_exposes_public_exile_command_and_card_details ... FAILED
  panicked at crates\ai\tests\decider_observation.rs:25:5:
  assertion `left == right` failed
    left: Null
   right: "Shock"

the_observation_does_not_hardcode_a_twenty_life_game ... FAILED
  panicked at crates\ai\tests\decider_observation.rs:57:5:
  the request must not assert a 20-life profile:
  {"game":"Magic: The Gathering practice slice (2-4 live seats, 20 life)", ...}
```

Change: the request now carries public exile/command-zone/commander-damage data,
permanent counters/damage/pump and printed `oracle` text, opponent public
name/deck name, and no starting-life assertion.

GREEN command and result:

```
cargo test -p aperture-ai --test decider_observation
test result: ok. 2 passed; 0 failed
```

## Slice 2 — fail-closed protocol schema (true RED, then GREEN)

Regression: `crates/ai/tests/decider_schema.rs`

To make the cardinality bound importable for direct request tests,
`MAX_OPTIONS`/`MIN_OPTIONS` were exported first; the behavioral run was:

```
cargo test -p aperture-ai --test decider_schema
```

Actual RED (4 failed):

```
missing_or_malformed_usage_is_never_treated_as_clean ... FAILED
  a malformed usage schema must not be accepted:
  ChoiceAnswer { key: "a", label: "Pass priority", answer_confidence: 0.6, confidence: 0.0, model: None }

invalid_confidence_is_refused_instead_of_defaulted_to_zero ... FAILED
  an invalid confidence must not be accepted:
  ChoiceAnswer { key: "a", label: "Pass priority", answer_confidence: 1.5, confidence: 0.2, model: None }

an_oversized_protocol_response_body_is_refused ... FAILED
  an oversized response body must be refused:
  ChoiceAnswer { key: "a", label: "Pass priority", answer_confidence: 0.6, confidence: 0.0, model: None }

choose_enforces_cardinality_and_unique_keys_for_directly_built_requests ... FAILED
  assertion failed: matches!(client.choose(&one), Err(DeciderError::TooFewOptions(1)))
```

Change: usage schema is required and strictly typed; present confidence fields
must be finite and inside `[0, 1]`; the response read is bounded to 256 KiB;
`choose` validates 2..=90 options, non-empty keys and unique keys before any
request.

GREEN command and result:

```
cargo test -p aperture-ai --test decider_schema
test result: ok. 4 passed; 0 failed
```

## Slice 3 — envelope binding, staleness and offered membership (interface RED, then GREEN)

Provenance limit: the first boundary run below was a missing-API compilation
failure, not a behavioral RED. The six initial boundary assertions are GREEN
coverage, not evidence of test-first behavioral failure. Later correction-slice
staleness and fallback-preference assertions did fail behaviorally before their
repairs.

Regression: `crates/ai/tests/decider_boundary.rs`

RED command:

```
cargo test -p aperture-ai --test decider_boundary
```

Actual RED: the new typed API did not exist yet:

```
error[E0432]: unresolved import `aperture_ai::DecisionBinding`
error[E0432]: unresolved import `aperture_ai::DecisionEnvelope`
error[E0599]: no method named `reason_code` found for enum `DeciderError`
error[E0599]: no variant named `IllegalChoice` found for enum `DeciderError`
error: could not compile `aperture-ai` (test "decider_boundary")
```

Change: `DecisionBinding` + `DecisionEnvelope` bind viewer/revision/decision/
generation to an immutable offered-action snapshot; `DeciderPolicy::decide_bound`
validates the binding before contacting the service, resolves the returned key
against the snapshot, and revalidates the resolved action against the live legal
offer. `DeciderError::reason_code` exposes stable codes.

GREEN command and result (first run caught a fixture-planning mistake: the two
positions shared an object id, so no offered action differed; the offered view
was given two Forests to make the snapshots genuinely distinct):

```
cargo test -p aperture-ai --test decider_observation --test decider_schema --test decider_boundary
crates/ai/tests/decider_boundary.rs: test result: ok. 6 passed; 0 failed
crates/ai/tests/decider_observation.rs: test result: ok. 2 passed; 0 failed
crates/ai/tests/decider_schema.rs: test result: ok. 4 passed; 0 failed
```

The first boundary GREEN attempt failed only because the test fixture's
`ObjectId`s coincided (`the two positions must differ in at least one offered
action`), not because the boundary accepted an illegal action.

## Existing regressions (GREEN, unchanged assertions)

```
cargo test -p aperture-ai --test decider --test decider_usage --test decider_redirect
decider: 4 passed
decider_usage: 5 passed
decider_redirect: 1 passed
```

## UI boundary diagnostics

`crates/ui/src/session.rs` now sends a bound snapshot to the default decider
worker and labels discarded completions with `stale_generation`,
`stale_revision`, `stale_decision`, `wrong_seat` or `illegal_choice`. New
regression `an_ai_completion_outside_the_live_offer_is_discarded_with_a_reason`
in `crates/ui/tests/import_and_async.rs`.

```
cargo test -p aperture-ui --test import_and_async
test result: ok. 12 passed; 0 failed

cargo test -p aperture-ui --test session
test result: ok. 19 passed; 0 failed
```

## Real-engine example

```
cargo run -p aperture-ai --example model_decide
```

Observed output:

```
valid fixture: decider-2b accepted by the engine -> Mulligan { keep: true }; model protocol-fixture chose Keep hand (7 cards) (preference 0.620; not a win probability) (revision 1)
truncated fixture: heuristic (decider fallback) accepted by the engine -> Mulligan { keep: true }; decider-2b unavailable, used the heuristic [truncated]: ... (revision 1)
stale envelope [stale_generation]: heuristic (decider fallback) -> Mulligan { keep: true }; engine unchanged at revision 0
stale envelope [stale_revision]: heuristic (decider fallback) -> Mulligan { keep: true }; engine unchanged at revision 0
```

The fixture listener is stopped and joined on drop. All responses are protocol
fixtures; no real checkpoint was available, so real-model inference is
unverified.

## Formatting and lints

```
cargo fmt --all -- --check   -> FMT_OK
cargo clippy -p aperture-ai --all-targets -- -D warnings -> Finished, no warnings
```


## Correction slice — director review fixes (RED, then GREEN)

Focused commands only; the unchanged search suite was not run.

### 1. Default async UI on a one-option view

RED:

```
cargo test -p aperture-ui --test import_and_async \
  the_default_async_decider_reports_a_single_legal_option_as_forced_without_http
```

Actual RED:

```
the forced path must not be reported as a prepare fallback:
no decision request sent [too_few_options]: only 1 legal option(s): the choice is forced, not a model question
```

Fix: `request_ai_decision_default` checks `MIN_OPTIONS` before preparing an
envelope; a single legal action returns `DecisionSource::Forced` with no HTTP
call. GREEN: 1 passed; the test also asserts the loopback fixture was never
contacted.

### 2. Fallback preference must be cleared

RED (boundary suite):

```
assertion `left == right` failed: the rejected proposal's concentration must not be attached to the heuristic action
  left: Some(0.6)
 right: None
```

Fix: `fallback_decision` always sets `preference: None`; no call site passes the
rejected proposal's concentration.

### 3. Binding staleness before forced handling

RED:

```
assertion `left == right` failed: stale_generation must win over the forced choice:
Some("only 1 legal option(s): the choice is forced and no model call was made")
  left: Forced
 right: DeciderFallback
```

Fix: `decide_bound` validates the envelope binding first, then handles a forced
live choice. New tests cover stale generation and wrong seat on a one-option view
and assert no request/reason loss.

### 4. Non-choice decision type is malformed

RED:

```
a score answer is not a supported choice:
ChoiceAnswer { key: "a", label: "Pass priority", answer_confidence: Some(0.6), confidence: None, model: None }
```

Fix: a present `answers.decision.type` must be `choice`; a present non-choice
type is refused as `malformed_response`. An absent type remains accepted.

GREEN results:

```
cargo test -p aperture-ai --test decider_boundary   -> 7 passed
cargo test -p aperture-ai --test decider_schema     -> 5 passed
cargo test -p aperture-ui --test import_and_async   -> 13 passed
cargo fmt --all -- --check                           -> FMT_OK
cargo clippy -p aperture-ai --all-targets -- -D warnings -> clean
```
