# Review 284 — slice 28 (Circle of the Moon Druid), contract lens, round 5

- Task: `t_67778feb` — #17 Certify the v1 Commander starter corpus end to end
- Branch: `aperture/issue-17` · draft PR46 · head of record `6f58f75e6282b44e81a54132cbff7437f19b072d`
  (reviewed code head `38dc519322a920ff4e067787a7c02d045bdf2354`)
- Delta under review: `27d8f48..6f58f75` (previous approved head → tip)
- Lens: **contract** (round 5 = four prior `changes_requested` returns + 1). Re-read the
  ORIGINAL #17 body and the four acceptance criteria, audit the deliverable strictly against
  them, and re-test every item a prior round asked for.
- Reviewer made **no implementation edits**. Additions this round are untracked only:
  `evidence/issue17/review284/{REVIEW284.md,audit-284.py,audit-284.json}` plus out-of-tree
  scratch at `C:/AI/r284a` (a `git archive` of the record bytes, removed from the record).

**Verdict: APPROVED_INCREMENT — no blocking defect in the slice-28 delta. The ORIGINAL #17
contract is still unfinished (1 of 8 lists certified, 327 of 459 union names unsupported), so
this card returns to sourced implementation, not to completion.**

---

## 1. What the delta actually contains

`git diff --stat 27d8f48..6f58f75` limited to non-evidence paths:

| file | change |
|---|---|
| `crates/core/src/{ability,card,catalog,engine,layer,state}.rs` | new printed layer 4/7b static, `SubtypeLine::Bear`, 144th catalog card, turn-begin + enter-battlefield recompute, one `Object` comment |
| `crates/core/tests/issue17_slice28.rs` | +356 lines, 5 tests |
| `crates/core/tests/{card_printings,issue13_commander_combat,issue17_slice15_corpus}.rs` | count updates (144 pool, Ayula 21) and the slice-28 fixture chained into the printing check |
| `decks/package-slice28.txt`, `fixtures/starter-corpus/slice28-sourced.json` | new 40-card evidence package + sourced printing record |
| `decks/starter-corpus/manifest.json` | one line removed (`Circle of the Moon Druid` leaves Ayula's `missing_cards`) |
| `README.md`, `docs/issue17-starter-corpus.md` | pool count 143→144, layer/limits wording, slice-28 section |

- `38dc519..6f58f75` is exactly four files: three regenerated mutation logs and one doc
  sentence (`git diff --stat` verified). No code, test, fixture or manifest byte.
- `crates/ai` and `crates/ui`: **zero** changed lines in `27d8f48..6f58f75`.
- The eight desired lists under `decks/starter-corpus/planned/` are **unchanged** (0 diff
  lines), so no bundled replacement or snapshot edit was smuggled in.
- The three modified existing tests only move counters/assertions that must move when the pool
  grows; no assertion was deleted or weakened (diff read line by line).

## 2. Contract audit against the ORIGINAL #17 criteria

| # | Criterion | Status at `6f58f75` | Evidence |
|---|---|---|---|
| 1 | ≥8 explicitly supported 100-card lists | **OPEN — 1 of 8** (Jasmine only) | my own audit, §6 |
| 2 | every included card + interaction implemented with conformance coverage | **OPEN — 327 of 459 union names unsupported**; this slice closes 1 (`Circle of the Moon Druid`, Ayula 22→21) | my own audit + `issue17_slice28` tests |
| 3 | four-seat games on exact deck snapshots, never bundled replacements | **partially demonstrated only**: the slice-28 journey is an explicitly-labelled 40-card package (`decks/package-slice28.txt`), 4 seats / 40 life, exact snapshot digest `5d787cb0…`; it is NOT a desired 100-card starter | §5 |
| 4 | document per-card/interaction exclusions; arbitrary Archidekt remains a non-goal | **satisfied and extended**: slice-28 section in `docs/issue17-starter-corpus.md`, limits in `README.md`, non-goal sentence preserved | diff read |

No criterion was claimed met that is not met; the checkpoint, the doc section and the PR body all
state 1 of 8 and 327 missing.

## 3. Prior rounds re-tested (contract duty)

Review 280 / round 3 items — all still correct at `6f58f75`:
1. `crates/core/src/layer.rs:3-17` and `:28-42` state the four implemented layers
   (`Type` 4, `Ability` 6, `Setting` 7b, `Modifying` 7c) and scope the gap to 5, 7a, 7d. ✔
2. `docs/issue17-starter-corpus.md` no longer calls 7b a missing colour layer; colour change is
   attributed to layer 5. ✔
3. The shipped slice-24 driver reproduces from the record's own bytes — I re-ran that driver's
   *slice-28* sibling out of tree (§5); the slice-24 driver is untouched by this delta. ✔
4. `crates/core/src/card.rs:203-207` now says the Little Bear counter reads the **effective**
   subtype via `Game::has_subtype`, and `engine.rs` really calls it at the Bear branch. ✔

Review 282 / round 4 qualifications:
1. **"Do not claim required-check green until reconciled."** Now legitimate: the live
   `pull_request` run for `6f58f75` is green because master carries the supervisor's `73820d7`
   fix. I confirmed 10/10 check runs SUCCESS keyed to `6f58f75` via the API this session (§7). ✔
2. **`docs/…:192` printed-vs-granted haste** — landed exactly as requested ("Haste is
   implemented as a *granted* keyword … cards that print haste are not in the pool"). ✔
3. Slice-25 driver header comment still describes Surrak — **still stale**, still cosmetic, still
   not a hold (that file is not touched by this slice). See finding N3.

## 4. Engine change read cold

- Layer 4: `Game::has_subtype` answers the printed `SelfBecomesSubtypeDuringYourTurn` line first,
  then any layer 4 effect in timestamp order (last wins), else the printed type line. Every
  subtype question in the engine goes through it — I grepped: the only direct printed reads left
  are `CardDef::has_subtype` for Goblin tokens (`card.rs:480`) and `type_line.contains("Planeswalker")`
  (`engine.rs:6579`), neither reachable by a type-changing card in this pool.
- Layer 7b: `recompute_layers` applies the printed 4/2 base first, then timestamp-ordered
  one-shot setters, then printed 7c statics and 7c modifiers; counters are added on top by
  `Object::power/toughness`. `enter_battlefield` recomputes when the entering card prints a
  layer 4/7b static, and turn begin recomputes so the condition flips with the active player.
- Correct per CR 611.3a/613.1d/613.4b as printed; the "printed setter is ordered before all
  one-shot setters regardless of timestamp" simplification is disclosed in the doc and is
  unobservable in this pool (one setter card).
- Tests assert behaviour, not source text: subtype via offer lists and a real counter branch, base
  4/2 under a real Giant Growth (7/5), under a real +1/+1 counter (5/3), turn flip and refusal,
  three seats, plus a **negative control** (a Human Soldier gets no counter) that keeps the Bear
  branch discriminating.
- Mutation evidence is bound to the current bytes: all three RED logs record
  `sha256 before mutation = 7439a45c…`, identical to `sha256sum crates/core/src/engine.rs` today,
  `restored byte-for-byte: True`, and each mutation fails a different subset (the control test
  passes under every mutation). `run-mutations.py` refuses to run unless the anchor is unique.

## 5. Execution — every claim reproduced

Run from a `git archive` of `6f58f75` into `C:/AI/r284a`, path deps made absolute, its own
`CARGO_TARGET_DIR`, driver `src/main.rs` blob hash `2f438351…` == record:

| check | result |
|---|---|
| `cargo run --release --locked` on the slice-28 driver | exit 0, terminal `Win{P3}`, seed 2828, **2065 actions / 50 turns / 5251 replay events**, digest `48bf3ae58ac38bfb49d193dcb29772ba96cd870c8fe75373727083d86abafaf3`, snapshot `5d787cb0…`, 9 Druid casts, 636 on-turn / 1404 off-turn invariant sightings |
| regenerated `demo-journey.json` / `demo-save.json` / `demo-replay.json` vs committed | **byte-identical** (`065a6bcb…`, `df9487ea…`, `1a6aae9c…`) |
| separate `aperture-cli replay --file` on my own generated replay | `replayed digest 48bf3ae58ac3 | matches recorded | events 5251` |
| legacy issue-17 replays at this head | slice 19 `5fd841b2`, slice 20 `72cd69fb`, slice 22 `17f03348`, slice 24 `a41bbfd2`, slice 25 `4d6ea58e`, slice 26 `86bbd60b` — all match recorded |
| `evidence/issue14-abilities-replay.json` | mismatch `2078001e…` vs `ebd5340f…` — **pre-existing and documented** in `evidence/issue17/legacy-replay-note.md` (already stale at the issue-17 base `8a72cc6`, verified there with a base build). Not a slice-28 regression. |
| `cargo fmt --all --check` | exit 0 |
| `cargo test --workspace --locked` | **114 suites / 803 passed / 0 failed** |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| focused targets at head (`issue17_slice28`, `issue17_slice15_corpus`, `card_printings`) | 8 + 2 + 5 passed, 0 failed |

The workspace totals reproduce the handoff exactly (114/803/0), which is the number a stale
handoff most often gets wrong.

## 6. Independent corpus audit

My own script (`audit-284.py`) parses the `card!` family in `crates/core/src/catalog.rs`, the
eight desired deck files plus each list's commander, and cross-checks the manifest's
`missing_cards` arrays by set equality per list. It also refuses to print a pass unless the
manifest agrees.

```
catalog_supported 144 · union_names 459 · union_supported 132 · union_missing 327
per_list_missing [55, 0, 51, 48, 21, 52, 52, 53] · ayula_missing 21
certified_lists 1 (jasmine-vanilla-stompy) · manifest_agrees true · problems []
```

Identical to the delta's `slice28/corpus-audit.log`, to the doc's prose counts and to the
checkpoint. (Note for future audits: the manifest counts each list's **commander** in the
referenced set while the deck files name it only in a header comment, so a deck-file-only
computation is 1 short on the two lists whose commander is unsupported — Elas and Zada. That is
a property of the data layout, not an error in the delta.)

## 7. Live source and live CI

- Scryfall `afr/177` and `hbg/204` re-fetched this session: `Circle of the Moon Druid`,
  `{2}{G}`, `Creature — Human Elf Druid`, 2/4, oracle text identical to the catalog string and the
  fixture, immutable id `700c9ed8-6bfe-4f1e-855a-5f31e0d2fc9b`; both printings carry one distinct
  text. Fixture carries `retrieved_at 2026-10-27T12:07:11Z`.
- `gh api …/commits/6f58f75…/check-runs`: **10 check runs, all `success`, all keyed to
  `6f58f75`** (two runs × rust ubuntu/macos/windows, board+roadmap, windows package). PR46 is
  OPEN / draft / MERGEABLE / BEHIND master with `headRefOid` = `6f58f75`.
- `38dc519`: 5 success (push workflow) and 3 rust failures (pull_request workflow, the disclosed
  master-drift E0308 in `crates/ai/tests/review.rs`). The checkpoint describes exactly this
  split; no inherited green is claimed for the tip.

## 8. Findings

**Blocking: none.** Nothing in the delta misstates a capability, weakens an assertion, changes a
desired snapshot, or edits `crates/ai`/`crates/ui`.

**N1 (non-blocking record accuracy, fix when that file is next touched).**
`docs/issue17-starter-corpus.md:462-466` says "layer 5 colour changes and the animation effects
that turn a non-creature permanent into a creature are still missing, which is why `Stuffed Bear`
and `Beorn's Hospitality` remain unsupported." For `Beorn's Hospitality` the third blocker is
**layer 7a** ("power and toughness are each equal to the number of lands you control" is a
characteristic-defining ability), which this same delta's own work order states correctly
(`evidence/issue17/slice27/AYULA-NEXT-PACKAGE.md:23`: "Needs CDAs (7a), deliberately absent"), and
which `layer.rs` still lists as unimplemented. Colour is not a blocker for that card at all.
Suggested replacement: "…are still missing — and `Beorn's Hospitality` additionally needs a
characteristic-defining P/T (layer 7a), which stays deliberately absent — which is why `Stuffed
Bear` and `Beorn's Hospitality` remain unsupported."

**N2 (non-blocking, record hygiene).** The checkpoint's "Untracked:" list names this checkpoint,
`review283/` and three `slice28/` evidence files, but the worktree also carries older untracked
review directories (review255/275/278/280/282 and their journey copies) and, notably,
`evidence/issue17/slice28-journey-driver/target/` — a build directory left inside a committed
evidence folder. Tracked tree state is described correctly (four pre-existing slice-23 evidence
modifications); only the untracked enumeration is short. Consider a `.gitignore` entry for
`evidence/**/target/`.

**N3 (non-blocking, carried since round 2).** `evidence/issue17/slice25-journey-driver/src/main.rs:4`
still describes casting Surrak as the driver's preference. Not touched by this slice.

**Strategic note for the supervisor (not a defect, not a hold).** Criterion 1 is the binding
gate and it has moved by one card this slice: Ayula is 21 names short, and the other six lists
are 51–55 names short. At one-to-two sourced cards per slice, the eight-list gate is hundreds of
cards away; the board should decide deliberately whether to (a) drive Ayula to zero missing with
bulk sourced packages (the modular fanout authorization names three reserved isolated workers for
exactly this), or (b) restate the near-term target. Continuing to add one card per reviewed slice
is honest but does not converge on #17 inside any near horizon.

## 9. Verdict

`APPROVED_INCREMENT` at `6f58f75` (code `38dc519`): the increment is correctly implemented,
reproducible from the record bytes, honestly documented and green at its own exact head. Because
the ORIGINAL #17 contract is unfinished (1 of 8 lists certified, 327 names unsupported), the card
returns to implementation on the **same card / PR46 / branch / saved histories**. No completion,
no merge, no master push, no issue closure, no downstream unlock — `t_26a7a4ff` (scheduled) and
`t_3c2fe51a` (todo) stay dependency-blocked on the certified corpus, verified untouched this
round. Next sourced target per the maintained work order: `Stuffed Bear` (additive layer 4 type
change + layer 5 colour set) — and N1's one-clause doc correction can ride along in that slice.
