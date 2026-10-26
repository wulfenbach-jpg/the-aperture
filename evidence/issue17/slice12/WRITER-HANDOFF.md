# Issue 17 slice 12 — artifact mana-rock package handoff

Three sourced artifacts are supported. No push. Full #17 unfinished.

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
  creatures/lands.
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

Before cards/engine work, all six slice12 tests failed at catalog lookup
(`Sol Ring must be supported`, etc.). After scaffolding card records the
artifact mana/payment tests still failed until `artifact_mana`, the mana-source
scan and the activated abilities existed. No retrospective TDD claim for older
slices.

## Verification

```
cargo test -q -p aperture-core --test issue17_slice12 --test issue17_slice9 \
  --test issue17_tokens --test catalog_pool --test card_printings \
  --test issue13_commander_combat
  -> 7, 6, 10, 5, 6, 43 passed
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
* actual artifact activation count in this trace: **0**. Mind Stone/Hedron
  Archive were drawn (and some discarded) but the simple heuristic never cast or
  activated an artifact; focused tests prove activation. No inferred count.

## Corpus audit

New audit only: `evidence/issue17/slice12/corpus-audit.py` →
`corpus-state.json`. Current: catalog **119**, union 459, supported **107**,
missing **352**, manifest matches. Per-list: Isamaru 55, Jasmine 0, Talrand 51,
Krenko 48, Ayula 43, Elas 53, Tatyova 54, Zada 53. Desired snapshots unchanged.
Docs current counts and islandwalk citation (`CR 702.14c`) corrected; historical
checkpoints not rewritten.

## Remaining gaps

No 100-card desired starter is newly certified. Artifact mana is explicit, not
auto-paid by the heuristic; no AI policy edits. Commander's Sphere remains
excluded. Friendly Teddy/Aetherize exclusions and broader legend/equipment/tutor/
reanimation/Ayula gaps remain.
