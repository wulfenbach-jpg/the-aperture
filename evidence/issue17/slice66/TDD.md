# Slice 66 — TDD record (implementer, not reviewer)

The record below is what actually happened, in order, including the mistakes.

## R365-1 (the functional defect from review 365)

1. Two regressions were written first, in `crates/core/tests/issue17_slice66.rs`:
   one asserting the trigger is staged ABOVE the ability and resolves before the
   damage (Goblin Sharpshooter's untap as the observable dies trigger) and one
   asserting a target-choosing dies trigger (Festering Goblin) asks for its
   target while the ability still waits below.
2. RED, before any engine change: **0 passed / 2 failed, exit 101**; the stack
   held only `Activated { ability: 1 }` and no `TriggerStaged` event existed.
3. Fix: `put_triggers(events)` after the ability's `StackItem` push in
   `apply_activate_ability`.
4. GREEN: first test 1/2. The second still failed for a test-side reason — the
   driver picked the *first* offered trigger target (the Commander) instead of
   the Bears it then asserted on. Fixed the test to select the Bears explicitly
   and to assert the -1/-1 actually landed; 2/2.
5. Focused suites (slice 62/63/64/65, commander combat, priority/stack) re-run
   unchanged and green: the staging point does not disturb existing paths.

## Cards

6. Sourcing before authoring: `evidence/issue17/slice66/fetch-sources.py` fetched
   both cards and enumerated every printing (5 and 6) with one distinct oracle
   text each; the catalog strings were then written from that file.
7. `crates/core/tests/issue17_slice66_cards.rs` written against the printed
   behaviour: dies watcher (other Goblin / own death / Bear / opponent's Goblin),
   one offer per legal Goblin with a forged non-Goblin and foreign Goblin
   refused without mutation, the two-token activation with the whole `{3}{R}`
   paid, the "other Goblins" anthem, and artifact destruction with an enchantment
   refused.
8. RED: 1 passed / 4 failed. Three failures were test defects (a token that
   ceases to exist has no `State.Object`; layer 7c statics are recomputed on a
   priority pass, not at fixture build; `try_object` returns graveyard objects
   too, so death must be asserted by zone). One was a **product** defect: the
   artifact was never destroyed because `target_is_legal` assumed the creature
   default for every permanent target, so the ability fizzled with "all of its
   targets are illegal". Fixed by reading the ability's printed target spec.
9. GREEN: 5/5, then the journey test (1/1) and the printing-conformance test
   (25/25 in `card_printings`).

## Journeys

10. The in-engine journey was written from offered actions only. First run
    failed because it asserted the ping's damage at the moment the trigger was
    *staged* (the damage has not resolved until the target is chosen) — the
    assertion was moved after the answer and paired with an event-order check.
11. Second run failed on declaring attackers: the creatures cast this turn are
    summoning sick, so only the two pre-existing Bears may attack. The attack
    list was corrected, and the journey was extended to answer the *second*
    ping (the Trashmaster's sacrifice kills another Goblin while Pashalik is on
    the battlefield) explicitly instead of leaving it to the auto-answer.
12. The separate-process CLI game was then run on a new 40-card package and
    replayed by the shipped binary.
