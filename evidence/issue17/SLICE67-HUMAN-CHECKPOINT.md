# Slice 67 checkpoint

**Product repair + demanded card, self-check only.** Same writer/history, branch
`aperture/issue-17`, draft PR46, base `7b99fcb…`. No push/merge/closure/unlock.

## R367 repair
- Removed the activation path's immediate `put_triggers` plus unconditional
  priority/Event::Priority. Activated ability stays on the stack; canonical
  settle recomputes characteristics, runs SBAs to stability, stages all waiting
  triggers above it, and grants priority only when no pending/resolving item
  remains.
- R367-1: exact offered Pashalik token sacrifice now yields valid pending state,
  no false priority, real sandbox SessionSave loads, and loaded/original
  continuation digests match.
- R367-2: Trashmaster sacrifice drops the anthem, the damaged token dies before
  target choice, the doomed token is not offered, and both Pashalik death
  triggers are staged exactly once; R365-1 ordering regressions unchanged.
- R367-3: slice66 in-engine journey is truthfully relabelled a fixture-driven
  integration scenario with injected combat; historical logs untouched.

## Siege-Gang Lieutenant
PLST M3C-113, `{3}{R}`, Goblin 2/2, fetched 2026-10-29T15:00:20Z (3 printings,
one oracle text). Implements the Lieutenant beginning-combat trigger only with
the controller's own commander (rechecked at resolution), two real 1/1 red Goblin
tokens with haste until end of turn, and `{2}, Sacrifice a Goblin`: 1 damage to
any target. Tests cover cast/resolve, commander present/absent/opponent, token
stats/haste, source departure and real activation. Skirk Prospector remains
missing (nonland mana ability not approximated).

## Counts/gates
- Fresh audit: **180 catalog / 459 union = 166 supported + 293 missing**;
  per-list `[55,0,51,35,0,52,52,53]`, Krenko **35**, **2/8 certified**.
- checkpoint 6, cards 5, ordering, slice62 7, slice63 10, printings 26, catalog 5,
  commander 43, slice15 2, replay_digest 6, import 3+1.
- Full workspace: `cargo test --workspace --all-targets --locked --no-fail-fast`
  **159 result lines / 1174 passed / 0 failed / 2 ignored, exit 0**; fmt 0,
  workspace all-target clippy -D warnings 0, workspace release 0
  (`slice67/native-check/workspace-repair.log`). Focused gates also 0.
- Journey: 1233 steps/26 turns, terminal P2, digest `e9bc716d…db3`, snapshot
  `2522cd7e…0732`; Krenko cast 4, Siege-Gang Lieutenant 4, 14 Siege tokens,
  15 attacks, 147 combat-damage events. Shipped CLI replay matches.
- Driver is an explicit 40-card evidence tool; injected combat is disclosed and
  it is not product AI/strength/GUI/100-card certification.

## R367-1/2 activator-priority cause and fix
The pre-repair failure was `command_zone_setup::ayula_ignores_opposing_bears...`
at line 2115: `pass: NotYourDecision { expected: 1, got: 0 }`. `apply_activate_ability`
left priority None and canonical settle granted it to the active player, but P0
was a non-active activator on P1's turn. `activation_priority` now records the
activator and `grant_priority` consumes it only after SBAs and trigger staging;
pending/resolving and R367-1/2 behavior stay intact. Both health: active activations
still receive the same seat they previously did, so accepted active-player cases
cannot regress; the full workspace is green.

## Honest TDD note
R367 has retained compiling REDs. No separate Siege pretest RED was retained;
that GREEN is not claimed as TDD.
