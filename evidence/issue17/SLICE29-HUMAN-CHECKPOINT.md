# Issue17 — slice 29: Stuffed Bear (original eight-list contract still unfinished)

## Current candidate

- Same native card `t_67778feb`, checkout `C:/AI/aperture-worktrees/issue-17`, branch `aperture/issue-17`, draft PR46.
- **Code head of record `99785d783f991918b541fef7793a3e6a1bb48e75`** — 47 files: the engine/ability/catalog work, nine focused tests, the fixture and manifest/doc updates, the slice-29 journey driver with `decks/package-slice29.txt`, and the slice-29 evidence (gates, mutation script + logs, corpus audit, journey artifacts, raw Scryfall sources).
- **Tip `de3ba07c0a89360a4f86422417790ddf6677a328`** — one evidence/tooling file (`slice29/update-pr-body.py`): the committed script could not reproduce the recorded body because `str.format` choked on the printed mana cost `{2}`; it now substitutes explicit markers and takes the head token as an optional argument, and re-running it reproduces `pr-body-readback.md` byte-for-byte. No code, test, fixture or doc byte changes.
- PR46 body updated with the slice-29 section and refreshed state bullets; the readback equals the written body (`slice29/pr-body-before.md`, `pr-body-after.md`, `pr-body-readback.md`, script `slice29/update-pr-body.py`).
- Local == remote == PR `headRefOid`. PR OPEN / draft. No merge, no master push, no issue closure, no downstream activation, no new team, no global provider/model change.

## What this increment adds

`Stuffed Bear` (`{2}` Artifact; Scryfall **MID 259**, immutable `4cd40276-016b-475f-906f-74d31dceaf70`, oracle `1402c116-194e-49ca-a8c1-09ca659c220c`, retrieved **2026-10-27T13:14:48Z**; MID 259 and DBL 259 carry identical wording; raw responses in `evidence/issue17/slice29/sources/`, fixture `fixtures/starter-corpus/slice29-sourced.json`).

Printed text: "{2}: This artifact becomes a 4/4 green Bear artifact creature until end of turn."

It is the pool's first permanent that is **not a creature until an ability animates it**, and one printed ability spans three layers:

1. **Layer 4** (CR 613.1d) — `ContinuousKind::AddCreatureSubtypes` *adds* the creature card type and the Bear subtype to the printed Artifact type, where the existing `SetCreatureSubtypes` replaces a creature's subtype line.
2. **Layer 5** (CR 613.1e) — `ContinuousKind::SetColor(Green)`; the pool's first colour-changing effect.
3. **Layer 7b** (CR 613.4b) — `SetBasePowerToughness { 4, 4 }`, applied before the one-shot setters and before layer 7c.

All three are ordinary until-end-of-turn `ContinuousEffect`s, so cleanup ends them (CR 514.2) and departure prunes them (CR 400.7); nothing bespoke expires, and no effect object can be orphaned by bounce or reload.

The rules layer now asks two layered queries instead of reading printed characteristics:

- `Game::is_creature` (printed type **or** a layer 4 addition). Every engine site that used `Object::is_creature` now calls it — target offers, attack and block legality, combat damage, state-based actions, regeneration, fight revalidation, combat pruning, the coach and the view. `Object::is_creature` remains the printed baseline and says so.
- `Game::has_color` (layer 5 set, else printed colour through the new `CardDef::is_color`/`is_black`). The Spectral Bears intervening-if asks it.

Two integration consequences that are easy to miss and are tested:

- **Death.** `move_to_graveyard_inner` now captures whether the permanent *was* a creature at the moment it left, and the death record carries that answer through `queue_self_death` and `queue_watchers_for_deaths`. An animated Stuffed Bear that dies is therefore a creature death: Zulaport Cutthroat's drain fires in the test.
- **Summoning sickness.** A permanent that prints an ability which can animate itself records CR 302.6 sickness on arrival (`ability::can_become_creature`), and the untap step clears sickness for every permanent. An artifact animated on the turn it entered cannot attack; animated on a later turn it attacks for four.

## Verification actually run (native, this slice)

- Genuine RED first: `evidence/issue17/slice29/red-unsupported.log` — `catalog::by_name("Stuffed Bear")` was `None` and the deck parser refused `1 Stuffed Bear`, both captured before any engine edit.
- `cargo fmt --all --check` **pass** (`fmt.log`), `cargo clippy --workspace --all-targets --locked -- -D warnings` **pass** (`clippy.log`), `cargo build --workspace --release --locked` **pass** (`release-build.log`).
- Focused: **197 passed / 0 failed** across 22 suites (`focused.log`), including the nine slice-29 tests and slices 19–28, `issue17_slice15_corpus`, `issue13_commander_combat`, `rules_conformance`, `catalog_pool`, `card_printings`, `abilities`, `timing_mana`, `sba_cleanup`, `replay_digest`, `combat`, `issue15_layers`, `issue15_effects`.
- Full workspace: **115 suites / 812 passed / 0 failed**, exit 0 (`workspace.log`, `WORKSPACE_EXIT=0`).
- **Four discriminating mutation REDs** (`run-mutations.py`, logs `red-mutation-*.log`): layer 4 addition ignored → 6 slice-29 tests fail; layer 5 ignored → 4 fail; layer 7b ignored → 7 fail; entry sickness dropped → exactly the attack test fails. `engine.rs` is restored **byte-for-byte** in every case (sha256 identical before and after; the script refuses to run if an anchor is not unique).
- **Real four-seat/40-life journey** from `evidence/issue17/slice29-journey-driver` on `decks/package-slice29.txt` with exact snapshots (Jasmine Boreal injected for all four seats by the demonstrator): terminal **`Win{P3}`**, seed 2929, **1912 actions / 44 turns / 5311 replay events**, digest **`ac16e510ca4691a69981beafcb577f3aa8f01bd6973710f86b89275899030024`**, 4 Stuffed Bear casts, 119 activations, `demo-journey.json` / `demo-save.json` / `demo-replay.json` regenerating **byte-identically** (`journey-sha-before.txt` == `journey-sha-after.txt`). The driver asserts the animated-or-plain invariant after every accepted action: 1301 animated sightings (creature + green + artifact + 4/4) and 3741 plain sightings (noncreature + colourless).
- **Separate replay**: `./target/release/aperture-cli replay --file evidence/issue17/slice29/demo-replay.json` → `replayed digest ac16e510ca46 | matches recorded | events 5311` (`cli-replay.log`, `REPLAY_EXIT=0`).
- **Independent corpus audit** (`slice29/corpus-audit.py`, recomputed from `catalog.rs` + the deck files with manifest equality assertions): **145 catalog / 459 union = 133 supported + 326 missing**, per list `55/0/51/48/20/52/52/53`, Ayula **20** short, **1 of 8 certified** (`corpus-audit.log`, ends `AUDIT OK`).
- Live source check: Scryfall `mid/259` fetched at 13:14:48Z, both printings identical; catalog text and fixture match it verbatim.

## Known limits, disclosed

- Green is the only colour any supported effect sets, and no rule in the pool discriminates a *green* permanent from a colourless one. Layer 5's observable effect today is the query itself (and the animated permanent's colour in tests), not a rules outcome: the Spectral Bears clause reads the layered colour but cannot currently change its answer because of this card. Stated in the doc rather than implied.
- The animation adds only the Bear subtype line the pool prints; a future "becomes a <type> in addition" card adds a `SubtypeLine` variant instead of a free-form string.
- `Object::is_creature` is now explicitly the printed baseline. `crates/ai` still reads it directly (its search belief model), so an AI lane that ever evaluates an animated Stuffed Bear would see a noncreature; this lane does not edit `crates/ai`, and the card is not in any desired starter list.
- `state.battlefield_creatures` (used by one timing test) also remains printed-only.
- The journey is a 40-card package, not a 100-card starter certificate.
- Three tracked slice-23 evidence files still carry pre-existing uncommitted appends; they are disclosed, not folded into this head.
- Untracked by design: this checkpoint, `slice29/exact-head-ci-*`, `slice29/capture-ci.py`/`ci-*.log`, so the branch tip keeps its own CI.

## CI status — read this precisely

`slice29/capture-ci.py 99785d7` exits 0 and reports **10 check runs, all completed SUCCESS, every one keyed to `99785d783f991918b541fef7793a3e6a1bb48e75`** (`slice29/exact-head-ci-summary-99785d7.json`, raw payloads `exact-head-ci-99785d7.json` / `exact-head-runs-99785d7.json`, poll log `ci-watch.log`, capture log `ci-capture.log`). Both the push-triggered run (`40515845297`) and the pull_request-triggered run (`40515855378`) contributed those checks: `rust (ubuntu-latest)`, `rust (windows-latest)`, `rust (macos-latest)`, `windows package` and `board and roadmap`, each SUCCESS at this head.

One reporting wrinkle, stated so nobody inherits an assumption: at capture time the pull_request workflow *run object* `40515855378` still reported `status: in_progress` even though every one of its five jobs is `completed / success` (re-read with `gh api repos/.../actions/runs/40515855378/jobs`). The push run is `completed / success`. No check in either run failed, and no green is inherited from any earlier commit.

## Contract status (original #17 — unchanged, unfinished)

1 of 8 desired 100-card starters certified (Jasmine only). 145-card catalog; 459-name union = 133 supported + 326 missing; **Ayula 20 short**; per list `55/0/51/48/20/52/52/53`. Desired snapshots unchanged, no bundled replacements, arbitrary Archidekt compatibility still an explicit non-goal. This slice certifies **no** list.

## Next action

Continuing on the same card / PR46 / branch / saved histories: implement the next Ayula names from their authoritative printed text with printing id and retrieval stamp recorded. The sourced work order `evidence/issue17/slice27/AYULA-NEXT-PACKAGE.md` ranks `Drover Grizzly` (needs saddle) next for new machinery, and the additive type change this slice added unblocks `Beorn the Fierce`'s "becomes a Bear in addition" clause. Do not complete #17, merge, push master, close the issue, or unlock `t_26a7a4ff` / `t_3c2fe51a` from this worker.
