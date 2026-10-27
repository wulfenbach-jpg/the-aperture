# Review 27 — slice-27 record repair, independent re-adjudication

Reviewed head: **`27d8f483cf431626c0eb95f308e6e328cf8353c3`** (branch `aperture/issue-17`,
draft PR46, base `master`). Delta under review: `80fff73..27d8f48` (one commit,
"docs(issue17): align layer documentation with the four implemented layers").
Reviewer run 27, read-only on implementation files; all artifacts written under
`evidence/issue17/slice27/` and my own scratch directory.

## Verdict

**APPROVED_INCREMENT — no blocking finding.** All four record defects raised by
review 280 are corrected at the exact head, the correction is comment/doc/evidence
only (verified line by line: no non-comment change anywhere in `crates/`), and the
committed slice-24 driver now builds and reproduces the recorded journey from the
record's own bytes. One literal wording overstatement and three hygiene notes are
recorded below as non-blocking.

This is an increment verdict for the record repair only. The original issue #17
contract is untouched and unfinished: **1 of 8 desired 100-card starters certified**
(Jasmine only), 143-card catalog, 459-name union = 131 supported + 328 missing,
Ayula 22 short. The slice-24 journey is a 40-card masked-demonstrator package; it is
not product AI, not strength evidence, and not a 100-card certificate.

## What I ran (my own commands and outputs)

| Check | Command | Real result |
|---|---|---|
| HEAD identity | `git rev-parse HEAD` | `27d8f483cf431626c0eb95f308e6e328cf8353c3` |
| Delta shape | `git diff --stat 80fff73..27d8f48` | 13 files, +321/−21: 3 comment-only `crates/core` files, 1 doc, 1 driver assertion, 7 `slice27/` evidence files |
| Behavior neutrality | `git diff 80fff73..27d8f48 -- crates/` filtered to non-comment lines | **empty** — every `crates/` change is `//`, `///` or `//!` |
| fmt | `cargo fmt --all --check` | exit **0** |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit **0** (5 crates checked) |
| Focused | `cargo test -p aperture-core --locked --test issue17_slice26 --test issue17_slice25 --test issue17_slice24 --test issue17_slice23` | exit **0**, 11 + 10 + 7 + 6 = **34 passed, 0 failed** |
| Workspace | `cargo test --workspace --locked` | exit **0**, **113 suites, 798 passed, 0 failed, 1 ignored** — matches the recorded totals |
| Driver build+run | `cargo run --release --locked` in `evidence/issue17/slice24-journey-driver`, `CARGO_TARGET_DIR` = my scratch dir | exit **0**; terminal `Win{P3}`, **1602 steps / 37 turns / 4127 replay events**, digest `a41bbfd2760034f791a3616bd5d6faa0fd79ad8e9cee300eb69cc38c5a1ec631`, `replay_digest_matches: true` (`review27-driver-run.log`) |
| Artifact regeneration | `sha256sum` of `slice24/demo-{journey,save,replay}.json` before and after the run | identical: `2d6ec0ed…`, `857a6ca7…`, `681a6337…`; `git status --porcelain evidence/issue17/slice24/` shows no `M` rows. Nothing needed restoring |
| Corpus audit, their script | `python evidence/issue17/slice27/corpus-audit.py` | `AUDIT OK`, 143 / 459 = 131 + 328, per list `55/0/51/48/22/52/52/53`, 1 certified, exit 0 |
| Corpus audit, my recompute | out-of-tree Rust probe calling `aperture_core::catalog::all()` (143 `CardDef`s) + my own deck parser, compared against `manifest.json` `missing_cards` | **143 unique names / 459 union = 131 supported + 328 missing**, per list `55/0/51/48/22/52/52/53`, all eight decks exactly 99 library cards, computed missing set == manifest missing set for every list, 1 certified (`review27-independent-audit.log`) |
| Live PR46 | `gh pr view 46` | OPEN, draft, `headRefOid = 27d8f48…`, MERGEABLE, base `master` |
| CI, push | `gh run view 40493923124` | `Rust quality`, event `push`, headSha `27d8f48…`, **success**, 5/5 jobs success |
| CI, PR merge ref | `gh run view 40493931766 --log-failed` | `failure`; 3 rust jobs red, `windows package` + `board and roadmap` green (details in `review27-ci.md`) |

## The four claimed corrections

### 1. Layer documentation — **PASS**

* `crates/core/src/layer.rs:4-15` (module header): "implements four of them, each
  because a supported card prints it: layer 4 … (`Type`), layer 6 … (`Ability`),
  layer 7b … (`Setting`) and layer 7c … (`Modifying`)", then scopes the gap:
  "Layer 5 (colour change), 7a … and 7d … are not implemented", plus the 7d
  counters-lane note at `:12-15`.
* `crates/core/src/layer.rs:35-44` (enum doc): "Four layers are implemented, each
  arrived at by a card that prints it", 7a explained as dropped with issue 15's
  review, "Layers 5 (colour change) and 7d (+1/+1 counters) are likewise not
  implemented".
* `crates/core/src/engine.rs:446-463` (`recompute_layers`): "layer order — 4 type,
  6 ability, 7b base power/toughness, 7c modifiers" (`:447`) and the 7d note
  (`:461`). No "only 7c" text remains anywhere in `crates/` (`grep` finds only the
  PR-body description of what was fixed).
* Cross-check against the code, not the prose: enum variants `Type`/`Ability`/
  `Setting`/`Modifying` at `layer.rs:52/57/59/62` with `label()` → `4/6/7b/7c` at
  `layer.rs:68-71`; and the four are really exercised — `Layer::Ability` effects
  constructed at `engine.rs:679,703` (Goreclaw/static trample grant, slice 23),
  `Layer::Setting` at `engine.rs:6450` (Exuberant Wolfbear base P/T, slice 25),
  `Layer::Modifying` at `engine.rs:740` (pumps, slice 26 `+3/+3`), `Layer::Type` at
  `engine.rs:5942` (Ursine Champion subtype line, slice 26). Tests pin two of them:
  `tests/issue17_slice23.rs:55` asserts the `Ability` label, `tests/issue17_slice26.rs:127`
  asserts `Layer::Type` effects exist.

### 2. `docs/issue17-starter-corpus.md` — **PASS**

`docs/issue17-starter-corpus.md:456-459` now reads: "(Slice 25 added the layer 7b
base power/toughness setter and slice 26 the layer 4 subtype line, so layers 4, 6,
7b and 7c now exist; layer 5 colour changes and the animation effects that turn a
non-creature permanent into a creature are still missing.)" 7b is no longer claimed
missing, and colour change is correctly labelled layer 5. The remaining "animation"
claim is accurate rather than a mislabel: the implemented `Type` layer only replaces
the subtype line (`ContinuousKind::SetCreatureSubtypes`, `layer.rs:131-137`;
`Game::has_subtype` at `engine.rs:622-638`), it never adds the creature card type,
so a `Stuffed Bear`-style animation really is still missing.

### 3. Slice-24 driver assertion — **PASS** (with a non-blocking record caveat, N1)

* Committed at HEAD: `git show 27d8f48 -- evidence/issue17/slice24-journey-driver/src/main.rs`
  is the one-line change `"enters with a +1/+1 counter and gains haste"` →
  `"gets a +1/+1 counter and gains haste"`; `git show HEAD:…/main.rs:137` carries the
  corrected wording, and `crates/core/src/engine.rs:6479` emits exactly
  `"{name} gets a +1/+1 counter and gains haste until end of turn"`.
* The worktree copy of that file is byte-identical to HEAD after CRLF normalisation
  (both sha256 `34db45b8…` with `\r` stripped; `git status` reports no modification of it),
  so building it in place compiles the committed bytes.
* Built and run in my own target dir: exit **0**, `Win{P3}`, 1602 actions, 37 turns,
  4127 replay events, digest `a41bbfd2…` — exactly the recorded claim. The three
  `slice24/demo-*.json` artifacts regenerated byte-identically (hashes above), so
  there was nothing to restore.

### 4. `crates/core/src/card.rs` Little Bear comment — **PASS**

`crates/core/src/card.rs:202-205` now says the counter is conditional on "the
target's own *effective* subtype — `Game::has_subtype`, so a layer 4 type change
counts". The branch does exactly that: `engine.rs:6254-6262`,
`let is_bear = self.has_subtype(probe, "Bear");` where `has_subtype`
(`engine.rs:622-638`) consults active `Layer::Type` effects before the printed type
line. Comment now matches behavior.

## CI analysis (checked live, not inherited)

* **Push run 40493923124 — SUCCESS.** Workflow `Rust quality`, event `push`, headSha
  `27d8f48…`; all five jobs (board and roadmap, windows package, rust ×3) succeeded.
  This is the run that certifies the branch head itself.
* **Pull-request run 40493931766 — FAILURE** for `rust (windows|ubuntu|macos-latest)`;
  `windows package` and `board and roadmap` passed. The run is keyed to the PR head
  `27d8f48` but compiles GitHub's synthetic merge ref
  `f68997faa13abf29f75d650c9ccf5c5bd813b8b7` = this branch + `origin/master`
  `51dffdab` ("#28 Move review … (#56)"). `--log-failed` on all three:
  `error[E0308]: mismatched types … crates/ai/tests/review.rs:60:26 … expected u32, found u8 … crates/core/src/mana.rs:86`.
* Verified locally: `crates/ai/tests/review.rs` **does not exist on this branch**
  (`ls crates/ai/tests/`); it was added on master by `51dffda`, and its helper
  `fn pool(colors: &[(ManaColor, u8)])` passes `*count: u8` into `ManaPool::add`.
  `git show origin/master:crates/core/src/mana.rs | grep 'pub fn add'` →
  `amount: u8`; this branch's `crates/core/src/mana.rs:86` is `amount: u32`,
  introduced by branch-only commit `e300bdd`.
* **Attribution: NOT attributable to this delta.** It is a semantic conflict between
  master's brand-new test and older branch code, both predating `27d8f48`; the
  reviewed delta touches no type signature, no `mana.rs`, no `crates/ai`. The head's
  own push run is fully green. Recorded for the merge decision, not charged to this
  increment. Note the corollary: the "10/10 check runs" count for `27d8f48` is
  7 success + 3 failure — the earlier 10/10-per-head framing does not apply to this
  head, and the repair does not claim it.

## Findings

**Blocking: none.**

Non-blocking:

* **N1 (record wording, the only substantive nit).** PR46 body item 4 asserts "the
  worktree is clean". It is not literally clean: `git status --porcelain` reports four
  modified tracked files —
  `evidence/issue17/SLICE23-HUMAN-CHECKPOINT.md` (+47/−6),
  `evidence/issue17/sessions/…01a1720f….jsonl` (+982/−0, pure append),
  `evidence/issue17/review-sessions/…01a17237….jsonl` (+263/−0, pure append),
  `evidence/issue17/slice23/native-gates.log` (binary 6074→6087 B).
  None is in `crates/` or in the driver; all four carry mtimes 00:59–02:27, i.e. they
  predate the 04:08 repair commit, so they are pre-existing harness/slice-23 residue
  and outside this delta. The substantive requirement — no uncommitted driver fix —
  is met. Recommended wording: "no tracked implementation or driver file is modified".
  Any reproducer must still read the driver via `git show HEAD:` because the working
  tree as a whole is not equal to the head.
* **N2.** The evidence-only driver crate emits one warning on build: unused import
  `aperture_core::layer::ContinuousKind` (`main.rs:10`). It is not a workspace member
  and `clippy --workspace` is green, so this affects nothing, but the driver log now
  contains a warning line.
* **N3.** A pre-existing record defect surfaced while checking cleanliness:
  `HEAD:evidence/issue17/SLICE23-HUMAN-CHECKPOINT.md` still contains the literal
  placeholder `SLICE23_HEAD`, and the worktree carries the completed slice-23 text.
  Out of scope for this delta; worth folding into a later evidence-only commit rather
  than reconstructing the tree.

## Contract status (unchanged by this review)

1 of 8 desired 100-card starters certified; 143-card catalog; 459-name union = 131
supported + 328 missing; per list `55/0/51/48/22/52/52/53`; Ayula 22 short; Jasmine
the only certified list. Desired snapshots unchanged. Full #17 remains unfinished
regardless of this verdict. The slice-24 journey remains a 40-card masked
demonstrator, not product AI or strength evidence.

## Reviewer conduct

I made **no implementation edits**, no commits, no pushes, no merges, no issue or PR
state changes, and I did not touch `mana.rs`, `crates/ai`, or any tracked file. My
outputs are the five files in `evidence/issue17/slice27/` — `REVIEW27.md`,
`review27-gates.log`, `review27-driver-run.log`, `review27-independent-audit.log`,
`review27-ci.md` — plus scratch under
`C:/Users/alex3/AppData/Local/hermes/cache/scratch/rs27*` (build target dir, catalog
probe crate, my audit script). The driver run rewrote
`evidence/issue17/slice24/demo-{journey,save,replay}.json` with byte-identical
content; hashes before and after match, so no restore was needed.
