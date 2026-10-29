# Issue 17 slice 13 — independent delta review

**review_outcome: APPROVED_INCREMENT**  
Exact head: `1d99b74371f06bdb645a3d4494ddb48e02d81e14`  
Base: `985d8ff`  
`full_issue17_accepted: false`.

## Verified

- Slice13 focused: 4/4 (Sazh controller-only typed counter; Baloths Beast
  source-left; Professor Pest dies trigger exactly once; sourced printings).
- Strict archival controls: slice12 11/11; the exact
  `four_seat_co_loser_apnap_source_departure_continues_survivors` test passes
  with the new token-only guard. My trigger_queue/revealed/hand-list probe now
  rejects all three; live-owner Removed and real off-battlefield token controls
  remain rejected.
- UI sentinel tests: UI lib 1/1, UI session 22/22; import `start_flow` 21/21.
  Changes are test fixtures only; no production UI/import behavior.
- Seed1301 real journey: terminal P1, 1568 actions/34 turns/5018 events, digest
  `4cbaa34f0de1`; fresh save-out reloads and replays the same digest. Old slice8
  replay still matches `ffbeaee0db38`/6087 events.
- Log events: Sazh 2, Baloths 3, Professor 4 casts; 18 +1/+1 counter events;
  1 Beast and 4 Pest creations. No Pest dies-trigger/life line in this terminal
  trace; the focused test proves it.
- Native source check all seven fields true. Catalog 122, union 459 = 110
  supported + 349 missing; desired snapshots unchanged. Beast Token/Pest Token
  are CR111.4 names and nonimportable.
- `cargo fmt --check` and `git diff --check` pass.

## Nonblocking accuracy notes

- Handoff says “no Pest death occurred”; the log records a Pest put into a
  graveyard and ceasing at the terminal mass-death/game-over point, but no
  Pest dies-trigger/life line. The focused test proves the trigger.
- Handoff “gains life before the token ceases” is loose: the trigger is queued
  before CR 111.7 cessation; the life effect resolves later from the stack.
- `slice13/gates.log` records only release-build output; the test/fmt/clippy
  receipt is not fully represented there. I independently ran the focused suites.

## Remaining gaps

Full #17 is **1 certified / 7 planned**, with 349 union names missing. Artifact
mana remains explicit-only for the product heuristic; Friendly Teddy, Aetherize,
Commander’s Sphere and broader legend/equipment/tutor/reanimation/Ayula/Tatyova
mechanics remain open. Current exact-head CI is not claimed green. No merge/
closure or release/#26/#32 unlock follows from this increment.
