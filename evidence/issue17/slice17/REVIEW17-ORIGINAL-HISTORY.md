# Slice 17 — true original-history adjudication

**APPROVED_INCREMENT** at `3934d765158733a8351a594ed27564cb14431cb8`.
This is the true original reviewer history; the earlier `slice17` review launch
used the wrong `review-sessions/` path and produced a separate new history. I read
its `REVIEW17.md` as evidence but independently re-ran the focused checks below.

## Results

- `cargo test -p aperture-core --test issue17_slice17 --test command_zone_setup --test replay_digest --locked`:
  **6/6 slice17**, **34/34 command-zone**, **6/6 replay-digest**. The six slice17
  tests include the strengthened no-death/actual-counter control, real target
  offer/selection, token-only morbidity, owner-vs-controller Caller count, and
  actual turn-boundary reset.
- `replay --file evidence/issue17/slice17/demo-replay.json`: 4 seats, 1866
  actions, terminal P3, 4952 events, digest `85fabffc9aa1` matches.
- `replay --file evidence/issue17/slice17/demo-save.json`: stored/recomputed
  digest `85fabffc9aa1` match; save replay reproduces 1866 actions.
- Source fixture has immutable printing URLs and retrieval timestamps; native
  audit 134 catalog / 122 supported / 337 missing, Ayula 31 missing, 1 certified.
  Desired snapshots unchanged.

## Assessment

The repaired EachPlayer draw resume carries source/cursor/next seat/cards-left/
per-seat/done/live-total, validates mid-commander-replacement suspension, and the
commander replacement regression proves later live seats draw exactly once.
Cleanup reset happens only on actual turn-number advance, and the six morbid
tests now assert actual battlefield counters rather than vacuous queue state.
Ownership counting, token exclusion, milling exclusion, and later zone moves are
covered. No product-source defect found.

**Limitations:** the focused tests do not independently exercise a mass-
destruction batch containing Teddy; Caller's ownership/count test is not a
dedicated simultaneous-death batch; the validator's triggered-source exception is
broader than this slice; full workspace was not rerun. Exact-head CI is reported
10/10 SUCCESS including required checks. This is an increment only: original #17
remains 1/8 certified and 337 union names missing; no full acceptance, merge,
closure, or downstream unlock.
