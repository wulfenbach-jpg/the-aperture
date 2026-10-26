# Issue 17 slice 14 — dynamic mana and threshold

Exact head reported after commit. No merge/master push. Full #17 unfinished:
**126 catalog / 459 union = 114 supported + 345 missing**, 1 certified / 7 planned.

This slice has one independent delta review at `472aeba` (APPROVED_INCREMENT,
nonblocking cleanup items fixed in the following documentation/cleanup commit).
The `CardDef::mana_choices` / `CardDef::creature_mana` fields the reviewer
flagged as dead were removed; behaviour lives only in the `CardId` accessors.

## Sources

Four records extracted from the coordinator’s immutable
`evidence/issue17/slice13/candidate-source.json` (retrieved
`2026-10-26T17:44:38Z`) into `fixtures/starter-corpus/slice14-sourced.json`;
all four, with Scryfall id/printing URL/retrieval stamp.

* **Arcane Signet** `{2}` Artifact — `{T}: Add one mana of any color in your commander's color identity.`
* **Fellwar Stone** `{2}` Artifact — `{T}: Add one mana of any color that a land an opponent controls could produce.`
* **Werebear** `{1}{G}` 1/1 Human Bear Druid — `{T}: Add {G}.`; Threshold +3/+3 with ≥7 cards in your graveyard.
* **Commander's Sphere** `{3}` Artifact — `{T}: Add one mana of any color in your commander's color identity.` / `Sacrifice this artifact: Draw a card.`

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

1. Absent-catalog first; after scaffolding cards/fields/action with no dynamic
   mana engine, offered colors were `[]` (Arcane/Fellwar) and Werebear
   sickness/threshold behavior failed.
2. Product legal-offer defects (fixed here): a ready Werebear was absent from
   `mana_sources`, so no `ActivateManaAbility` was offered; and tapped dynamic
   artifacts were still offered colors that `validate` refused. RED asserted
   “ready Werebear must be offered” and “tapped Arcane Signet must offer none”.
   Fix: `mana_sources` includes ready creature mana; dynamic offers require
   priority, no pending choice, and an untapped source. Driver-local tapped
   filters were removed; it now chooses only actual projected legal actions.
3. A slice12 test that still asserted Commander’s Sphere was excluded was
   updated to the now-supported truth.

## Verification

```
issue17_slice14           10 passed
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

* package includes all four new cards (still 40 cards, not a desired starter).
* **completed evidence-only driver** (`slice14/journey-driver`), seed 1414,
  exact four seats, 40 life, terminal P0, 2371 steps / 42 turns, digest
  `028cc2e33168…`, replay events 6665, replay digest matches:
  * casts: Arcane Signet 7, Fellwar Stone 7, Commander’s Sphere 7, Werebear 18
  * mana activations: Arcane 51, Fellwar 39, Sphere 7, Werebear 82
  * Sphere sacrifice-draw activations: 7
  * separate release CLI replay + fresh `--save-out` save reload/replay both
    reproduce `028cc2e33168` and 6665 events
* default product-heuristic trace retained separately: seed 1401, terminal P0,
  1785 steps, digest `fdc9b089f8f9…`, 17 Werebear casts, 0 artifact casts or
  activations. Product heuristic limitation remains; no AI policy edit.

## Corpus/docs

New audit only: `evidence/issue17/slice14/corpus-audit.py` →
`corpus-state.json`. Catalog **126**, union 459, supported **114**, missing
**345**, manifest matches. Per-list: Isamaru 55, Jasmine 0, Talrand 51, Krenko
48, Ayula 39, Elas 53, Tatyova 52, Zada 53. Eight desired snapshots unchanged.
Current docs counts refreshed; historical evidence untouched.

## Remaining gaps

No desired 100-card starter newly certified. Product heuristic ignores artifact
casting and nonland mana activation (evidence-only/focused tests cover it; no AI
policy edit). Friendly Teddy and Aetherize remain excluded; Commander's Sphere
is now implemented and tested. Broader Commander mechanics remain open.
