# Slice 66 — Pashalik Mons + Goblin Trashmaster, and R365-1 (cost triggers resolve above the ability)

Branch `aperture/issue-17`, draft PR 46. Opening record commit **`849466f`**
(R365-2/3/4), product + repair commit **`edfc363`** (this is the code head CI is
bound to). `origin/master` untouched at `73820d7`; the branch contains master.

## What changed

**R365-1 (functional, from review 365).** `apply_activate_ability` paid cost
zone changes (the chosen-Goblin sacrifice, self-sacrifice, discard, graveyard
exile) but never staged the resulting triggers, so a sacrificed Goblin's dies
trigger resolved **below** the ability it paid for — after the damage, with no
response window (CR 603.3; 602.2b applies 601.2h–i). The function now calls
`put_triggers` immediately after the ability's `StackItem` is pushed, exactly
where the cast path (`engine.rs:5221`) and the land-play path (`engine.rs:4692`)
already do.

*Sweep of the other cost-payment sites* (asked for explicitly): every cost
payment for an activated ability lives in that one function — `discard_as_cost`,
`sacrifice_creature_subtype`, `exile_graveyard_cards`, `sacrifice_source`,
`exile_source_from_graveyard` — and all of them now drain the queue through the
single call after the push, so no shape can be left behind by construction. The
cast path and the land-play path were already staging at the same point; the
nonland mana-ability path has no zone-changing cost in this pool (land mana
abilities only tap). The only cost shape in the pool that can kill a *creature*
(and therefore trigger anything) is the chosen-subtype sacrifice, which is what
the two new regressions exercise.

**Pashalik Mons (DMR 133, `{2}{R}`, legendary 2/2 Goblin Warrior).** Printed text
fetched live on 2026-10-29 (`fixtures/starter-corpus/slice66-sourced.json`, 5
printings, **one** distinct oracle text): "Whenever Pashalik Mons or another
Goblin you control dies, Pashalik Mons deals 1 damage to any target. /
`{3}{R}`, Sacrifice a Goblin: Create two 1/1 red Goblin creature tokens."
New shared primitive: `TriggerEvent::GoblinYouControlDies`, a *subtype-narrowed*
dies watcher. The dying permanent's effective Goblin subtype is captured on the
death record before the zone change (CR 613.1d + 700.4), so a layer-4 animation
counts and a Bear never does; the watcher includes the source itself and is
restricted to Goblins its controller controls.

**Goblin Trashmaster (M19 144, `{2}{R}{R}`, 3/3 Goblin Warrior).** Same fetch (6
printings, one text): "Other Goblins you control get +1/+1. / Sacrifice a
Goblin: Destroy target artifact." The anthem is the printed layer 7c static
("other", so it never pumps itself). The activated ability adds two primitives:
`TargetSpec::Artifact` (exactly one artifact, either controller, never an
enchantment and never the aggregate "any number" set) and
`Effect::DestroyTargetArtifact`. CR 608.2b target legality now reads the
ability's own printed target spec instead of assuming the creature default, so a
target that stopped being an artifact makes the ability fizzle rather than
resolve against a legal-looking object.

**Corpus.** 179 catalog / 459 union = **165 supported + 294 missing**, per-list
`[55, 0, 51, 36, 0, 52, 52, 53]`, Krenko 38→36, still **2 of 8 certified**
(`evidence/issue17/slice66/corpus-audit.py` → `AUDIT OK`). Krenko's
`missing_cards` **and** `missing_interactions` were rewritten whole
(`evidence/issue17/slice66/update-manifest.py`).

**Records (R365-2/3/4, opening commit).** The slice-65 event count is corrected
to 5571, the runnable journey command is given with `--out`/`--replay-out`
(annotated as errata; the historical checkpoint body is not rewritten), and
`decks/package-slice65.txt`'s header now describes the deck it actually contains
and drops the false cast claim.

## Gates run in this slice

| gate | result |
|---|---|
| `cargo fmt --all --check` | exit 0 (`slice66/fmt.log`) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 (`slice66/clippy.log`) |
| `cargo test --workspace --all-targets --locked` | **157 result lines / 1162 passed / 0 failed / 2 ignored, exit 0** (`slice66/workspace-all-targets.log`); slice 65 was 154 / 1153, and the delta is exactly +2 ordering regressions, +5 card tests, +1 journey, +1 printing conformance = +9 |
| R365-1 RED before the fix | both new order tests failed on the unfixed engine: the stack held only `Activated { ability: 1 }` and no trigger was ever staged (0 passed / 2 failed, exit 101) |
| four-seat in-engine journey | `issue17_slice66_journey`: 4 seats, 40 life, **129 events**, digest `7b1df80ec3c2af52b0b4bf4dbb04d88a9fad0494662f32ce477d2dff5ffe5865`; casts both cards, the two-token sacrifice, the ping above the ability (event-order assertion), the artifact destruction, and combat |
| four-seat CLI journey (separate process) | `selfplay --seats 4 --seed 2626 --commander-combat-practice` on `decks/package-slice66.txt`, report `slice66/journey.json`: terminal `Win{P2}`, **1373 steps / 30 turns / 3733 events**, digest `70d147dbdac0`; separate `aperture-cli replay --file …/journey-replay.json` → `matches recorded`, exit 0 |

**What the CLI game actually did with the new cards** (verbose replay,
`slice66/journey-verbose.log`): 8 casts of Pashalik Mons, 4 casts of Goblin
Trashmaster, and **22** "Pashalik Mons deals 1 damage" resolutions — the new dies
watcher firing in a real four-seat game. The heuristic policy never *activated*
either new ability (all 12 activations in that game are Krenko, Mob Boss), and no
artifact reached the battlefield, so the artifact destruction is exercised by the
in-engine journey, not by this game. That is disclosed rather than glossed.

**Not claimed:** the CLI journey is a 40-card demonstrator, not 100-card
certification, not product-AI strength, not GUI evidence.

## CI, stated exactly

* Code head **`edfc363`**: `gh api …/commits/edfc363/check-runs` returns **10
  runs, every `head_sha` = `edfc363`, 10 completed `success`, 0 failed, 0
  cancelled** — push run `40887559100` and `pull_request` merge-ref run
  `40887573023`, both `attempt=1` (`slice66/exact-head-ci-edfc363.json`). The
  evidence commit was pushed only after both workflows had finished.
* The evidence tip is a *later* commit and starts its own runs. Those runs are
  recorded in the handoff when they finish; the tip's checks are not described as
  green before they complete. Product files are unchanged between the code head
  and the tip (only `evidence/` paths differ).

## Honest scope

* Original issue #17 remains **unfinished**: **2 of 8** lists certified, six
  planned lists with real gaps, Krenko 36 names still missing.
* `apply_activate_ability` staging is a real behaviour change in accepted code:
  a replay recorded on an earlier head in which a sacrifice-cost dies trigger
  resolved *after* the ability will now diverge (no broad backward-continuity
  claim). No `GameState` field was added, so plain saves and replays that do not
  contain such a sequence are unaffected.
* Two members of the assigned sacrifice-a-Goblin family are still missing and are
  listed as missing, not approximated: **Skirk Prospector** (`Sacrifice a Goblin:
  Add {R}` is a *mana* ability, CR 605.1a — it must not use the stack, which this
  pool's nonland mana-ability path does not yet model) and **Siege-Gang
  Lieutenant** (the Lieutenant ability word plus a haste grant to its own
  tokens).
* Native authored this increment and is not its independent reviewer.
* No completion, merge, master push, closure, dependent unlock, new team or
  global-config change. Children `t_26a7a4ff` / `t_3c2fe51a` stay gated and are
  not increment-review children.

## Next action

One independent, concise, practical delta review of `4e33fb0..edfc363` (code
head `edfc363`): the staging point (does every cost shape really funnel through
it, and is the trigger above the ability in both the stack and the event order)
and the two new primitives (subtype-narrowed dies dispatch including the source
itself and layer-4 subtypes; `target artifact` refusing enchantments and the
abolished "assume creature" legality default) are the places where a plausible
implementation passes its own test and is still wrong. Then the same writer,
history, card, branch and draft PR 46 continues Krenko's remaining 36 names.


## Slice 67 erratum (historical logs not rewritten)

The in-engine journey's opening prose claimed every action came from projected
legal offers with no mid-game state edits. Its combat section directly installs
DeclareAttackers/priority/pending and submits a handcrafted declaration. That
portion is a fixture-driven integration scenario with an injected combat
transition, not an all-offers game. The cast/activation assertions and historical
logs remain unchanged; `crates/core/tests/issue17_slice66_journey.rs` now labels
this truthfully.
