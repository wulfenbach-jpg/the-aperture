# Issue 17 slice 8 — independent delta review

**review_outcome: REQUEST_CHANGES**  
Reviewed head: `818afdb8afbb5977b6e08b085654304e4b5754bf`  
Base: `7e1eb5f1b8218e24d7a03377c3cb3c5bb0981a4f`  
`full_issue17_accepted: false`; #16 integration not approved.

## Findings

1. **Medium — stale corpus counts in tracked docs/manifest text.**  
   `docs/issue17-starter-corpus.md:34` says Isamaru has 56 missing; `:48` says
   100 supported / 359 missing; `:63` says Krenko 51 / Talrand 54.
   `decks/starter-corpus/manifest.json:142,204,335` says Talrand 54, Krenko 51,
   Elas 58. The verified counts are catalog **111**, union **459**, supported
   **99**, missing **360**; per-list Isamaru 55, Krenko 49, Talrand 51, Elas 55.
   The `missing_cards` arrays are correct; the prose/interaction strings are stale.
   Discriminator: `python evidence/issue17/slice8/corpus-audit.py` prints
   `99/360` and the actual per-list counts.

2. **Low — one named control overclaims.**  
   `razorclaw_triggers_once_with_two_blockers_and_not_when_unblocked`
   (`crates/core/tests/issue17_slice8.rs:402`) declares one attacker and two
   blockers but never tests an unblocked attack. Production code only queues
   `BecomesBlocked` for `turn.blockers`, so current behavior appears correct, but
   the named unblocked control is not exercised. Add an unblocked/no-trigger
   assertion or rename the test.

## Independently run checks

- Focused suite with `--locked`: slice8 8/8, tokens 10/10, slice6 13/13,
  counters/tokens/library 16/16, catalog 5/5, printings 6/6, issue13 43/43.
- Four-seat `package-slice8.txt` journey, seed 801, Isamaru each seat,
  commander-combat practice: terminal P0, 2377 actions / 68 turns, digest
  `ffbeaee0db38…`, 6087 events; four exact 40-card seat snapshots; separate
  replay matches.
- Real changed events: Soul's Attendant cast 3 times (all countered before
  resolution); Razorclaw cast 4 times with 2 blocked triggers; Volley Veteran
  cast 4 times with 2 ETB triggers. Dramatic Reversal/Goblin War Strike were
  drawn but not cast; focused tests, not this heuristic trace, prove them.
- `slice8-native-source-check.json`: all eight live Scryfall records match.
  Brightstone Ritual, Friendly Teddy and Aetherize have no catalog/CardId/code
  support and remain manifest-missing; the package excludes them.
- Original eight desired files, Jasmine list and historical slice3 audit are
  unchanged.

## Remaining gaps

Eight desired starters remain the gate (**1 implemented / 0 certified**; 360
union names missing). Brightstone/Friendly Teddy/Aetherize need bounded mana,
serialized multi-player draw continuation and attacker-list commander replacement.
Ayula tribal/counter text and broader legend/equipment/tutor/reanimation support
remain open. Goblin War Strike targets players only; no planeswalker permanents
exist in the pool, so its printed planeswalker half is not currently targetable.
This increment does not certify a starter or unlock release/#26/#32 lanes.
