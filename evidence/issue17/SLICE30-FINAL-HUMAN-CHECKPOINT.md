# Issue17 — slice 30: Drover Grizzly and the pool's first saddle (original eight-list contract still unfinished)

## Current candidate

- Same native card `t_67778feb`, checkout `C:/AI/aperture-worktrees/issue-17`, branch `aperture/issue-17`, draft PR46.
- **Code head of record `dea6fab`** (repair after native delta inspection); **tip `073fd19eca457868bbd0d6be28026fb3b4071086`** — local follow-ups `44b3351` (native verification receipts, evidence-only driver cleanup) and `4073c72`/`073fd19` (docs-only: correct the false "departure pruning narrowed" claim and the stale 133/326 split → 134/325).
- Local == remote at `073fd19`; PR OPEN / draft. No merge, master push, issue closure, downstream activation, new team or global config change.

## What this increment adds

`Drover Grizzly` (`{2}{G}`, 4/2 Creature — Bear Mount; Scryfall **OTJ 161**, immutable `560062cd-34f8-4d30-9e25-099b03961724`, retrieved **2026-10-27T14:40:04Z**; raw response, retrieval stamp and fixture `fixtures/starter-corpus/slice30-sourced.json` retained). Printed text: "Whenever this creature attacks while saddled, creatures you control gain trample until end of turn." plus "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"

Reusable surface rather than a one-card patch (CR 702.171a–c, pinned text `evidence/issue17/slice2-rules-source.txt` lines 5280–5286):

- `Action::SaddleMount { source, creatures }` — a typed activation whose cost is *other untapped creatures you control*; summoning-sick helpers are legal (saddle is not a `{T}` ability), the Mount itself is excluded, controllers must match, duplicates are refused, and the summed **effective** power must reach the printed threshold. Helpers tap as the cost.
- Sorcery timing only; the ordinary activation path refuses a saddle ability (`ActivateAbility`) so the cost can never be paid with no helpers.
- The ability resolves on the normal stack and applies a `Saddled` designation effect (`ContinuousKind::Saddled`, until end of turn) that cleanup ends and that battlefield departure prunes.
- `TriggerCondition::SourceIsSaddled` types the printed intervening condition, so an unsaddled attack produces no trigger at all.
- Resolution grants trample to each creature the controller controls **at resolution**, through the existing layer-6 `GrantsTrample` effects, and the grant uses the captured card identity so a source that has left cannot panic or change the recipient set.
- New rules sites use `Game::is_creature` (not the printed baseline), so an animated Stuffed Bear can saddle and receive trample; a plain artifact cannot.
- The saddle stack item carries `affected: Some(source)`, so a source that leaves while the ability is on the stack resolves with no effect and no transfer to a later same-ID incarnation.

## Native verification actually run

- Native inspection of the first cut found two real defects and returned them to the same writer: printed-only creature queries in the new saddle/grant rules (animated Bear wrongly excluded), and an unbound saddle stack item (source departure could mark a later incarnation). Both repaired in `dea6fab`; the repair tests were written after the code change and are disclosed as GREEN-strengthening, not strict TDD. A genuine RED exists for the unsupported card before registration.
- `evidence/issue17/slice30/run-native.sh` (committed) prints every exit code: **`FMT_EXIT=0`, `FOCUSED_EXIT=0`, `CLIPPY_EXIT=0`, `RELEASE_EXIT=0`, `JOURNEY_EXIT=0`, `REPLAY_EXIT=0`, `AUDIT_EXIT=0`**.
- Focused: **61 tests passed / 0 failed** across `issue17_slice30` (6), `issue17_slice29`, `issue17_slice28`, `issue17_slice24`, `timing_mana`, `replay_digest`, `card_printings`, `catalog_pool`.
- Real four-seat / 40-life package journey from `evidence/issue17/slice30-journey-driver` on `decks/package-slice30.txt` with exact snapshots (Jasmine Boreal injected for all four seats by the demonstrator): terminal **`Win{P1}`**, seed 2323, **2653 actions / 55 turns / 6968 replay events**, digest **`b711d966f9bd2e60187d4eafc5be852eea01b5f141fcc253813b1c38c8a85677`**, 9 Drover casts, 116 saddle activations from *offered* actions, 22 trample-grant events; separate `aperture-cli replay --file evidence/issue17/slice30/demo-replay.json` → **matches recorded, 6968 events**.
- Fresh save (`demo-save.json`) round-trips; the driver asserts exact deck snapshots, no `Truncated` events, replay/digest equality and genuine saddle/trample events.
- Fresh corpus audit (`slice30/corpus-audit.py`, recomputed from `catalog.rs` + the eight deck files with manifest set-equality): **146 catalog / 459 union = 134 supported + 325 missing**, per list `55/0/51/48/19/52/52/53`, Ayula **19**, **1 of 8 certified**, `AUDIT OK`.
- Independent review: **APPROVED_INCREMENT** at `44b3351` in the same original reviewer history (`evidence/issue17/slice30/REVIEW30-ORIGINAL-HISTORY.md`), with its own 6/6 slice-30 test run, release replay check and fresh audit. The two documentation corrections it named are the `4073c72`/`073fd19` commits; the reviewed behavior is unchanged by them.
- Exact-head CI is captured by `slice30/capture-ci.py` (exits non-zero unless every check is green *and* keyed to the given head); receipt `slice30/exact-head-ci-summary-073fd19.json`.

## Known limits, disclosed

- **Bounded offer enumeration.** Saddle offers are generated for at most 10 eligible helpers (all nonempty subsets meeting the threshold); above that cap no saddle action is offered even though direct `SaddleMount` validation accepts any legal set. This is a bounded-practice limitation, not whole-card conformance and not an exhaustive choice surface.
- Layers remain 4/5/6/7b/7c only; 7a (characteristic-defining P/T) and 7d are still unimplemented, so `Beorn's Hospitality`, `Lumra` and similar cards stay unsupported.
- Saddle is implemented for the one printed saddle cost (Saddle 1); no other saddle card is in the pool, so other thresholds are exercised only through the typed validator shape.
- The journey is a **40-card package, not a 100-card starter certificate**; it is an evidence-only masked demonstrator, not product AI and not strength evidence.
- `crates/ai` still reads the printed creature baseline for its belief model (not edited by this lane).
- Three tracked slice-23 evidence files still carry pre-existing uncommitted appends; disclosed, not folded into this head.
- Untracked by design: this checkpoint, the CI capture script and receipts, and worker/reviewer logs, so the branch tip keeps its own CI.

## Contract status (original #17 — unchanged, unfinished)

**1 of 8** desired 100-card starters certified (Jasmine only). 146-card catalog; 459-name union = 134 supported + 325 missing; per list `55/0/51/48/19/52/52/53`; Ayula 19 short. Desired snapshots unchanged, no bundled replacements; arbitrary Archidekt compatibility remains an explicit non-goal. This slice certifies **no** list.

## Next action

Continue the same card / PR46 / branch / saved histories with the next sourced Ayula names from `evidence/issue17/slice27/AYULA-NEXT-PACKAGE.md`: `Kamahl's Summons` (ONS 269) needs a per-player hand-subset reveal choice, `The Earth King` (TLA 172) is half-blocked on library search, and `Beorn the Fierce` (HOB 119) is closer now that additive type change exists. Do not complete #17, merge, push master, close the issue, or unlock `t_26a7a4ff` / `t_3c2fe51a`.
