# Slice59 final repair checkpoint

- Branch `aperture/issue-17`, same writer/session. Initial local heads
  `c6b62eb` + `cc5cd00` are superseded; the repair commit is the final local head.
  No push, merge, master push, closure, unlock or nested agents.
- Native five-RED repair suite is green without weakening: `issue17_slice59_repairs`
  5/5. Added real-trigger regressions 9/9 in
  `issue17_slice59_ring_repairs.rs`: Ring2 stack + resumable discard/save, Ring3
  response + EndCombat delayed stack + bounce/recast, Ring4 positive damage/Fog/all
  opponents, Ring1 block legality/legend ownership, two Rangers/source-left/captured
  controller, emblem abilities not bearer abilities.
- The Ring is `CardId::TheRing`, an engine-only emblem excluded from catalog/by_name/
  import/pool count. All four abilities use TriggerRef/stack/APNAP; level 4 loses life
  on resolution after SBAs. Empty Ring state remains omitted until a temptation.
- Whole-field audit `AUDIT OK`, 165 catalog / 152 supported / 307 missing / Ayula 1.
- Gates with own receipt: fmt 0, workspace all-target clippy -D warnings 0, locked
  workspace release 0, focused suites 0 (`slice59/final/gates.log`).
- Final journey: 4 seats, 40 life, exact 40-card package, seed 2323, terminal P3,
  1749 decisions/44 turns, digest `d335cb52…321e`, snapshot `0607cfa7…f4c0`;
  8 Rangers casts, 3 Ring choices, 65 combat-damage events. CLI replay matches;
  non-sandbox journey save loads and restores to the same digest.
- Limits: emblem has no physical zone object; full control-change/replacement Ring
  interactions are not claimed. 40-card journey is not 100-card certification.
  #17 remains open; 1 of 8 lists certified. Reporter pointer still old #20; not touched.
