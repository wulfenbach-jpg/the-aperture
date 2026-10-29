# Issue17 — slice 27: review-280 record repair, independently approved (original eight-list contract still unfinished)

## Current candidate

- Same native card `t_67778feb`, checkout `C:/AI/aperture-worktrees/issue-17`, branch `aperture/issue-17`, draft PR46.
- **Head of record `27d8f483cf431626c0eb95f308e6e328cf8353c3`** (local == remote == PR `headRefOid`, read back after push). PR OPEN / draft. No merge, no master push, no issue closure, no downstream activation, no new team, no global provider/model change.
- Delta `80fff73..27d8f48`: 13 files, +321/−21 — three **comment-only** `crates/core/src` files, one doc, one evidence-driver assertion, seven `slice27/` evidence files. **No behavior change**: a filter of `git diff 80fff73..27d8f48 -- crates/` to non-comment lines is empty.

## What this increment does

Review 280 (round 2 of slice 26) found **no functional defect** and four record defects; this head corrects all four:

1. `crates/core/src/layer.rs` (module header and `Layer` enum doc) and `engine.rs::recompute_layers` said only layer 7c was implemented and called layers 4-6/7b unimplemented. They now state the four implemented layers — `Type` (4, Ursine Champion), `Ability` (6, Goreclaw), `Setting` (7b, Exuberant Wolfbear), `Modifying` (7c) — and scope the gap to layers 5 (colour change), 7a and 7d.
2. `docs/issue17-starter-corpus.md` no longer calls 7b a missing colour-change layer; colour change is layer 5 and the residual gap is stated as layer 5 plus non-creature animation.
3. `crates/core/src/card.rs` now says Little Bear's counter is decided by the **effective** subtype through `Game::has_subtype`, matching `engine.rs`'s `is_bear` branch.
4. `evidence/issue17/slice24-journey-driver/src/main.rs:137` asserted the pre-`c2d9b35` wording and panicked at the recorded head. The one-line correction is **committed**, so the slice-24 evidence is reproducible from the record's own bytes.

Also corrected the record's own wording: `80fff73` is described as comment-only source edits with no behavior change, not "evidence-only".

## Verification actually run (native, this slice)

- `cargo fmt --all --check` exit 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0.
- Focused suites (`issue17_slice26/25/24/23`, `abilities`, `timing_mana`, `catalog_pool`, `card_printings`, `issue17_slice15_corpus`, `issue13_commander_combat`, `rules_conformance`): **122 passed, 0 failed** (`slice27/native-gates.log`).
- `cargo test --workspace --locked`: **113 suites / 798 passed / 0 failed** (`slice27/workspace-gates.log`, `slice27/tally.log`).
- Slice-24 driver rebuilt and run **from the committed bytes**: exit 0, terminal `Win{P3}`, **1602 actions / 37 turns / 4127 events**, digest `a41bbfd2…`, 11 entry buffs / 11 haste grants, and `demo-journey.json` / `demo-save.json` / `demo-replay.json` regenerate **byte-identically** (`2d6ec0ed…`, `857a6ca7…`, `681a6337…`; `slice27/slice24-driver-head-run.log`, `slice27/slice24-artifact-hashes.txt`).
- Independent corpus audit recomputed from `catalog.rs` + the deck files with a manifest-equality assertion: **143 catalog / 459 union = 131 supported + 328 missing**, per list `55/0/51/48/22/52/52/53`, **1 of 8 certified** (`slice27/corpus-audit.py`, fails on any mismatch).

## Independent review

A fresh independent reviewer (read-only; not the saved OMP histories; no implementation edits) re-adjudicated `80fff73..27d8f48` by execution: **APPROVED_INCREMENT, no blocking finding**. Its own runs: fmt/clippy clean, focused slice suites 34 passed, workspace 113 suites / 798 passed / 0 failed, the driver rebuilt in its own target dir reproducing the digest with byte-identical artifacts, and its own corpus audit 143 / 459 = 131 + 328. Per-correction results: **1 PASS, 2 PASS, 3 PASS, 4 PASS**. Report `evidence/issue17/slice27/REVIEW27.md`.

Non-blocking notes it recorded: an imprecise "worktree is clean" claim (corrected in the PR body — see below), a stale crate name in `slice26-journey-driver/Cargo.toml`, and retained prior debt (has_subtype timestamp wording, `SubtypeLine::has` whitespace splitting, `once_per_turn` keyed on the object).

## CI status — read this precisely

- **Branch head `27d8f48`: push run 40493923124 SUCCESS 5/5**, all keyed to that SHA (`board and roadmap`, `rust (ubuntu-latest)`, `rust (windows-latest)`, `rust (macos-latest)`, `windows package`).
- **Merge ref for the same commit: pull_request run 40493931766 FAILURE** on all three rust jobs. Cause is **master drift, not this delta**: the PR run compiles the synthetic merge with `origin/master` `51dffda` ("#28 Move review … PR56"), whose new `crates/ai/tests/review.rs:60` passes a `u8` to this branch's `crates/core/src/mana.rs:86` `add(&mut self, ManaColor, amount: u32)`. The `u32` widening is branch-only commit `e300bdd` (approved slice-9 work) and does not exist on master, so the merged tree does not compile.
- Consequence: **PR46 cannot satisfy its required checks until the two lanes reconcile the mana API.** That reconciliation is an integration decision (merge order / API shape), not something this lane may fix by editing `crates/ai` tests or regressing an approved widening. Evidence: `slice27/ci-27d8f48.json`, `slice27/ci-capture.log`; flagged as `hotspot: crates/core/src/mana.rs` on the card.

## Worktree state, stated precisely

Four tracked evidence files from the slice-23 finalisation still carry uncommitted edits that predate this repair and touch no code or driver: `evidence/issue17/SLICE23-HUMAN-CHECKPOINT.md`, two append-only session `.jsonl` logs, and `evidence/issue17/slice23/native-gates.log`. They are disclosed rather than folded into `27d8f48`, so the reviewed, CI-green head does not absorb unrelated history appends. No file under `crates/` or in any evidence driver is modified at this head.

## Contract status (original #17 — unchanged, unfinished)

1 of 8 desired 100-card starters certified (Jasmine only). 143-card catalog; 459-name union = 131 supported + 328 missing; **Ayula 22 short**; per list `55/0/51/48/22/52/52/53`. Desired snapshots unchanged, no bundled replacements, arbitrary Archidekt compatibility still an explicit non-goal. This slice certifies **no** list and changes no card support.

## Next action

Continuing on the same card / PR46 / branch / saved histories: implement the next missing Ayula cards from their authoritative printed text with printing id and retrieval stamp, toward genuine certification of one whole 100-card starter, while the supervisor resolves the `mana::add` merge conflict that currently makes the PR merge-ref uncompilable. Do not complete #17, merge, push master, close the issue, or unlock `t_26a7a4ff` / `t_3c2fe51a` from this worker.
