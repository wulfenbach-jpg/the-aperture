# Slice18 assignment 3 — SAME original writer, bounded 12min, two coverage gaps only

Your repair `79d976d` fixed the blocking regression, the queued-trigger loss, the citations and the archived-reference traversal. Native re-read confirmed the single `match self.state.pending.take()` dispatch and 701.14a/701.14b sites.

Two items from assignment 2 are still missing and are the only work now. Add tests only (plus minimal test-support changes if truly required):

1. **Pending-choice persistence.** Ayula's new pendings carry serialized state (`Pending::TriggerMode { trigger, queued }`, and `Pending::TriggerTarget { first: Option<Target> }`). Prove, through the real save/session path used by the CLI or session save loader (not a hand-built struct):
   - save while `Pending::TriggerMode` is waiting, reload, answer the mode, finish the ability, and require the same final state and replay digest as the uninterrupted run;
   - save while a two-target fight has `first: Some(..)` and the second target unanswered, reload, answer, resolve, and require the same final state/digest;
   - a reloaded save must still refuse an action by the wrong seat on that pending.
   If the existing save version/validator cannot represent these pendings, that is a real finding: report it explicitly rather than skipping the test, and do NOT bump a persistence version without saying so in the handoff.

2. **Forged/invalid pending submissions refused with state untouched.** With `Pending::TriggerMode` waiting, a `ChooseTriggerTarget` submission must be refused and the pending/state unchanged; with `Pending::TriggerTarget` waiting, an `AnswerTriggerChoice` must be refused; and `ChooseTriggerTarget` naming an object that is not in the offered legal set must be refused without mutating. Assert the engine error and that digest/decision serial/pending are unchanged.

Verify: the affected test files plus `command_zone_setup`, `abilities`, `rules_conformance`, `issue17_slice17`; fmt; then commit. Update `evidence/issue17/slice18/HANDOFF.md` and `IMPLEMENTATION-RESULT-slice18.json` with the new tests and the exact results. No new cards, no product-surface redesign, no push, no merge, no closure, current lane target only.
