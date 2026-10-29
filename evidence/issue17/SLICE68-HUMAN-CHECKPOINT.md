# Slice 68 checkpoint

**Product batch, self-check only — not independent review.** Same writer/history,
branch `aperture/issue-17`, draft PR46, base `2a4c8ff…`. No push/merge/closure.

## Delivered
- **Skirk Prospector** (DMR 137, fetched 2026-10-29T19:26:17Z): real CR605.1a
  nonstack mana ability. Typed action names source and sacrificed Goblin; every
  legal controlled Goblin is offered (including itself), wrong subtype/opponent
  choices are refused atomically, no tap or summoning restriction, mana is added
  immediately with no response window. Cost deaths queue normal dies triggers and
  stage through the canonical post-payment checkpoint. Produced mana pays a real
  Goblin War Strike; pending-trigger sandbox save loads.
- **Zo-Zu the Punisher** (DDS 41, fetched 2026-10-29T19:26:18Z): typed
  AnyLandEnters watcher captures the entering land's controller, deals 2 through
  the normal damage replacement pipeline, works for opponents and after source
  departure, and does not fire for creatures. Existing own-only landfall cards
  remain unchanged.
- Manifest updated together: Krenko missing 35→33. Fresh audit:
  **182 catalog / 459 union = 168 supported + 291 missing**.

## Verification
- `issue17_slice68` 5/5; checkpoint 6; ordering; command_zone_setup 43;
  printings 27; catalog 5; commander 43; slice15 2; replay_digest 6; import 3+1.
- All focused gates/fmt/clippy/release exits 0; aggregate 0
  (`slice68/final-native/gates.log`).
- Journey: 1326 steps/26 turns, terminal P2, digest `e4603396…d612`, snapshot
  `6d34434d…f32b`; Skirk cast 7, Zo-Zu 3, Goblin War Strike 7, 7 mana sacrifices,
  31 Zo-Zu triggers, 13 attacks, 79 combat-damage events. Shipped CLI replay
  matches recorded.
- Driver is an explicit 40-card evidence tool using projected offers; not product
  AI/strength/GUI/100-card certification.

## TDD / residual
No pre-implementation compiling RED was retained for Skirk or Zo-Zu; tests were
written after implementation, so this is **not** claimed as TDD. Krenko remains
planned at 33 missing; **2/8 certified**, not 3/8. No full workspace rerun was
performed in this slice; the prior successful full-workspace receipt is retained.
