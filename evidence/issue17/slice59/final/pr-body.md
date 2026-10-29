## Slice 59 — Dúnedain Rangers and The Ring (bounded, trigger/stack model)

Same draft PR, branch `aperture/issue-17`. Original #17 remains **unfinished**: **1 of 8
lists certified**, Ayula one name short (Evercoat Ursine). No merge, master push, issue
closure or dependent unlock.

**This is a candidate for independent increment review, not a self-approval.** The first
Ring draft in this slice was rejected by native prepublication tests (0 passed / 5 failed)
and repaired; the second native correction fixed a legend-rule defect (5 passed / 2 failed
before the fix). Both REDs are retained in `evidence/issue17/slice59/`.

### Card
**Dúnedain Rangers** — LTR 159, `{3}{G}`, Human Ranger 4/4. Oracle text fetched from
Scryfall (immutable `630e1e36…`, oracle `7de2a364…`, retrieved 2026-10-29T04:29:30Z) and
recorded in `fixtures/starter-corpus/slice59-sourced.json`. Its printed landfall Ring
temptation is a real trigger with a `NoRingBearer` intervening-if evaluated at queue and
resolution; a source that left the battlefield still uses its captured controller.

### The Ring (engine-only emblem, never importable)
`CardId::TheRing` is excluded from `catalog::all`/`by_name`/`pool_size`/import. Its four
printed abilities are normal `TriggerRef`/stack/APNAP abilities, not declaration or damage
side effects:

1. **Level 1** — the Ring-bearer is legendary and can't be blocked by greater power.
2. **Level 2** — attack trigger draws, then a distinct resumable `EffectDiscard`; it does
   not run cleanup or end combat.
3. **Level 3** — becomes-blocked trigger resolves to a delayed EndCombat stack trigger; a
   bounced/recast incarnation is not hit.
4. **Level 4** — only positive combat damage to a player triggers; Fog prevents it; each
   live opponent loses 3 on resolution after SBAs.

**CR 704.5j correction:** the legend SBA now groups only legendary permanents (printed
legendary or that controller's level-1+ Ring-bearer). One legend beside an ordinary
same-name card is not a choice, the offer must be the complete group, and a forged truncated
choice is refused by `GameState::validate` and a real `SessionSave::from_json` load.
Commander-only refusal text no longer claims the legend rule is unimplemented.

### Executed evidence (local)
- `issue17_slice59_repairs` **7/7**, `issue17_slice59_ring_repairs` **9/9**,
  `issue17_slice59` **3/3**, focused suites 79/79 in aggregate; fmt 0, workspace all-target
  Clippy `-D warnings` 0, locked workspace release 0, whole-field corpus audit 0
  (`evidence/issue17/slice59/final/gates.log`, each command's own exit).
- A whole-workspace run recorded one failure: a commander-only reason assertion that
  encoded the retired legend claim. It was corrected and that suite rerun **44/44 exit 0**;
  no full green local workspace total is claimed, exact-head CI supplies that gate.
- **Corpus:** 165 catalog / 459 union = **152 supported + 307 missing**, per-list
  `[55,0,51,48,1,52,52,53]`, 1 of 8 certified; both Ayula gap fields rewritten together.
- **Final release journey:** 4 seats, 40 life, exact 40-card package, seed 2323, terminal
  P3, **2017 actions / 49 turns / 5090 events**, digest
  `7936133df8f6aa883cfb6f03e514896c62ae33b4d2c40b8f2e83e356620a1e50`, snapshot
  `0607cfa7…f4c0`; 9 Rangers casts, 4 Ring choices, levels `[0,1,1,2]`, 68 combat-damage
  events. Non-sandbox save round-trips to the same digest; separate release CLI replay
  reproduces 2017 actions / 5090 events (`slice59/final/replay.log`, exit 0). This is an
  evidence-only masked demonstrator, not product AI/strength/GUI or 100-card certification;
  it reaches level 2 only, with levels 3–4 covered by focused trigger tests.

### Honest limits
The emblem is persisted per seat as `RingState`, not a physical zone object; the bearer id
on a trigger is combat-event provenance. No complete Ring control-change/replacement
interaction set. 40-card journey is not a starter certificate. Original #17 remains open.

Exact-head CI for the code commit is recorded separately from this evidence tip; the PR body
comment will name it once both workflows complete.
