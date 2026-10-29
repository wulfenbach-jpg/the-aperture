# Issue 17 slice 9 — independent delta review

**review_outcome: REQUEST_CHANGES**  
Reviewed head: `ea17c272d61d0644ab2ce378870a50643c3db1e0`  
Base: `c22b240be304802ed31a7f7f9acb8d656d6218bf`  
`full_issue17_accepted: false`.

## Findings

1. **Medium — stale tracked corpus counts.**  
   `docs/issue17-starter-corpus.md:48` says **99 supported / 360 missing** and
   `:71` says Krenko **49** missing. The verified state is **102 supported / 357
   missing**, with per-list `55/0/51/48/46/55/54/53`; manifest arrays match.
   Discriminator: `python evidence/issue17/slice9/corpus-audit.py` prints
   102/357 and Krenko 48.

2. **Low — gate receipts are incomplete/empty.**  
   `evidence/issue17/slice9/repair-gates.log` is 0 bytes; `gates.log` contains
   only five build lines. The handoff’s test/fmt/clippy/build receipt claim is
   therefore not backed by its cited logs, although I independently reran the
   focused tests, fmt, core clippy and release build successfully.
   Discriminator: `wc -c repair-gates.log` = 0; `gates.log` has only Compiling/
   Finished lines.

## Checks

- Focused `--locked`: slice9 6/6, slice5 10/10, timing_mana 12/12, issue15
  effects 4/4, issue15 layers 4/4, catalog 5/5, printings 6/6.
- `cargo fmt --all -- --check`, `git diff --check`, core clippy `-D warnings`,
  release CLI build — all exit 0.
- Four-seat exact `package-slice9.txt`, seed 901, Krenko each seat,
  commander-combat practice: terminal P0, 1339 actions / 32 turns, digest
  `5d20bb4a35e2…`, 4379 events; four exact 40-card snapshots; separate replay
  matches.
- The heuristic cast **zero** of Brightstone Ritual/Mana Geyser/Rite of Flame in
  this trace; focused tests, not the journey, prove their behavior. No new
  ritual `ManaAdded` events occur in the game log.
- Old slice8 replay still matches `ffbeaee0db38…` / 6087 events.
- Audit: catalog 114, union 459 = 102 supported + 357 missing; manifest matches;
  desired list files unchanged. Slice9 source records match live printings;
  Friendly Teddy and Aetherize remain excluded.

## Scope gaps

Full #17 remains unfinished: **1 implemented / 0 certified** of eight desired
lists and 357 missing union names. Broader legend/equipment/tutor/reanimation and
Ayula tribal/counter text remain unsupported. This pass does not merge/close
#17, certify a starter, or unlock release/#26/#32 lanes.
