# Issue17 — slice17 human checkpoint

## Increment, not whole-issue acceptance

Same branch `aperture/issue-17`, draft PR46. Three sourced Ayula-list cards:
- **Friendly Teddy** `{2}` 2/2 artifact creature Bear Toy: dies → each player draws. Active-player turn order, source-left trigger, explicit serialized continuation when a commander draw suspends; mid-suspension save/load works.
- **Ulvenwald Bear** `{2}{G}` 2/2: morbid enters trigger if any creature died this turn, then two +1/+1 counters on target creature (either controller). Token deaths count.
- **Caller of the Claw** `{2}{G}` 2/2 flash Elf: enters → one 2/2 green Bear per nontoken creature put into its controller's graveyard from battlefield this turn. Counts owner, not battlefield controller; mill/discard don't count; later graveyard departure doesn't erase a death.

Source URLs/retrieval dates: `fixtures/starter-corpus/slice17-sourced.json`.
Fresh native audit: **134 catalog;459union=122supported+337missing;1certified/7planned**. Ayula31 missing. Desired100-card snapshots unchanged; all eight libraries99cards plus named commander. `slice17/native-corpus-state.json`.

## Review and genuine corrections

TRUE original-history practical adjudication **APPROVED_INCREMENT** at `3934d765158733a8351a594ed27564cb14431cb8` (`slice17/REVIEW17-ORIGINAL-HISTORY.md`), no blocking product defect. Parent caught a launch-path error: first review at `bfaa782` used root `review-sessions/` and created a new history, not the saved original. Its real independent evidence remains in `REVIEW17.md`, but its same-history claim is false. Both histories are preserved; the true original was subsequently resumed at the exact `evidence/issue17/review-sessions/…` path and independently ran slice17 6 / command-zone34 / replay6 plus separate replay/save checks. `e383d9d` only strengthened tests and `3934d76` only recorded evidence; no product source changed after `bfaa782`.

Two first-cut defects repaired: cleanup is still same turn, so history resets only at actual turn advance; initial Teddy draw loop lost later seats after commander replacement, now explicit serialized progress preserves them. Counter events restored. `BearToken` added to `CardId::is_token`. Compatibility-only AI changes initialize/copy two public state fields (`crates/ai/src/search.rs`, `crates/ai/tests/common.rs`), no policy/search change.

TDD was scaffold-first, not strict test-first; initial morbid assertions were permissive. Preserve disclosed deviations (`TDD-slice17.md`, `slice17/NATIVE-TEST-GAP.md`), don't claim retroactive compliance. Final tests require actual zero/exact-two counters and a genuinely offered opposing target.

## Executed verification

- Parent independently ran final slice17 **6**, card_printings **8**, catalog_pool **5**, slice15 corpus **2**, all pass (`slice17/native-focused.log`); fmt check passes.
- Reviewer ran lib12 / command-zone34 / then-slice17 5 / replay_digest6 / ordering_persistence2, all pass; includes actual-turn reset and commander-draw save/load.
- Writer final workspace warnings-denied clippy real exit0; releaseCLI built. Full historical workspace not repeated after repair; exact final-headCI is collected after final publication, never inherited.
- One real evidence-only masked offered-action journey, NOT productAI/strength: exact `decks/package-slice17.txt`, **4seats/40life/40-card package**, seed1616, **terminalP3/1866actions/42turns/4952events**, digest `85fabffc9aa16a0f9ba53a5f64a751c43bb9ae66a3ce96f3e7402f1c633c7027`. Driver explicitly injects **Jasmine Boreal for all four seats**. Casts:Teddy12/Ulvenwald8/Caller8; Caller creates4Bears. Parent separately replayed replay and fresh session-save with matching digest/save reproduction (`slice17/native-save-replay.log`). NOT a desired100-card starter certificate.

## Original gate remains open / resume

Missing names: Isamaru55/Talrand51/Krenko48/Ayula31/Elas52/Tatyova52/Zada53. Ayula commander itself still needs modal fight/counter trigger. Equipment and other per-list exclusions remain. No arbitrary Archidekt/fullCommander/v1 claim. No merge/masterpush/closure/certified-downstream unlock.

Continue SAME card/PR46/branch and exact saved histories:
- writer:`evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl`
- reviewer:`evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl`

Next: sourced remaining Ayula-card/commander implementation where feasible, then genuine desired-list certification; no re-audit approved increments. Current-lane target only; preserve other-agent cache cleanup, no retired builds/global config edits. Existing enabled15-minute supervisor owns same-card renewal. Final publicationSHA/CI conclusions are in fresh handoff receipt. No implementation command left running at checkpoint.

