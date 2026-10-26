# Optional local decision model

The local model path is an **optional** adapter around the deterministic baseline
heuristic. It is off by default, ships no weights, starts no service, downloads
nothing and never requires a network. The engine remains the final authority for
rules, legal actions and state mutation.

## What the model sees

`decision_request(&PlayerView)` builds one masked observation from the acting
seat's projection only. It contains:

- live life totals keyed by absolute seat, turn/step/priority and seat count;
- the viewer's hand, battlefield, graveyard, exile, command zone and mana pool;
- each opponent's public name/deck name, life, hand/library counts, battlefield,
  graveyard, exile and command zone;
- public commander identities, zones, command-zone cast counts/tax and the
  commander-damage ledger;
- per-permanent damage, tapped/summoning state, power/toughness, pump, +1/+1
  counters and printed `oracle` text (the only keyword source the masked view
  carries; keywords are not inferred);
- the stack, pending decision prose, viewed scry/surveil cards and the
  engine-generated legal-action labels.

It never contains library contents, another seat's hand contents or any other
hidden zone. It does not assert a starting-life profile: `PlayerView` does not
expose one, so the request reports current life only rather than hardcoding a
20-life game.

## Binding an answer

The installed typed readout (`POST /v1/systemone`) has no field for custom
correlation ids, so the adapter binds locally:

```rust
let binding = DecisionBinding { viewer, revision, decision, generation };
let envelope = DecisionEnvelope::prepare(&view, binding)?;
let decision = policy.decide_bound(&view, &envelope, current_binding);
```

`DecisionEnvelope` holds the masked request plus an immutable snapshot of the
engine-offered actions (`envelope.offered()`). A returned opaque key is resolved
against that snapshot, then the resolved action is checked against the **live**
legal offer before application. `decide_bound` refuses the answer when the
viewer, revision, decision serial or session generation differs from the values
observed when the envelope was prepared.

`Policy::decide` remains available for synchronous callers. It cannot see
revision/decision serials in `PlayerView`, so it uses generation 0 and zeroed
binding values; generation-aware callers use `decide_bound`. The UI keeps its
independent worker generation guard and discards obsolete completions before any
fallback is executed in a replacement session.

## Fail-closed protocol validation

The client refuses, with a stable `DeciderError::reason_code()` and a visible
fallback note:

| Condition | Reason code |
|---|---|
| endpoint is not a credential-free loopback `http://` URL | `bad_endpoint` |
| no legal options / fewer than 2 / more than 90 | `no_options`, `too_few_options`, `too_many_options` |
| empty or duplicate option key on a directly built request | `empty_option_key`, `duplicate_option_key` |
| transport/offline failure | `transport` |
| non-success HTTP status | `http_status` |
| response body over 256 KiB | `response_too_large` |
| unparsable JSON/UTF-8, or an `answers.decision.type` present but not `choice` | `malformed_response` |
| missing usage object or missing/wrong-typed `truncated`, `state_tokens_dropped`, `truncated_questions` | `malformed_usage` |
| present `answer_confidence`/`confidence` that is non-numeric, non-finite or outside `[0, 1]` | `invalid_confidence` |
| no choice / key not offered | `no_choice`, `unknown_key` |
| response-indicated truncation | `truncated` |
| envelope for another viewer | `wrong_seat` |
| session generation / revision / decision changed | `stale_generation`, `stale_revision`, `stale_decision` |
| resolved action is no longer in the live legal offer | `illegal_choice` |

A missing confidence field is treated as unreported, not as a reported zero.
An absent `answers.decision.type` is accepted for installed-protocol
compatibility; a present non-`choice` type (for example `score`) is refused as
malformed. No probability-distribution normalization is assumed without
protocol evidence. Redirects are not followed, environment proxies are not
used, and the endpoint is restricted to loopback.

## Fallback

Every refusal falls back to `HeuristicPolicy` over the **current masked legal
view**. The fallback is deterministic, legal and labelled:

```
decider-2b unavailable, used the heuristic [truncated]: ...
```

A single legal action is reported as `DecisionSource::Forced` and is never sent
to the model (the typed readout requires at least two criteria). The UI default
worker checks this before preparing an envelope, so a forced move is not
misreported as a fallback. Binding staleness is checked before the forced choice:
a stale forced envelope keeps `stale_generation`/`stale_revision`/`stale_decision`
or `wrong_seat` as its reason instead of losing it to `Forced`.

On every fallback path, `Decision.preference` is `None`: a rejected model
proposal's concentration is never attached to the heuristic action that is
actually applied.

## Preference is not win probability

`answer_confidence` is **preference concentration** for the chosen option, stored
as `Decision.preference` and labelled "not a win probability" in visible notes.
It is neither a calibrated win probability nor a search evaluation. Information-
set search reports its own practice-outcome estimates separately and is not
mixed into this value.

## Offline behavior and tests

No weight files, packages or services are part of this repository. A local
service/checkpoint must be obtained and licensed separately by the operator; its
license is independent of this project's MIT/Apache-2.0 source license. The
adapter does not download or start it.

The adapter is usable offline: `DeciderPolicy` constructs with a loopback config,
and every failure path falls back to the heuristic. Tests and the example use
labelled loopback **protocol fixtures** (fixed typed-readout documents), not
model weights:

```
cargo test -p aperture-ai --test decider_schema --test decider_boundary
cargo run -p aperture-ai --example model_decide
```

The example proves one valid fixture answer is accepted by a real engine, a
truncated fixture answer falls back to a legal heuristic action accepted by the
engine, and stale generation/revision envelopes are refused with the engine
unchanged. Its fixture thread is stopped and joined before exit.

## Limitations

- The optional service endpoint was not available for this change, so validation
  against a real Decider checkpoint is **unverified**. All answers here come
  from protocol fixtures; no strength, calibration or real-inference claim is
  made.
- The masked view is the only observation. No hidden-state shortcut exists, and
  the model never determines rules or legal moves.
- This is a practice-slice adapter, not a claim of stronger play.
