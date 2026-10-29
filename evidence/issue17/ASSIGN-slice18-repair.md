# Slice18 assignment 2 — SAME original writer, bounded 26min, REPAIR before review

Native prepublication inspection of your committed head `9c3a594` found one BLOCKING regression and three concrete defects. Do NOT add new cards or new features this pass. Repair only these, with tests that actually fail first, then focused GREEN, then a fresh journey. At the end amend or add ONE focused commit; parent owns publication.

## 1. BLOCKING — your new `apply_trigger_choice` silently consumes ordinary optional triggers
`crates/core/src/engine.rs::apply_trigger_choice` does:
```rust
if let Some(Pending::TriggerMode { .. }) = self.state.pending.take() { ... }
let Some(Pending::TriggerChoice { .. }) = self.state.pending.take() else { return; };
```
`Option::take()` runs unconditionally, so when the pending is `TriggerChoice` the first `if let` takes it, fails to match, drops the value, and the second `take()` sees `None` and returns. Accepting a "you may" ability no longer sets `trigger_answered`/`resolving`, and declining no longer emits `TriggerDeclined` and no longer pops the ability. This breaks approved pre-existing behavior.

Proved by running your head:
`cargo test -p aperture-core --test abilities` -> `optional_trigger_is_declined_or_used_by_its_controller` FAILED at `crates/core/tests/abilities.rs:391` ("declining is a typed event"), 12 passed 1 failed.

Repair it so exactly one `take()`/`match` dispatches on the actual pending variant and both paths keep their old behavior. Keep/strengthen the existing test — do not weaken or delete its assertions. Add an explicit accept-path assertion if the existing test only checks the decline path, and prove the accept path really applies its effect (not merely that the stack emptied).

## 2. Queued triggers are dropped when Ayula has no legal mode
In `stage_ayula_trigger` the `(false, false)` arm pushes `Event::TriggerRemoved` and returns without draining `queued`, and `stage_ayula_target`'s `0 => {}` arm likewise drops `queued`. `stage_triggers` also `return`s into `stage_ayula_trigger`. A second trigger queued in the same batch (e.g. Soul's Attendant on the same entrance) is then lost forever. Drain the remaining queue on every path, and add a test where a second trigger is queued alongside Ayula and must still be placed/resolved after Ayula's ability is removed.

## 3. Wrong pinned rule citation for fight
The pinned rules text in `evidence/issue17/slice2-rules-source.txt` puts Fight at **701.14a** (each deals damage equal to its power) with **701.14b** for "one or both are no longer on the battlefield / illegal target → neither fights" and **701.14d** for "not combat damage". Your code cites `701.12a` in `crates/core/src/card.rs`, in the fizzle event string, and in the offered-action rules — plus any handoff/docs text. Fix every site to 701.14a / 701.14b as appropriate. Re-check the modal citations (603.3c, 700.2b) are the ones actually used.

## 4. Archived-reference validation not extended to the new pendings
`GameState`'s removed-object reference check handles `TriggerMode` by validating only `trigger`, and `TriggerTarget` by validating trigger/legal/queued but not the new `first` anchor. A malicious archived state could therefore reference a departed object through `queued`/`first` and pass validation. Extend the check so every live object reference in the new pendings (`trigger`, `queued`, `legal`, `first`) is validated, with a negative regression that a forged archived reference is refused.

## 5. Required coverage you disclosed as missing (add now, focused only)
- Ayula pending-choice **save/load + replay** round-trip: serialize while `Pending::TriggerMode` is waiting AND while a two-target fight is half-answered (`first: Some`), reload through the real session/save path, finish the choice, and require the same final state/digest as the uninterrupted run.
- **Source-left** behavior: Ayula leaves the battlefield after the trigger is on the stack — the ability must still resolve with its chosen mode and legal targets, and must not panic or read stale state.
- **Prevention/replacement**: fight damage going through the existing prevention pipeline (use a supported prevention card) must be reduced while the other direction still applies, and both directions remain one simultaneous event pair.
- Forged pending refusal: submitting `ChooseTriggerTarget`/`AnswerTriggerChoice` when the pending is not the matching kind must be refused with the state untouched.

## Verification and handoff
- Tests first for each repair; keep a real RED transcript (the `abilities` failure above is already one — record that exact command/result) and add a RED for the queued-trigger loss and the archived-reference gap (those may need a scratch revert of the fix to show the failure; a compile error is not a RED).
- Then focused GREEN: `--test command_zone_setup`, `--test abilities`, `--test rules_conformance`, `--test issue17_slice17`, `--test card_printings`, `--test catalog_pool`, `--test issue17_slice15_corpus`, `--test issue13_commander_combat`. Re-run the journey once with the repaired binary and record terminal/actions/digest/replay/save.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --release -p aperture-cli` (current lane target only; no second cargo in parallel).
- Update `evidence/issue17/slice18/HANDOFF.md` + `IMPLEMENTATION-RESULT-slice18.json` with the repairs, the exact RED results, your commit, and the still-open gaps. Keep the disclosed scaffold-first TDD deviation honest; do not claim retroactive strict TDD.
- Ownership unchanged: core/corpus/decks/fixtures/docs/evidence. No import/UI/AI-policy edits, no `git add -A` of session histories, no push/merge/closure. Stop after committing so the parent can collect exact-head CI and one same-history practical review.
