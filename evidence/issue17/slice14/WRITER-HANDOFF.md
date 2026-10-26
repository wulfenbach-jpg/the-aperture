# Issue 17 slice 14 — dynamic mana and threshold

Exact head reported after commit. No merge/master push. Full #17 unfinished:
**126 catalog / 459 union = 114 supported + 345 missing**, 1 certified / 7 planned.

## Sources

Three records extracted from the coordinator’s immutable
`evidence/issue17/slice13/candidate-source.json` (retrieved
`2026-10-26T17:44:38Z`) into `fixtures/starter-corpus/slice14-sourced.json`;
only these three, with Scryfall id/printing URL/retrieval stamp.

* **Arcane Signet** `{2}` Artifact — `{T}: Add one mana of any color in your commander's color identity.`
* **Fellwar Stone** `{2}` Artifact — `{T}: Add one mana of any color that a land an opponent controls could produce.`
* **Werebear** `{1}{G}` 1/1 Human Bear Druid — `{T}: Add {G}.`; Threshold +3/+3 with ≥7 cards in your graveyard.
* **Commander's Sphere** `{3}` Artifact — `{T}: Add one mana of any color in your commander's color identity.` / `Sacrifice this artifact: Draw a card.` (added from the same immutable candidate source; four source records total).

## Behavior

* Dynamic colors are explicit legal `ActivateColorManaAbility` actions in stable
  WUBRG order; forged colors are refused before tap/mutation with unchanged
  digest. Activation is immediate (no stack/priority transfer), taps the source.
* Arcane Signet uses registered commander identity only (Isamaru W; Jasmine W/G;
  Krenko R; Talrand U; Tatyova U/G). No commander or empty identity offers no
  colors.
* Fellwar Stone uses live opponent-controlled lands’ typed basic mana types,
  including tapped lands, excludes own lands/artifacts/colorless/eliminated
  players; colors disappear when the land leaves.
* Werebear’s green tap ability is a creature mana ability: summoning sickness
  refuses it, later untap activation works. Threshold is a real layer 7c static
  (+3/+3 while controller has ≥7 cards in graveyard, any card type), updates both
  ways, excludes opponents’ graveyards, and stacks with typed counters and pumps.
* Existing Sol Ring/Mind Stone/Hedron Archive fixed mana behavior and Goblin/
  Drake/Beast/Pest token semantics unchanged.

## Commander's Sphere

Hand-cast/stack/battlefield test: dynamic colors offered from registered
commander identity, immediate mana activation, then sacrifice as a cost while
the artifact is tapped removes it before the draw ability resolves; draw goes on
the stack, resolves, and a second activation is refused. Implemented by reusing
the Arcane Signet dynamic choice plus Mind Stone sacrifice/draw machinery; no new
broad mechanism.

## Genuine RED

Absent-catalog failure first; after scaffolding cards/fields/action with no
dynamic mana engine, tests failed with offered colors `[]` (Arcane/Fellwar),
Werebear sickness/threshold unavailable — real mechanic RED before implementation.
A test assertion was corrected to match the engine’s correct
“cannot produce red mana here” refusal wording; no production weakening.

## Verification

```
issue17_slice14            8 passed
catalog_pool               5 passed
card_printings             7 passed
issue13_commander_combat  43 passed
issue17_slice13            4 passed
issue17_tokens            10 passed
cargo fmt --all --check   exit 0
cargo clippy --all-targets -D warnings  exit 0
cargo build --release -p aperture-cli   exit 0
```

## Journey

`decks/package-slice14.txt` — 40-card supported package, not a desired starter;
four seats, Krenko commander each, 40 life, seed 1401:

* the package file was updated to include Commander's Sphere (still 40 cards).
* default product-heuristic run, seed 1401: terminal P0, 1785 steps / 40 turns,
  digest `fdc9b089f8f9…`; 17 Werebear casts, **0 artifact casts and 0 mana
  activations**; replay and fresh save-out/reload match 5897 events.
* the evidence-only slice14 driver was adapted but **did not complete**: it cast
  and activated the three artifacts, but could not force a Werebear mana
  activation before termination, and its mandatory assertions therefore abort.
  It is left uncommitted with the exact incomplete command
  `CARGO_TARGET_DIR=C:/AI/aperture-worktrees/issue-17/target cargo run --manifest-path evidence/issue17/slice14/journey-driver/Cargo.toml`.
  Focused tests prove all four cards; no journey activation claim is inferred.

## Corpus/docs

New audit only: `evidence/issue17/slice14/corpus-audit.py` →
`corpus-state.json`. Catalog **126**, union 459, supported **114**, missing
**345**, manifest matches. Per-list: Isamaru 55, Jasmine 0, Talrand 51, Krenko
48, Ayula 39, Elas 53, Tatyova 52, Zada 53. Eight desired snapshots unchanged.
Current docs counts refreshed; historical evidence untouched.

## Remaining gaps

No desired 100-card starter newly certified. Product heuristic ignores artifact
casting and nonland mana activation (evidence-only/focused tests cover it; no AI
policy edit). Commander’s Sphere/Friendly Teddy/Aetherize exclusions and broader
Commander mechanics remain open.
