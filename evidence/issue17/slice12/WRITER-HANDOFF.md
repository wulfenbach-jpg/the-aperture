# Issue 17 slice 12 — artifact mana-rock package handoff (cast repair)

Three sourced artifacts are supported and castable from hand. No push; full #17
unfinished.

## Implemented

| card | printing | behavior |
| --- | --- | --- |
| Sol Ring | frc/21 | `{T}: Add {C}{C}` immediate mana ability |
| Mind Stone | mbc/76 | `{T}: Add {C}`; `{1}, {T}, Sacrifice: Draw a card` |
| Hedron Archive | fdn/726 | `{T}: Add {C}{C}`; `{2}, {T}, Sacrifice: Draw two` |

Live Scryfall named printings fetched before implementation, stored in
`fixtures/starter-corpus/slice12-sourced.json` with printing URL, Scryfall id and
actual UTC `2026-10-26T17:27:16Z`.

**Commander's Sphere excluded**: its mana ability produces a color in the
commander's color identity, which the engine does not evaluate. Not added or
stubbed.

## Engine behavior

* `Kind::Artifact` is a real noncreature permanent kind; artifacts are not
  creatures/lands. `CardDef::spell_spec`/`is_permanent_spell` now include
  artifacts, so an artifact in hand is a real noncreature permanent spell that
  resolves onto the battlefield.
* `CardDef::artifact_mana` declares the CR 605 tap-mana ability. Legal
  activation is immediate: no stack entry and no priority transfer, the source
  actually taps, and a second activation is refused with unchanged state.
* Artifacts do not receive creature summoning sickness; the existing
  priority/pending-choice gate is preserved (no bypassing an existing choice).
* Artifact mana is not auto-paid by the land-only payment planner. Players must
  activate it explicitly; focused tests do, and the heuristic limitation is
  disclosed.
* Mind Stone/Hedron Archive sacrifice-draw abilities are ordinary typed
  activated abilities using the stack, `{1}`/`{2}` generic costs, tap and
  sacrifice-source costs, correct ability index 0, real graveyard movement and
  source-left resolution.

## Genuine RED

The first slice12 RED was **absent-catalog**, not the mana mechanic: all initial
tests failed at `catalog::by_name`. After scaffolding records, the mana/ability
tests failed until `artifact_mana` and the activated abilities existed.

A second, later RED at `fd18dfc` proved the real castability defect: with real
`Action::CastSpell` from hand the engine replied “Sol Ring is not a spell”,
because `spell_spec`/`is_permanent_spell` only admitted creatures. Fixed by
including artifacts canonically; no card-specific shortcut. The new cast tests
cover all three artifacts, sorcery-timing refusal, Negate countering an artifact
as a noncreature spell, same-turn tap-then-no-untap refusal, and real
resolution/activation.

## Verification

```
cargo test -q -p aperture-core --test issue17_slice12 --test issue17_slice9 \
  --test issue17_tokens --test catalog_pool --test card_printings \
  --test issue13_commander_combat
  -> 9, 6, 10, 5, 6, 43 passed
cargo fmt --all --check
cargo clippy -q --all-targets -- -D warnings
cargo build --release -p aperture-cli
  -> exit 0 (evidence/issue17/slice12/gates.log)
```

Tests cover: sourced text/type; immediate Sol Ring mana with no stack/priority;
tapped repeat refusal with state digest unchanged; Mind/Hedron printed mana
amounts; sacrifice-draw stack, zone change and source-left draw; insufficient
mana refusal; artifact noncreature semantics.

## Journey

`decks/package-slice12.txt` — 40-card supported artifact package, not a desired
starter; four seats, Krenko commander each, 40 life, seed 1201:

* terminal, P3 wins, 1746 steps / 42 turns, digest `328dadbdbdc6…`
* separate replay matches 7051 events; save replay matches
* actual artifact **cast** events in this trace: **0**; actual mana activations:
  **0**. The package still contains lands/creatures and terminates, but the
  unmodified heuristic never selects an artifact spell or
  `ActivateManaAbility`; focused tests prove casting and activation. This is
  disclosed as a heuristic limitation, not inferred support. Digest is unchanged
  from the pre-fix run (`328dadbdbdc6…`) because the trace never touches artifacts.

## Corpus audit

New audit only: `evidence/issue17/slice12/corpus-audit.py` →
`corpus-state.json`. Current: catalog **119**, union 459, supported **107**,
missing **352**, manifest matches. Per-list: Isamaru 55, Jasmine 0, Talrand 51,
Krenko 48, Ayula 43, Elas 53, Tatyova 54, Zada 53. Desired snapshots unchanged.
Docs current counts and islandwalk citation (`CR 702.14c`) corrected; historical
checkpoints not rewritten.

## Native representative journey (after writer stopped)

The native coordinator supplied an evidence-only masked, artifact-preferring
legal-action driver at `journey-driver/` without changing the product AI. It
reads the exact same `decks/package-slice12.txt` into all four replay snapshots;
no fixture state injection, replacement decks or rejected-action fallback.
Seed1201 reaches terminal P3 in1573 accepted actions/32turns, digest
`d3eade58f133c278114cb1289b5ac42e1be51999b4179faca972ec9800fac0ec`;
replay6981events matches. Real casts SolRing7/MindStone8/HedronArchive10;
real mana activations53/7/6 respectively; stacked sacrifice/draw activations
MindStone8/HedronArchive9. See `demo-journey.json` and `demo-replay.json`.
This demonstrates the package, not a new starter certificate or product policy.
The original unmodified-heuristic journey and its zero-artifact limitation above
remain truthful and retained. Native live immutable source checks match all six
recorded fields on each of the three cards (`native-source-check.json`).

## Remaining gaps

No 100-card desired starter is newly certified. Artifact mana is explicit, not
auto-paid by the heuristic; no AI policy edits. Commander's Sphere remains
excluded. Friendly Teddy/Aetherize exclusions and broader legend/equipment/tutor/
reanimation/Ayula gaps remain.
