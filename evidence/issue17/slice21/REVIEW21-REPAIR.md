# Slice 21 repair review — regeneration shield transfer

**APPROVED_INCREMENT** at `f59d29be91b77a9f234d268d438f42039915213d` (base `d9d2c94`).

The earlier `REVIEW21.md` **CHANGES_REQUESTED** is preserved. This is a delta
adjudication of the repair, not a retroactive approval of the failed head.

## Repair verified

- The only engine change is the 8-line `detach_object` hunk: when the detached
  object is actually on the battlefield, shields keyed to its id are dropped.
  No serialized field, global rekey, or change to non-commander identity was
  introduced.
- The corrected test now Unsummons the exact same Dragon-Scarred Bear id,
  offers/recasts that same card, confirms the shield is gone, then Shocks it;
  it dies. This reproduces the prior transfer path and demonstrates the fix.
- Exile, sacrifice, and zero-toughness cases now assert the shield is ended on
  battlefield departure and was not applied as regeneration.
- Harness state is consistent: the shield is still consumed during actual
  regeneration (lethal, multiple-shield, blocked-attacker and save/reload tests
  remain green).
- No dedicated source-left test exists; inspected code only: `RegenerateSource`
  checks the source is on the battlefield at resolution. I do not claim a
  dedicated test for that edge.

## Commands actually run

- `cargo test -p aperture-core --test issue17_slice21 --locked`: **8/8 passed**.
- `cargo test -p aperture-core cleanup_expires_regeneration_shields --locked`:
  **1/1 passed**.
- Release replay `evidence/issue17/slice21/demo-replay.json`: 1906 actions,
  4909 events, digest `0d20c7597183` matches.
- Legacy release replay `evidence/issue17/slice20/demo-replay.json`: 2064
  actions, 5281 events, digest `72cd69fb1143` matches.
- `git diff --check`: exit 0.
- Reused native `identity-repair-gates.log`: slice21 8, cleanup unit, focused
  pins, fmt, clippy, and release all exit 0.

No blocking finding. Full #17 remains **1/8 certified**, **333 missing**,
Ayula **27**; no merge/closure and exact-head CI remains parent-owned.
