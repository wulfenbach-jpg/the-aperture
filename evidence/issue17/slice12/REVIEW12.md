# Issue 17 slice 12 — independent artifact delta review

**review_outcome: REQUEST_CHANGES**  
Reviewed head: `9ec90aa7ff45d9d20981321b64da16234ba0b776`  
Base: `c10ebfe`  
`full_issue17_accepted: false`.

## Findings

1. **High — save roundtrip is broken on the representative artifact journey.**  
   `replay --file slice12/demo-replay.json --save-out <new>` writes the save
   (digest `d3eade58f133`), but replaying that fresh save exits 1:
   `token #541 (Goblin Token) is in the removed from the game zone; tokens
   cease to exist off the battlefield`. The retained
   `evidence/issue17/slice12/save.json` fails likewise at token #507. The
   writer handoff’s “save replay matches” claim is false for both. A departing
   player’s tokens must not leave an unreloadable canonical state; add a save
   roundtrip regression on this package.
2. **Medium — stale Elas prose.** `docs/issue17-starter-corpus.md:120` says
   Elas has 55 unsupported nonland names; manifest and corpus audit say **53**.
   Update the prose to 53.

Artifact casting/activation itself checks out: clean hand→stack→battlefield
casts, immediate Sol Ring/Mind Stone/Hedron mana with no stack/priority change,
tap refusal with unchanged digest, sacrifice-draw source-left resolution,
noncreature Negate/combat semantics, and sorcery-timing refusal.

## Checks

- Focused `--locked`: slice12 9/9, slice9 6/6, tokens 10/10, catalog 5/5,
  printings 7/7, issue13 43/43. `cargo fmt --check` and `git diff --check` pass.
- Native demo replay: 1573 actions/32 turns, terminal P3, digest `d3eade58f133`,
  6981 events, four exact 40-card package snapshots; actual casts Sol Ring 7 /
  Mind Stone 8 / Hedron 10 and mana activations 53/7/6. Driver is evidence-only,
  not product AI.
- **Save-out then save replay failed** as above. Old slice8 replay still matches
  `ffbeaee0db38…` / 6087 events.
- Source check: all six live Scryfall fields match for all three artifacts;
  Commander’s Sphere remains excluded. Catalog 119, union 459 = 107 supported +
  352 missing; manifest arrays match and desired deck files are unchanged.
- Unmodified product heuristic casts zero artifacts; focused tests, not the
  product journey, prove the package behavior.

## Scope / gaps

Full #17 remains unfinished: **1 certified / 7 planned** desired lists and 352
missing union names. Artifact mana is explicit-only for the AI (no policy edit);
Commander’s Sphere, Friendly Teddy, Aetherize and broader
legend/equipment/tutor/reanimation/Ayula support remain open. No original #17
acceptance, merge/closure, or release/#26/#32 unlock follows.
