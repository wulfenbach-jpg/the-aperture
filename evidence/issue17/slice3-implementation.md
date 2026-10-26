# Issue 17 slice 3 — Tatyova landfall / Jaddi Offshoot handoff

Implemented sourced **Tatyova, Benthic Druid** (commander-only landfall: gain 1
life then draw) and **Jaddi Offshoot** (real defender + landfall gain 1 life).
One exact supported 40-card package fixture was played four-seat with replay and
save parity. The desired 100-card Tatyova list remains `planned` with 56 missing
names. No commit/push. Full #17 incomplete.

## Source records

Both cards come from the stored real Scryfall planned-list printing response:

* Tatyova, Benthic Druid — `fdn/247`, `{3}{G}{U}`, Legendary Creature — Merfolk
  Druid, 3/3, “Landfall — Whenever a land you control enters, you gain 1 life
  and draw a card.”
* Jaddi Offshoot — `tdc/260`, `{G}`, Creature — Plant, 0/3,
  “Defender\nLandfall — Whenever a land you control enters, you gain 1 life.”

Retrieved `2026-10-25T08:32:00Z`; fixture
`fixtures/starter-corpus/tatyova-sourced.json`; no metadata invented.

## Genuine RED

1. Initial behavioral RED (before the cards existed) — all three landfall/
   defender tests failed at the catalog lookup (`Tatyova/Jaddi must be
   supported`). After adding compile/scaffold entries but before mechanics:
   * `jaddi_landfall_gains_life_and_defender_blocks_attacks`:
     `defender cannot attack` (legal attackers still included Jaddi).
   * `tatyova_landfall_fires_only_for_its_controller_and_gains_then_draws`:
     life `40` vs expected `41` (no landfall trigger).
2. CLI help test for Tatyova failed before the help edit (`help omits Tatyova,
   Benthic Druid`).

Fix: `TriggerEvent::LandEnters`; Tatyova/Jaddi triggered abilities;
`apply_play_land` queues landfall only for permanents the land controller
controls and stages them before priority; `Effect` order is GainLife then Draw.
`CardDef::defender`, `Object::has_defender`, `legal_attackers` exclusion, an
explicit forged-declaration refusal (`CR 702.3b`) and heuristic attacker filter.
The forged attack regression asserts revision unchanged and no attackers.

## Verification

```
cargo test -q -p aperture-core --test issue17_tatyova --test issue17_tokens \
  --test issue17_talrand --test card_printings --test catalog_pool \
  --test issue13_commander_combat -p aperture-import --test starter_corpus_planned
  -> 3, 10, 6, 4, 5, 43, 1 passed
cargo test -q -p aperture-cli
  -> 8 + 1 passed
cargo fmt --all --check
cargo clippy -q --all-targets -- -D warnings
cargo build --release -p aperture-cli
  -> exit 0 (slice3-gates.log, slice3-focused.log)
```

Four-seat exact package `decks/package-tatyova-landfall.txt` (40 supported
cards, clearly **not** the desired 100-card starter), Tatyova commander each
seat, seed 21, Commander-combat 40 life:

* terminal, P0 wins, 2993 steps / 65 turns, digest `b31adfd97c31…`
* 20 Tatyova landfall triggers and real Jaddi landfall triggers in the log;
  Tatyova resolution gains life then draws; Jaddi never attacks
* replay digest matches; `--save-out` save replays to the same digest
  (`slice3-replay.log`, `slice3-save-replay.log`)

Before the heuristic defender filter, the same journey failed because the AI
forged an attack with Jaddi; that failure is preserved in the first game log.
The fixed run is terminal, not capped.

## Corpus refresh

Pool 86 → **88**. Manifest `missing_cards` recomputed:
Tatyova 56, Krenko 52, Talrand 54, Isamaru 57, Ayula 52, Elas 59, Zada 58.
Exact all-name totals over the eight desired lists, including commanders and
basics (coordinator evidence `slice3-native-corpus.py/json`): union **459**
unique names, **76 supported**, **383 missing**, catalog **88**. The manifest
`missing_cards` matches those verified contents. All eight exact desired list
files are unchanged; Tatyova remains `planned`.

## Repair pass (catalog split + accounting)

* RED first: a regression compared `catalog::all()` and `catalog::def()` for
  Jaddi Offshoot and failed `false != true`. The duplicate special-case JADDI
  static was removed; Jaddi now has one canonical `card_defender!` entry used by
  every catalog consumer. `catalog_def_and_catalog_all_agree_on_jaddi_defender`
  and the defender behavior test pass.
* The forged defender attack now asserts canonical state-digest equality, not
  just revision equality.
* New control test: land played → landfall trigger on stack → Tatyova returned
  to hand by real Unsummon → trigger still gains 1 life and draws; no `Priority`
  event between life, draw and `AbilityResolved`.
* Jaddi source role is now `landfall_package` (not a token producer).
* Tatyova tests: **5 passed** (catalog consistency, sourced text, controller-only
  landfall/order, source-left trigger, defender/forged refusal).
* Existing exact package journey/replay reused; Tatyova replay still verifies
  digest `b31adfd97c31…`.

## Superseded accounting note

An earlier revision of this handoff reported 447 nonbasic names / 67 supported /
380 missing. That was a nonbasic-only figure and is superseded by the exact
459/76/383 all-name totals above; the docs subsection now uses only the proven
totals.

## Changed files

`crates/core/src/{card,catalog,ability,engine,state}.rs`,
`crates/core/tests/{issue17_tatyova,card_printings,issue13_commander_combat}.rs`,
`crates/ai/src/heuristic.rs`, `crates/cli/src/main.rs`,
`decks/package-tatyova-landfall.txt`,
`fixtures/starter-corpus/tatyova-sourced.json`,
`decks/starter-corpus/manifest.json`,
`docs/issue17-starter-corpus.md`, `README.md`, plus evidence
`slice3-*` logs and this handoff.

## Gaps / next command

* Tatyova 100-card list remains planned (56 missing); no complete Tatyova
  starter claim.
* Landfall only for these two cards; other landfall payoffs, extra land drops
  and broader ramp remain unsupported.
* Defender is one typed static; no other defender/attack-restriction card.
* Legend rule still absent; Tatyova commander-only.
* Next focused command:
  `cargo test -q -p aperture-core --test issue17_tatyova --test issue17_tokens --test issue17_talrand`
