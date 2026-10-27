# Slice30 result — Drover Grizzly / reusable saddle

- Branch `aperture/issue-17`, base `de3ba07`; local slice30 head is final receipt.
- **PARTIAL #17**, 1 of 8 certified. No push/merge/closure.

## Implemented
Drover Grizzly `{2}{G}` 4/2 Bear Mount, OTJ 161, immutable
`560062cd-34f8-4d30-9e25-099b03961724`, retrieved `2026-10-27T14:40:04Z`; raw + normalized
fixtures recorded. Typed reusable `SaddleMount` action:
- sorcery timing, source excluded, helpers must be other untapped creatures the activator
  controls; helpers may be summoning sick; effective power total must meet Saddle 1;
- offered actions enumerate bounded subsets of eligible helpers (≤10 helpers, all
  nonempty subsets meeting threshold) and validate each; helpers tap on cost payment;
- the saddle ability resolves on the normal stack and applies a serialized until-end-of-turn
  `Saddled` effect; cleanup and battlefield-departure pruning end it;
- attack trigger condition is typed `SourceIsSaddled`; it fires only when actually saddled;
- resolution grants trample until end of turn to each creature controlled then via the
  existing layer-6 grant, source-left safe through captured card identity.
- No card-name conditional interpreter; no new GameState field. New stack fields are
  serde-default/skipped.

## RED
`cargo test -p aperture-core --test issue17_slice30` failed
`drover_grizzly_is_sourced_and_supported` before registration.

## Verified
- issue17_slice30: 4 passed (offered cast/saddle/trample; unsaddled no trigger; self/tapped/
  opponent/insufficient-power refusals; save/reload saddled state).
- card_printings 8, catalog_pool 5, slice15 2, commander 43, replay_digest 6.
- fmt/clippy/release exit 0; `gates.log`.
- Fresh audit: 146 catalog / 459 = 134 supported + 325 missing; Ayula 19; snapshots and
  certificate statuses unchanged.

## Journey
Evidence-only masked projected-offer demonstrator, NOT product AI: exact 4 seats/40 life,
seed 2323, `decks/package-slice30.txt`, Jasmine Boreal injected. Terminal Win P1, 2653
decisions, 55 turns; Drover cast 9, saddle activated 116 times via offered actions, 22
trample-grant events; replay matched and fresh save round-tripped. Digest
`b711d966f9bd2e60187d4eafc5be852eea01b5f141fcc253813b1c38c8a85677`, snapshot SHA256
`69497ca2f6c8ac51c44726e78652bde1ad1903ee677af8d00b7cc0d324369471`.

## Gaps
- No dedicated cleanup-expiry, battlefield-departure, or same-ID bounce/recast saddle test.
- No test for explicitly multi-helper saddle beyond the offered minimal one-helper set;
  validation supports it and offers enumerate subsets.
- No full workspace matrix rerun; exact-head CI owns it.
