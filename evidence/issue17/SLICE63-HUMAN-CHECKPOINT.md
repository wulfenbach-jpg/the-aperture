# Slice 63 repair checkpoint

**Product batch with native-authorized companion; self-check only.** Same
writer/history, branch `aperture/issue-17`, draft PR46, base `a4ac11ca…`.
No push/merge/closure/unlock. Historical c58213a receipts are preserved under
`evidence/issue17/slice63/final-native/`; this repair's receipts are under
`repair-final-native/`.

## Delivered / repaired
- Shared no-abilities predicate now has the controller guard on the **other**
  Chieftain haste branch while retaining King's global grants, Warchief's
  self-inclusive controlled haste, own-other trample, effective subtype/creature
  and graveyard controls. Existing slice62 assertion is unchanged and GREEN.
- **Goblin Warchief** (DOM 130): Goblin spells {1} less generic; controlled
  Goblins have haste including itself.
- **Goblin King** (10E 207): other Goblins across all controllers get +1/+1 and
  mountainwalk; source self-excluded; second King boosts first.
- **Dragonlord's Servant** (TDC 210): Dragon spells {1} less generic, never
  Goblins.
- Native-authorized out-of-list **Volcanic Dragon** (CMR 207, `{4}{R}{R}`, 4/4,
  Flying, haste, 12 printings/1 text) is a real positive control. Tests prove
  one Servant reduces to `{3}{R}{R}`, two stack to `{2}{R}{R}`, missing red pip
  is refused, opponent's Servant does not reduce it, and the resolved Dragon has
  flying/haste and can attack while sick.
- Krenko commander-tax interaction: real command-zone test proves Warchief's
  reduction applies after tax (`{4}{R}{R}` taxed cost payable as `{3}{R}{R}`,
  refused without Warchief).
- Krenko `missing_cards` 46→43 and interaction prose updated together.
  Fresh audit: **172 catalog / 459 union = 158 supported + 301 missing**.

## Verification
- `issue17_slice63`: 10/10; slice62 7/7; Ruxa, reduction, core lib, card_printings
  22, catalog_pool 5, commander 43, slice15 2, replay_digest 6, import planned
  3 + analyzer 1.
- fmt, workspace all-target clippy -D warnings, locked workspace release: all
  exit 0; aggregate exit 0 (`repair-final-native/gates.log`).
- Journey: 4 seats/40 life, 954 steps/23 turns, terminal P3, digest
  `43bbcce4…8848`, snapshot `b2539164…fa6a`; Warchief 6, King 6, Servant 3,
  Volcanic Dragon 1, Dragon cast with Servant offered 1, 14 attacks, 44
  combat-damage events. Separate shipped CLI replay matches recorded.
- Evidence driver uses projected legal offers only; explicitly not product AI,
  strength, GUI or exact-100-card certification.

## Non-claims
Krenko remains planned at 43 missing; **2/8 certified**, not 3/8. Volcanic
Dragon is an authorized out-of-list control and does not increase desired-union
support. Independent review/publication/CI remain native-owner steps.
