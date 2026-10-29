# Issue17 — slice25 human checkpoint

- Branch `aperture/issue-17`, base `3b06249`; local slice25 head is final receipt. No
  push/merge/closure/PR creation. **Full #17 unfinished; 1 of 8 certified.**

## Implemented
**Exuberant Wolfbear** `{3}{G}` 4/4 Wolf Bear, exact IKO 151 source
(`fixtures/starter-corpus/slice25-sourced.json`, immutable 8c8ff0da, retrieved
2026-10-27T09:19:21Z). Attacks trigger is optional and targets a Human you control.
Setting its base P/T is `Layer::Setting` (7b, CR 613.4b), applied before counters and 7c
modifiers. Source effective P/T is captured at resolution; if the Wolfbear leaves before
resolution, a first-departure `source_pt_lki` snapshot on queued/stack/pending ability
references supplies the actual last-known P/T. Target/source departure and cleanup use the
normal continuous-effect pruning/expiry paths; forged non-Human/opponent targets are refused
by `TargetSpec::HumanYouControl`.

## RED retained
`issue17_slice25` failed `exuberant_wolfbear_is_sourced_and_supported` before registration.

## Focused green
- issue17_slice25: 4 passed (offered cast/real attack trigger; optional accept sets base from
  current 5/5 source and target's counter adds after; decline leaves Human 2/1; source-left
  Unsummon uses LKI 5/5).
- card_printings 8, catalog_pool 5, slice15 corpus 2, commander 43, replay_digest 6.
- fmt/clippy/release exit 0, persisted `evidence/issue17/slice25/gates.log`.
- Fresh audit: 142 catalog / 459 = 130 supported + 329 missing; Ayula 23; snapshots and
  certificate statuses unchanged.

## Journey
Evidence-only masked projected-offer demonstrator, NOT product AI: exact 4 seats/40 life,
seed 2323, `decks/package-slice25.txt` (16 Forest/4 Plains), Jasmine Boreal injected.
Terminal Win P3, 1760 decisions, 41 turns; Wolfbear cast 8, attack trigger staged 9,
base-P/T set events 9; replay matched and fresh save round-tripped. Digest
`4d6ea58ee735c74a89e142eca90656dbf22faeaf49c2de0fb33c0ec9952c5d52`, snapshot SHA256
`1bd229439656be174278cd958eb7083ec495c43be3eefa8a17d4ee6fff881a0d`.

## Gaps / deviations
- No dedicated target-departure, cleanup-expiry, or source-recast-overrides-LKI test; no
  save/reload mid-optional-ability test. LKI/source-left and current-source growth are tested.
- No tap/untap interaction or browser/UI claim.
- No full workspace matrix rerun; exact-head CI owns it. New trigger/stack LKI fields are
  serde-default/skipped; legacy test literals mechanically initialized. No AI policy change.
