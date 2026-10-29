# Slice 17 independent delta review

> Parent continuity correction: this review accidentally ran in a NEW root `review-sessions/` history, not the saved original. Its executed results remain evidence; the same-history claim below is incorrect. The true original history subsequently adjudicated exact3934d76 in `REVIEW17-ORIGINAL-HISTORY.md`. Preserve both records.

- Reviewer: sole reviewer, same history; read-only for product source.
- Base: `6f294cfc7c82fa00ea3344c738f6ea4f96e97dc8`
- Exact HEAD inspected: `bfaa7827b5ffe40f9de8926efeed718dac5f6715`
- Delta: `7e4a013` (implementation/tests/fixtures/docs) + `bfaa782` (result/checkpoint).
- Branch/PR: `aperture/issue-17` / draft PR 46. No push, merge, reviewer launch or closure.
- Product source in the worktree is unmodified by this review; only unrelated evidence/session files are dirty/untracked.

## Verdict: APPROVED_INCREMENT

No product-source defect found in the repaired delta. This is an increment only:
original #17 is still unfinished, 1 of 8 starters is certified, and the eight-list
gate remains open. Not whole-issue acceptance.

## Actually executed

| check | result |
| --- | --- |
| `cargo test -q -p aperture-core --lib --test issue17_slice17 --test command_zone_setup --test replay_digest --test ordering_persistence` | 12 lib, 34 command-zone, 5 slice17, 6 replay-digest, 2 ordering tests passed |
| `aperture-cli.exe replay --file evidence/issue17/slice17/demo-replay.json` | 4 seats, 1866 actions, terminal win P3, replayed digest `85fabffc9aa1` matches recorded, 4952 events |
| `aperture-cli.exe replay --file evidence/issue17/slice17/demo-save.json` | stored/recomputed digest `85fabffc9aa1` match; replay of 1866 actions reproduces the save |

The release-CLI replay is the independent practical probe. It is the 40-card,
four-seat, 40-life package journey from seed 1616, not a desired 100-card starter.

## Correctness findings

1. **Friendly Teddy / all-player draw suspension.** Teddy is artifact+creature;
   its `Dies` trigger is queued after the source card has moved to the graveyard.
   `Effect::EachPlayerDraws` starts at `turn.active` and continues by live-seat
   order, so APNAP is used. `DrawResume::EachPlayer` serializes source, cursor,
   next seat, cards left, per-seat count, completed live seats and fixed live
   total. `command_zone_setup::each_player_draw_continues_every_live_seat_after_a_commander_replacement`
   is a real regression: commander is on P0's library top, Shock kills Teddy, the
   death trigger resolves into a `CommanderDraw` with `EachPlayer { next: P0 }`,
   priority is withheld, a mid-suspension `SessionSave` validates/reloads, and
   after `AnswerCommanderDraw` P1's hand grows by exactly one. This resolves the
   NATIVE-INSPECTION concern with behavior, not loop reading.
   *Limitation:* the focused suite does not separately execute a mass-destruction
   batch containing Teddy; the shared `move_to_graveyard_inner` batch path and the
   Teddy trigger path are each exercised, but their composition is inspected, not
   run. The journey kills Teddy by Shock and destroys 10 copies overall, but it
   does not assert APNAP draw order for a simultaneous sweep.

2. **Ulvenwald Bear / morbid.** Any dying creature sets
   `creature_died_this_turn`, including tokens; the flag is written before the
   token exclusion from the nontoken-owner count. The intervening-if is checked
   when the ETB trigger is queued. Reset happens only when `turn.number` advances,
   not in `finish_cleanup`; the lib regression calls real `finish_cleanup` twice
   and retains an earlier death, and the integration case proves the next actual
   turn starts empty. Counters go through the same authoritative counter state
   event shape (`... has N +1/+1 counter(s)`) as the sibling counter effects.
   *Limitation:* `ulvenwald_morbid_needs_a_real_death_and_targets_any_controller`
   still ends its positive branch with `target.is_none() || counters == 2`, and the
   no-death branch asserts trigger-queue emptiness rather than the entered Bear's
   zero counters. There is no token-only-death morbidity case. Parent's
   `NATIVE-TEST-GAP.md` already records this. Code inspection shows correct
   behavior; the assertion strength, not the source behavior, is the gap.

3. **Caller of the Claw.** Counts increment at the actual battlefield→graveyard
   move, indexed by `object.owner`, not controller. Token deaths are excluded
   because `CardId::is_token` now includes `BearToken` (real repair); milling uses
   a separate path and does not count; later zone changes do not decrement the
   count. Simultaneous SBA and artifact/enchantment destruction call
   `move_to_graveyard_inner` once per object, so each death increments once.
   The caller test covers owner≠controller, a token death, a milled creature, and
   one dead card later exiled from the graveyard, yielding exactly two Bears.
   Flash is the existing shared `check_timing`/`has_flash` path; the Caller flag is
   verified and the shared Ashcoat timing test covers instant-window casting.
   *Limitation:* no Caller-specific cast outside its controller's main phase and no
   dedicated simultaneous-death count assertion. Neither exposes a source defect.

4. **Compatibility / save validation.** `creature_died_this_turn` and
   `nontoken_creature_deaths_this_turn` are serde-default and skipped when
   false/empty; tracking is opt-in only while an Ulvenwald Bear or Caller of the
   Claw object exists. The unchanged pinned `replay_digest` (6) and
   `ordering_persistence` (2) suites pass, so unaffected closed-corpus digests are
   retained. The validator now allows a triggered stack source to remain in
   graveyard/exile/command/hand with at most one stack entry. That is broad for
   hand/command, but I found no new dangling reference, shape, or panic path:
   object owners/controllers are range-checked, the EachPlayer resume is validated
   at suspension (`next == drawing owner`, cursor/source/resolving/stack match), and
   malformed owner counts cannot panic.

5. **Catalog/fixtures/counts.** The sourced fixture has immutable Scryfall printing
   URLs and retrieval dates; card-printing conformance passes. Manifest removed
   only the three now-supported names from Ayula's missing list; counts 134
   catalog / 122 supported / 337 missing and Ayula 31 missing are consistent with
   the diff. Ayula and the eight-list gate remain explicitly open.

## Disclosed deviations

- TDD was scaffold-first (documented in `TDD-slice17.md`); the behavior RED is real
  but the initial fixture also had a Mountain/payment setup bug.
- The full workspace suite was not rerun after the repair; repair verification was
  focused suites, workspace clippy and release CLI. Parent owns exact-head CI and
  the focused required gates. I did not rerun them.
- No product edits, no test changes, and no review of unrelated old matrices.

## Next

No source change requested from this review. If test strength is to be raised
before publication, do it under the existing `NATIVE-TEST-GAP.md` brief; it does
not change this increment's source verdict.
