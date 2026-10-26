# Slice 12 review round 2 — save/archive adjudication

**review_outcome: REQUEST_CHANGES**  
Exact head: `5870716ff523b042ac6ea57cdee9d04def75c81e`  
Base: `9ec90aa`  
`full_issue17_accepted: false`.

## Blocking finding

**Archived-token validation remains too permissive.**  
`crates/core/src/state.rs:1588-1612` admits a departed owner’s token in internal
`Zone::Removed`, but only checks player zone lists, stack object ids and combat.
It does not check `trigger_queue`, `pending`/`resolving`, stack targets,
`effects`/`shields`, `activation_uses` or `revealed`.

Review probe: load `evidence/issue17/slice12-review-demo-save.json`, push a
`TriggerRef { source: 541 (archived Goblin Token), card: GoblinToken, ... }` into
`trigger_queue`, then `validate()`:

```
VALIDATED archived token in trigger_queue
```

The same probe with the archived id inserted in `revealed` also validates. This
is exactly the phantom-trigger/visibility class the repair was meant to refuse.
Reject archived tokens referenced by any live structure and add strict negative
controls; keep ordinary BF→GY token cessation unchanged.

## Verified repairs

- **R12-F1 ordinary saves fixed:** fresh save-out of the demo replay writes
  `d3eade58f133`; replaying the fresh save exits 0 and recomputes the same digest
  (1573 actions/6981 events). Retained `slice12/save.json` reloads at
  `328dadbdbdc6` (1746 actions/7051 events); old slice8 replay still matches
  `ffbeaee0db38`/6087 events.
- **R12-F2 fixed:** docs now say Elas has **53** unsupported nonland names,
  matching manifest/audit.
- **CI sentinel fixed test-only:** `start_flow` 21/21 passes with Darksteel Ingot
  replacing Sol Ring; absent-catalog guard preserved. No production import/UI
  edits.
- Focused core: slice12 10/10, tokens 10/10, slice6 13/13, issue13 43/43,
  catalog 5/5, printings 7/7; fmt/diff-check pass.

Artifact behavior, source receipts and the representative `d3eade58f133`
journey are reused unchanged. Current CI is not claimed green; the prior
`9ec90aa` failure was the test sentinel only.

## Remaining gaps

Full #17 is **1 certified / 7 planned** desired lists: 119 catalog, 459 union =
107 supported + 352 missing. Artifact mana remains explicit-only for the product
heuristic; Commander’s Sphere, Friendly Teddy, Aetherize and broader mechanics
remain unsupported. No merge/closure or release/#26/#32 unlock. The handoff’s
“No push yet” is stale metadata, nonblocking.
