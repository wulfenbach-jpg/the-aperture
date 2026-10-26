# Issue 17 slice 14 — dynamic mana and threshold

Exact head reported after commit. No merge/master push. Full #17 unfinished:
**125 catalog / 459 union = 113 supported + 346 missing**, 1 certified / 7 planned.

## Sources

Three records extracted from the coordinator’s immutable
`evidence/issue17/slice13/candidate-source.json` (retrieved
`2026-10-26T17:44:38Z`) into `fixtures/starter-corpus/slice14-sourced.json`;
only these three, with Scryfall id/printing URL/retrieval stamp.

* **Arcane Signet** `{2}` Artifact — `{T}: Add one mana of any color in your commander's color identity.`
* **Fellwar Stone** `{2}` Artifact — `{T}: Add one mana of any color that a land an opponent controls could produce.`
* **Werebear** `{1}{G}` 1/1 Human Bear Druid — `{T}: Add {G}.`; Threshold +3/+3 with ≥7 cards in your graveyard.

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

## Genuine RED

Absent-catalog failure first; after scaffolding cards/fields/action with no
dynamic mana engine, tests failed with offered colors `[]` (Arcane/Fellwar),
Werebear sickness/threshold unavailable — real mechanic RED before implementation.
A test assertion was corrected to match the engine’s correct
“cannot produce red mana here” refusal wording; no production weakening.

## Verification

```
issue17_slice14            5 passed
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

* terminal, P2 wins, 1391 steps / 34 turns, digest `51fdc345d490…`
* separate replay matches 4304 events; fresh `--save-out` save reloads/replays to
  the same digest
* actual events: **Werebear cast 6 times**; Arcane Signet/Fellwar Stone were
  drawn but not cast/activated; **0 dynamic mana activations** in this product
  heuristic trace. Focused tests prove casts/activations; no inferred counts.

## Corpus/docs

New audit only: `evidence/issue17/slice14/corpus-audit.py` →
`corpus-state.json`. Catalog **125**, union 459, supported **113**, missing
**346**, manifest matches. Per-list: Isamaru 55, Jasmine 0, Talrand 51, Krenko
48, Ayula 39, Elas 53, Tatyova 52, Zada 53. Eight desired snapshots unchanged.
Current docs counts refreshed; historical evidence untouched.

## Remaining gaps

No desired 100-card starter newly certified. Product heuristic ignores artifact
casting and nonland mana activation (evidence-only/focused tests cover it; no AI
policy edit). Commander’s Sphere/Friendly Teddy/Aetherize exclusions and broader
Commander mechanics remain open.
