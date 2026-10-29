# Issue17 — slice 28: Circle of the Moon Druid, independently approved (original eight-list contract still unfinished)

## Current candidate

- Same native card `t_67778feb`, checkout `C:/AI/aperture-worktrees/issue-17`, branch `aperture/issue-17`, draft PR46.
- **Head of record `38dc519322a920ff4e067787a7c02d045bdf2354`** — the increment: 34 files, +33582/−35 (code, tests, fixture, manifest, docs, the slice-28 driver and its evidence). **Independently reviewed and APPROVED** by a fresh read-only reviewer (report `evidence/issue17/review283/REVIEW283.md`).
- **Tip `6f58f75e6282b44e81a54132cbff7437f19b072d`** — an evidence/doc-only follow-up (4 files: three regenerated mutation logs and one doc sentence) that closes the reviewer's one non-blocking record finding. No code, test or fixture byte changes: `git diff 38dc519..6f58f75 --stat` is exactly those four files.
- PR46 body updated with the slice-28 section and refreshed state bullets; the readback differs from the written body only by a trailing newline (`slice28/pr-body-before.md`, `slice28/pr-body-after.md`, `slice28/pr-body-readback.md`, script `slice28/update-pr-body.py`).
- Local == remote == PR `headRefOid` after push. PR OPEN / draft. No merge, no master push, no issue closure, no downstream activation, no new team, no global provider/model change.

## What this increment adds

`Circle of the Moon Druid` (`{2}{G}`, printed 2/4 Creature — Human Elf Druid; Scryfall **AFR 177**, immutable `700c9ed8-6bfe-4f1e-855a-5f31e0d2fc9b`, oracle `90569b99-8b63-4d2e-b7de-b578b8adbdcb`, retrieved **2026-10-27T12:07:11Z**; both printings — AFR 177 and HBG 204 — carry identical wording; recording `fixtures/starter-corpus/slice28-sourced.json`, raw responses in `evidence/issue17/slice27/ayula-missing-sources/`).

Printed text: "Bear Form — During your turn, this creature is a Bear with base power and toughness 4/2. (It loses all other creature types.)"

It is the pool's **first printed continuous ability conditioned on whose turn it is** (CR 611.3a), and one printed ability spans two layers:

1. **Layer 4** (CR 613.1d): `StaticAbility::SelfBecomesSubtypeDuringYourTurn` + `SubtypeLine::Bear`. `Game::has_subtype` answers the printed line first and then any layer 4 effect in timestamp order, so the Druid *replaces* its printed "Human Elf Druid" line: Little Bear's "if that creature is a Bear" counter branch, Ayula's `TargetSpec::Bear` / `BearYouControl` offer lists and Exuberant Wolfbear's "target Human you control" all read the effective subtype (the Druid stops being a legal Human target on its controller's turn).
2. **Layer 7b** (CR 613.4b): `StaticAbility::SelfBasePowerToughnessDuringYourTurn`, applied by `recompute_layers` before the timestamp-ordered one-shot setters and before layer 7c, so the printed 4/2 is a *base* that pumps and +1/+1 counters sit on top of (Giant Growth on your turn gives 7/5, a counter gives 5/3).

Neither half creates a `ContinuousEffect`: the condition is answered when the characteristic is read, so nothing has to expire in cleanup and there is no effect object that a bounce, a re-entry or a save/reload could orphan or reuse. `enter_battlefield` now recomputes characteristics when the entering card prints a layer 4/7b static, because a base-setting value cannot be reconstructed additively later. Additive printed 7c statics (`Wight of Precinct Six`) keep their existing recompute points — that pre-existing staleness window is disclosed in the doc rather than silently changed.

## Verification actually run (native, this slice)

- `cargo fmt --all --check` **pass**, `cargo clippy --workspace --all-targets --locked -- -D warnings` **pass**, `cargo build --workspace --release --locked` **pass** (`slice28/{fmt,clippy,release-build}.log`).
- Focused: **172 passed / 0 failed** across 18 suites including `issue17_slice28` (5 tests), slices 19–26, `issue17_slice15_corpus`, `issue13_commander_combat`, `rules_conformance`, `catalog_pool`, `card_printings`, `abilities`, `timing_mana`, `sba_cleanup`, `replay_digest` (`slice28/focused.log`).
- Full workspace: **114 suites / 803 passed / 0 failed** (`slice28/workspace.log`, `slice28/gates-summary.log` shows `overall exit=0`).
- **Three discriminating mutation REDs** (`slice28/run-mutations.py`, logs `red-mutation-*.log`): turn condition ignored → exactly the two turn tests fail; layer 7b half ignored → the base-P/T tests fail; layer 4 half ignored → the subtype-dependent tests fail. `engine.rs` is restored **byte-for-byte** in every case (sha256 `7439a45c…` before and after), and the script refuses to run if the anchor is not unique.
- **Real four-seat/40-life journey** from `evidence/issue17/slice28-journey-driver` on `decks/package-slice28.txt` with exact snapshots (Jasmine Boreal injected for all four seats by the demonstrator): terminal **`Win{P3}`**, seed 2828, **2065 actions / 50 turns / 5251 replay events**, digest **`48bf3ae58ac38bfb49d193dcb29772ba96cd870c8fe75373727083d86abafaf3`**, 9 Circle of the Moon Druid casts, `demo-journey.json` / `demo-save.json` / `demo-replay.json` regenerating **byte-identically** from the committed bytes. The driver asserts the printed **4/2-or-2/4 invariant after every accepted action**: 636 on-turn and 1404 off-turn sightings.
- **Separate replay**: `./target/release/aperture-cli replay --file evidence/issue17/slice28/demo-replay.json` → `replayed digest 48bf3ae58ac3 | matches recorded | events 5251`.
- **Independent corpus audit** (`slice28/corpus-audit.py`, recomputed from `catalog.rs` + the deck files with manifest equality assertions): **144 catalog / 459 union = 132 supported + 327 missing**, per list `55/0/51/48/21/52/52/53`, Ayula **21** short, **1 of 8 certified** (`slice28/corpus-audit.log`, ends `AUDIT OK`).
- Live source check: Scryfall `afr/177` re-fetched at 12:07:11Z (2 printings, one distinct oracle text); catalog text and fixture match it verbatim.

## Independent review

A fresh read-only reviewer (not the saved OMP histories; no implementation edits, no commits, no push) re-adjudicated `27d8f48..38dc519` by execution: **APPROVED, no blocking finding** (`evidence/issue17/review283/REVIEW283.md`). It reproduced the workspace totals (114 suites / 803 passed), the focused set (172), the corpus audit (144 / 459 = 132 + 327, Ayula 21), the journey digest and byte-identical artifacts, and ran its own probe crate (14/14) covering: own-turn 4/2 + Bear/not-Human, opponent-turn 2/4 + Human, two Druids on different seats, Giant Growth on an opponent's turn = 5/7, base restored after the pump expires, Shock killing the 4/2 form but not the 2/4 form, Unsummon+recast, and serialize/deserialize then turn-change. It also re-ran three of this slice's mutations plus two of its own and confirmed `engine.rs` was restored byte-for-byte.

Non-blocking notes it recorded: (1) the checked-in mutation logs recorded a pre-`cargo fmt` sha256 and did not bind to the reviewed bytes — **fixed in `6f58f75` by regenerating them at head** (now `7439a45c…`); (2) the printed layer 7b setter is applied before all one-shot setters regardless of timestamp — **now disclosed in the doc**, unobservable with this pool; (3) the new `enter_battlefield` recompute is a new global recompute point, behaviour-neutral for every existing digest; (4) two `StaticAbility` entries share one printed text (one per layer), which the slice-28 test pins.

## CI status — read this precisely

- **Exact head `6f58f75`: 10/10 check runs completed SUCCESS, every `head_sha` = `6f58f75`** — including the **pull_request-triggered** run 40506451741 and the push-triggered run 40506442341 (`slice28/exact-head-ci-summary-6f58f75.json`, captured by `slice28/capture-ci.py`, which exits non-zero unless every check run is completed, successful and keyed to that head; raw payloads `exact-head-ci-6f58f75.json` / `exact-head-runs-6f58f75.json`).
- **Push-trigger run for `38dc519` (the reviewed code head) was also SUCCESS 5/5 keyed to `38dc519`**, so every reviewed code byte has its own green run.
- The pull_request run for `38dc519` failed on the three rust jobs with E0308 at `crates/ai/tests/review.rs:60` — a file that does **not** exist on this branch (`git cat-file -e 38dc519:crates/ai/tests/review.rs` fails) and comes from `origin/master` `51dffda` (#28/PR56). That was the `hotspot: crates/core/src/mana.rs` master-drift item; the supervisor fixed it on master in `73820d7` (PR57), and the merge ref for `6f58f75` now compiles, which is why the pull_request run above is green. Nothing in this slice edited `crates/ai` or `crates/ui`; the green merge ref is therefore a property of master, not of this delta.

## Worktree state, stated precisely

Tracked tree is clean at `6f58f75`. Four tracked evidence files still carry uncommitted **slice-23** appends that predate this slice and touch no code or driver (`SLICE23-HUMAN-CHECKPOINT.md`, two append-only session `.jsonl` logs, `slice23/native-gates.log`); they are disclosed rather than folded into this head. Untracked: this checkpoint, `evidence/issue17/review283/`, and `evidence/issue17/slice28/{exact-head-ci-*,exact-head-runs-*,capture-ci.py}`, which are deliberately outside the branch tip so the tip keeps its own CI.

## Contract status (original #17 — unchanged, unfinished)

1 of 8 desired 100-card starters certified (Jasmine only). 144-card catalog; 459-name union = 132 supported + 327 missing; **Ayula 21 short**; per list `55/0/51/48/21/52/52/53`. Desired snapshots unchanged, no bundled replacements, arbitrary Archidekt compatibility still an explicit non-goal. This slice certifies **no** list.

## Next action

Continuing on the same card / PR46 / branch / saved histories: implement the next Ayula names from their authoritative printed text with printing id and retrieval stamp recorded (the sourced work order `evidence/issue17/slice27/AYULA-NEXT-PACKAGE.md` ranks `Stuffed Bear` next, which needs the additive layer 4 type change and the layer 5 colour set; `Drover Grizzly` needs saddle), toward genuine certification of one whole 100-card starter, with the `mana::add` merge-ref conflict now resolved on master by the supervisor (73820d7). Do not complete #17, merge, push master, close the issue, or unlock `t_26a7a4ff` / `t_3c2fe51a` from this worker.
