# Native prepublication findings while slice18 writer active

1. BLOCKING inherited optional-trigger regression in `apply_trigger_choice`: new `if let Some(Pending::TriggerMode{..}) = self.state.pending.take()` consumes every nonmodal TriggerChoice when pattern does not match. Following second pending.take seesNone and silently drops entire existing optional resolution. Fix by match on single take (both variants), or inspect/as_ref before conditional take. Genuine regression test with existing optional card (e.g MotherBear? discover actual optional=true card) and accept/decline paths; require accepted optional effect actually happens. Test should fail pre-repair. Do not merely assert pending cleared.
2. `stage_ayula_trigger` false/false arm emits removed but DOES NOT `stage_triggers(queued,events)`, losing other pending triggers. Fix continue drain. Need reached no-target case e.g both Bears removed before staging as practical, no weakening original queue behavior.
3. State `references_removed_object` TriggerMode match checks trigger only, drops `queued`; TriggerTarget branch does not check new `first` anchor. Preserve archival strict-ref safety for both. New pending fields require session validation and source-left serialization actual roundtrip, not only final journey save.
4. Rule cite: pinned `slice2-rules-source.txt` Fight701.14a-d, modal603.3c/700.2b; current code701.12 incorrect.
5. `stage_ayula_target` len0 arm also doesn't continue queued. Only legal modes offered, but strict validation of forged pending must refuse inconsistent targets/modes before run.
6. Initial code edits preceded new tests; disclose scaffold-first/non-strictTDD; do not invent pre-changeRED. Retain actual assertion RED for concrete repairs.

Native has NOT changed product files, and no competing writer launched. These are concrete diff findings against intermediate working tree, may already be repaired by writer before handoff. Parent must check final code before launching repair.
