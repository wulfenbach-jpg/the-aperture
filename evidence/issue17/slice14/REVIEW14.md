# Issue 17 slice 14 — independent delta review

**review_outcome: APPROVED_INCREMENT**  
Exact head: `472aeba1e06b3a657647219815f8cc2380106cb4`  
Base: `a4db05f`  
`full_issue17_accepted: false`.

## Verified

- Focused `--locked`: slice14 **10/10**, slice13 4/4, slice12 11/11,
  tokens 10/10, issue13 43/43 (including the APNAP source-left discriminator),
  catalog 5/5, printings 7/7.
- Dynamic offers: ready Werebear is offered; sick/tapped Werebear is not;
  tapped dynamic artifacts offer no colors; every offered dynamic action
  validates; forged colors and wrong state are refused before tap. Arcane
  identity, Fellwar opponent-land colors (including tapped, excluding own/
  eliminated), Werebear threshold both ways/layers, and Sphere sacrifice while
  tapped all pass focused controls.
- Demo journey: terminal P0, 2371 actions/42 turns/6665 events, digest
  `028cc2e33168`; fresh save-out reloads/replays the same digest. Old slice8
  replay still `ffbeaee0db38`/6087.
- Actual demo counts: Arcane 7, Fellwar 7, Sphere 7, Werebear 18 casts;
  mana activations 51/39/7/82; 7 Sphere draws. Driver is evidence-only, not
  product strength.
- Native source receipts match; catalog 126, union 459 = 114 supported + 345
  missing; desired files unchanged; manifest matches.
- `cargo fmt --check` and `git diff --check` pass.

## Nonblocking findings

- `CardDef.mana_choices` and `CardDef.creature_mana` remain `None` for all
  catalog cards; behavior is driven by `CardId::dynamic_mana()`/
  `creature_mana()`. No consumer reads the dead fields, but the public catalog
  fields are inconsistent and should be removed or populated in cleanup.
- `docs/issue17-starter-corpus.md:67` still says Commander’s Sphere is
  unsupported, though it is implemented and tested.
- Handoff says slice14 has 8 tests (final 10); its last paragraph still lists
  Sphere among exclusions; demo JSON includes nonartifact names in
  `artifact_casts`/`mana_activations` keys (values are per-card). Known prose
  metadata; no behavior impact.

## Remaining gaps

Full #17 is **1 certified / 7 planned**, with 345 union names missing. The
product heuristic still casts no artifacts and does not auto-activate nonland
mana; no AI policy change. Broader Commander mechanics and Friendly Teddy/
Aetherize remain open. Current exact-head CI is pending/not green; no merge,
closure or release/#26/#32 unlock.
