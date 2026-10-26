# Slice 17 human checkpoint

- Branch: `aperture/issue-17`
- Implementation commit: `7e4a013`
- Base: `6f294cf`
- Status: **PARTIAL #17**; eight-list gate remains open.
- Review: not launched, per instruction.

## Accepted repairs
1. Cleanup is same-turn: death history survives repeated `finish_cleanup`;
   reset happens only at an actual `turn.number` increment.
2. Each-player draw has explicit serialized per-seat progress. Commander
   replacement at an earlier seat no longer skips later live seats; save/load
   mid-suspension validates and reloads; no early priority.
3. Counter placement emits the authoritative counter event.
4. AI delta disclosed as field initialization/copy only.

## Verification
Focused: slice17 5, cleanup lib 1, command_zone_setup 34, replay_digest 6,
ordering_persistence 2. `cargo clippy --workspace --all-targets -D warnings`
clean; release CLI built; fmt applied. Journey seed 1616 terminal win seat 3,
digest `85fabffc9aa16a0f9ba53a5f64a751c43bb9ae66a3ce96f3e7402f1c633c7027`,
replay matches, fresh save round-trips.

## Remaining gaps
- Ayula: 3/34 sourced; 31 missing.
- Seven of eight desired 100-card lists remain planned/partial.
- No full workspace rerun after repair (instruction), no 100-card Commander game.
- Old #20 reporter pointer remains; not touched here.
- No commit push/merge/PR/issue closure.
