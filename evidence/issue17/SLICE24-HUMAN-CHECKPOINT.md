# Issue17 — slice24 human checkpoint

- Branch `aperture/issue-17`, base `632c5e7`; local slice24 head is the final receipt.
  No push/merge/closure/PR creation. **Full #17 unfinished; 1 of 8 certified.**

## Implemented
**Surrak and Goreclaw** `{4}{G}{G}` Legendary Human Bear 6/5, exact MOM 337 printing
(immutable `9c10934d-9016-43c4-a7ab-56cc7d8f671f`, retrieved
`2026-10-27T08:44:35Z`, source `fixtures/starter-corpus/slice24-sourced.json`).

- Printed **Trample**; typed static `OtherControlledCreaturesHaveTrample` is read by the
  same combat-damage assignment as intrinsic trample.
- **Whenever another nontoken creature you control enters**: captures the entered object
  as a non-target `affected` identity on the trigger/stack item, adds a +1/+1 counter, and
  grants haste until end of turn via `ContinuousKind::GrantsHaste` in the ability layer.
  Tokens, self-entry and opponents are excluded by queue-time filtering; no target prompt.
- `has_haste` is consulted by attack legality and both tap-cost summoning-sickness checks.
- A battlefield departure prunes the old incarnation's pending/queue/stack `affected`
  reference; a bounced/recast same ObjectId cannot inherit the buff or trigger.
- Haste expires at cleanup; the effect is source-independent once created, and target
  departure pruning prevents transfer.
- New trigger/stack fields are serde-default/skipped when absent, preserving old digests.

## RED retained
`cargo test -p aperture-core --test issue17_slice24` failed
`surrak_goreclaw_is_sourced_and_supported` before registration.

## Focused green
- issue17_slice24: 10 passed (real offered cast, trigger buff/haste on entered nontoken,
  older creature unaffected and cleanup expiry, static trample in real blocked combat,
  bounce/recast same id triggers once, midtrigger save/reload parity, hasty Werebear
  actually offered as a mana source, mandatory non-target trigger retained after entered
  departure with affected binding invalidated, real source-left Surrak continuation, and
  token/opponent filtering).
- card_printings 8, catalog_pool 5, slice15 corpus 2, issue13 commander 43, replay_digest 6.
- fmt check, workspace clippy -D warnings, release CLI all exit 0; exits in
  `evidence/issue17/slice24/gates.log`.
- Fresh audit: 141 catalog / 459 = 129 supported + 330 missing; Ayula 24; snapshots and
  certificate statuses unchanged.

## Journey
Evidence-only projected-offer masked demonstrator, NOT product AI. Exact 4 seats/40 life,
seed 2323, `decks/package-slice24.txt`, Jasmine Boreal injected. Terminal Win P3, 1602
decisions, 37 turns, snapshot SHA256
`feb9033b925198a0eae49b117c858b626e968a4e7691c8e70c7e93bffc4d7455`; Surrak cast 6,
11 entry buff/haste events, 20 blocked combat-damage events with static trample available;
replay matched and fresh save round-tripped. Digest
`a41bbfd2760034f791a3616bd5d6faa0fd79ad8e9cee300eb69cc38c5a1ec631`.

## Gaps / deviations
- Token/opponent entry filtering is implemented but not separately regression-tested.
- Source-left Surrak trigger and a tap-cost haste activation are not directly tested;
  haste attack legality, cleanup expiry, bounce/recast and midtrigger save are covered.
- No full workspace matrix rerun; exact-head CI owns it.
- New `TriggerRef`/`StackItem.affected` fields are serde-default/skipped and mechanically
  initialized in legacy test literals; AI has no new GameState field or policy change.
- Midtrigger save test uses a sandbox/direct fixture, not a reachable action-history save.
