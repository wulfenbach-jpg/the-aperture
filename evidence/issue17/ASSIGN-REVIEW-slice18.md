# Slice18 delta review — SAME original reviewer history, bounded 14 min

You are the original independent reviewer for issue17 on branch `aperture/issue-17`, draft PR46. Review ONLY this delta; approved slices 1–17 need no re-audit. Base for the delta is the approved slice17 head `76f1b31045ee93e3ef4e6cc4da61198c1c44fa53`; candidate head `2785c2cefbb92b678ace58949d2373f982a4a208` (local = remote = PR46 branch; the GitHub PR API may lag by a minute).

## What the delta claims

One sourced card: **Ayula, Queen Among Bears** `{1}{G}` Legendary Creature — Bear 2/2, printed:
"Whenever another Bear you control enters, choose one — • Put two +1/+1 counters on target Bear. • Target Bear you control fights target creature you don't control."
Source `https://api.scryfall.com/cards/c046dfb0-9ca3-4219-a6b0-d7503bc2fbd0` (mh1 155), retrieved `2026-10-26T23:40:57Z`, recorded in `fixtures/starter-corpus/slice18-sourced.json`. Registered as a supported commander.

Plus repairs found by native inspection during the slice (all in the delta):
- a BLOCKING regression the first cut introduced in `apply_trigger_choice` (unconditional `pending.take()` consumed and dropped ordinary optional `TriggerChoice`), repaired;
- Ayula queued-batch loss on no-legal-mode paths, repaired;
- Fight citation corrected from CR 701.12a to 701.14a / 701.14b (pinned text `evidence/issue17/slice2-rules-source.txt`);
- archived-reference traversal extended to `TriggerMode.queued` and `TriggerTarget.first`.

## What to adjudicate (practical, not exhaustive)

1. **Re-read the printed text and the implementation** — is the trigger limited to another Bear *its controller* controls (tokens count, self and opponents' entries don't), is the mode chosen before targets, is counters-mode allowed to target an opponent's Bear, is fight mode offered only with both a Bear you control and a creature you don't control, and are both fight directions applied through the existing damage path with `701.14b` fizzle when either target is gone? Flag any place the code invents semantics the text does not support.
2. **Verify the regression repair by execution**: `cargo test -p aperture-core --test abilities` must pass 13/13, and the retained RED claim is that the same suite was 12 passed / 1 failed at `9c3a594` with "declining is a typed event". Confirm the accept path really applies its effect, not just that the stack empties.
3. **Spot-check the new tests are not vacuous** — exact counters, reached offered-target decisions, the fight-prevention case showing one direction reduced and the other still lethal, pending save/reload equality, and forged submissions refused with pending/decision/digest unchanged.
4. **Reconcile the corpus bookkeeping.** The handoff and docs state 135 catalog / 123 supported / 336 missing (union 459). An independent parse during this slice agreed on 336 missing and on 135 pool cards and confirmed no `missing_cards` name is in the catalog, but computed a union of 454 names including the eight commanders (⇒ 118 supported). Decide which number is correct by your own audit and say plainly which figure the records must carry; a record correction is acceptable as your review outcome if the code is fine.
5. **Judge the journey honestly** — `evidence/issue17/slice18/demo-journey.json` + `demo-replay.json`: 4 seats / 40 life / exact 40-card package / seed 1616, terminal `Win{P2}`, 1961 actions, 5028 events, digest `09e6bb25c505…`, and the driver injects **Ayula for all four seats** (evidence-only masked demonstrator, not product AI). Confirm it is labelled as not a 100-card certificate and that `sha256(decks/package-slice18.txt)` matches its `snapshot_sha256`.
6. Check scope discipline: no import/UI/AI-policy edits beyond mechanical `match` arms for the new enum variant, no desired 100-card snapshot replacement, no weakened assertions (diff-test any deleted line).

Do not re-run the historical matrix, do not rebuild retired worktrees, and use only the current lane target (`CARGO_TARGET_DIR` under this checkout). No merge, no master push, no issue closure, no upstream unlock — you are reviewing an increment, not accepting original #17 (still 1 of 8 desired starters certified).

## Output

Write `evidence/issue17/slice18/REVIEW18-ORIGINAL-HISTORY.md` with:
- verdict line: `APPROVED_INCREMENT` or `CHANGES_REQUESTED`, and the exact reviewed head;
- what you executed (commands + real counts), what you inspected, and any blocking finding with a reproduction;
- the bookkeeping decision from item 4;
- explicit statement that full #17 is unfinished and that no merge/closure/unlock was performed.

Commit the report (evidence-only commit is fine) and stop. Do not push, merge or close anything.
