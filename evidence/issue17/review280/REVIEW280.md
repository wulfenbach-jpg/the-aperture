# Review 280 — slice 26 re-review (execution lens)

Reviewed head: `80fff7357c31ebaf54d4c0f3ca8907e91dbccce7` (record head; code head `9cd0b68`)
Branch: `aperture/issue-17`, draft PR46. Reviewer: run 280, read-only on implementation files.

This is round 2 of the slice-26 delta (`ea10693..9cd0b68`, plus the corrective record
commit `9cd0b68..80fff73`). Round 1 returned CHANGES_REQUESTED with four record findings
and no functional defect. Round 2 re-tests those corrections and re-runs the delivery bar.

## Verdict

**CHANGES_REQUESTED — no functional defect found; four record defects remain, three of
them introduced or left in files this increment itself edits.** The original #17 contract
is still unfinished (1 of 8 desired 100-card starters certified, 328 union names missing),
so this card also returns to sourced implementation regardless of the delta verdict.

## What I ran (my own, not the handoff narrative)

| Check | Result |
|---|---|
| `cargo test -p aperture-core --locked` focused 11 suites (`issue17_slice26/25/24/23`, `abilities`, `timing_mana`, `catalog_pool`, `card_printings`, `issue17_slice15_corpus`, `issue13_commander_combat`, `rules_conformance`) | **122 passed, 0 failed** (`focused2.log`, exit 0) |
| `cargo test --workspace --locked` | **113 suites, 798 passed, 0 failed**, exit 0 (`workspace-tests.log`) |
| `cargo fmt --all --check` | exit 0 (`fmt.log`) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 (`clippy.log`) |
| Journey rebuilt from **my own copy** of the driver (`review280-journey/`, output redirected) | exit 0; terminal `Win{P3}`, seed 2626, **1366 actions / 34 turns / 3613 replay events**, digest `86bbd60bcec7b2162507628ee0e7377acb3ef2b1237ffaf85b66e59d6995de19` |
| My artifacts vs the shipped ones | `demo-journey.json` **content-identical**; `demo-save.json` and `demo-replay.json` **byte-identical** |
| Separate release CLI `replay --file <my replay>` | `replayed digest 86bbd60bcec7 \| matches recorded \| events 3613` |
| Live exact-head CI (`gh api .../commits/80fff73/check-runs`) | **10/10 completed SUCCESS, every `head_sha` = 80fff73**, total_count 10 |
| Live PR46 head (`gh pr view 46`) | `headRefOid = 80fff73`, OPEN, draft |
| Live Scryfall `m19/304` vs fixture vs `catalog.rs` vs `ability.rs` oracle text | all four identical; printing/oracle ids, set, number, cost, type line, P/T all match |
| Independent corpus audit (`review280/corpus-audit2.py`, fails on any manifest mismatch) | **143 catalog / 459 union = 131 supported + 328 missing**, per list `55/0/51/48/22/52/52/53`, **1 of 8 certified**, every deck file 99 library cards, every computed missing set == manifest `missing_cards` |
| Delta scope | `crates/ai` and `crates/ui` untouched; only `decks/starter-corpus/manifest.json` changed under `decks/starter-corpus/` (one line: `Ursine Champion` removed from Ayula's missing list) |

Mechanics I saw behave as claimed in the code path: `Game::has_subtype` is the query used by
the Bear/Human target filters, Little Bear's counter branch, Ayula's fight revalidation and the
Surrak entered-Bear filter (`engine.rs:2432, 2441, 2453, 6261, 6428, 6535, 6689`), and the
layer ordering is `Type(4) → Ability(6) → Setting(7b) → Modifying(7c)`.

## Round-1 findings, re-tested

1. **False "first once-each-turn restriction" claim — FIXED.** No remaining claim outside the
   checkpoint's own description of the correction (`catalog.rs` comment, `ability.rs` comment,
   test-module header all corrected).
2. **Ayula "23 missing" — FIXED.** `docs/issue17-starter-corpus.md:67` and `:458` both say 22;
   my independent audit also says 22; historical per-slice counts (29 after slice 19, etc.) keep
   their slice labels and are not presented as current.
3. **"Little Bear reads the printed subtype" — FIXED IN THE DOC ONLY, not at every site.** The
   doc now says effective subtype, but `crates/core/src/card.rs:203-204` still reads
   *"The counter is conditional on the target's own printed subtype"* — the identical stale claim,
   in a file this increment edits. See B4.
4. **Mutation record predating `cargo fmt` — FIXED and independently re-checked.** The two
   recorded blob hashes in `slice26/pre-mutation-sha256.txt` equal
   `sha256sum crates/core/src/engine.rs crates/core/src/ability.rs` at this head
   (`24e0ef55…`, `a28f1572…`), and the panic lines in both RED logs (150, 190, 296, 380→382)
   resolve to the cited assertions in the current test file. `regenerate-mutation-evidence.py`
   replaces bytes (not text), asserts exactly the expected failing sets, and exits non-zero
   unless the tree is restored byte-for-byte.

## Blocking findings

**B1 — `layer.rs` and `engine.rs` still document a layer set the engine no longer has, and the
increment edited both files.** `crates/core/src/layer.rs:4-8` says *"This starter set implements
layer 7c … Layers 4–6 (type, colour, ability), 7a … and 7b … are not implemented because no
supported card needs them; they are refused at import rather than approximated"*, and
`layer.rs:31-37` says *"Only layer **7c** is implemented … Layers 4–6, 7b and 7d are likewise not
implemented"*. `crates/core/src/engine.rs:447` repeats *"Only layer 7c exists in this starter
set"*. At this head `Layer` has four variants — `Type` (4), `Ability` (6), `Setting` (7b),
`Modifying` (7c) — exercised by slices 23–26, and slice 26's own commit added `Layer::Type` to
this file while leaving the contradictory header. Fix: state the four implemented layers, and
scope the "not implemented" list to 5, 7a and 7d.

**B2 — a wrong layer statement was introduced by the corrective commit itself.**
`docs/issue17-starter-corpus.md:456-458` now reads *"(Slice 26 later added the one layer 4
subtype-line change; layers 5, 7b colour changes and animation are still missing.)"*. Colour change
is layer 5; 7b is base power/toughness setting, implemented since slice 25 (`Exuberant Wolfbear`,
`Layer::Setting`). Fix the sentence so the claimed gap matches the engine.

**B3 — the shipped slice-24 journey driver does not run at the recorded head; the fix is sitting
uncommitted in the worktree.** `evidence/issue17/slice24-journey-driver/src/main.rs:137` asserts
`description.contains("enters with a +1/+1 counter and gains haste")`, but `c2d9b35` (in this PR)
changed the emitted text to *"gets a +1/+1 counter and gains haste"*; nothing in `crates/` emits
the old string. I built and ran the **HEAD bytes** of that driver (`git show HEAD:…`, output
redirected) → **panic at src/main.rs:140 "the nontoken entry trigger must resolve", exit 101**
(`slice24-head-driver.log`). The working tree carries an uncommitted one-line edit to that same
file (`git status`: `M evidence/issue17/slice24-journey-driver/src/main.rs`, present before this
review started) that makes it pass — which is also why the local tree does not equal the recorded
head 80fff73. Fix: commit the corrected assertion (or otherwise make the shipped driver reproduce
at HEAD) and leave the tree clean; the slice-24 evidence claim is otherwise not reproducible.

**B4 — `crates/core/src/card.rs:203-204` still says Little Bear's counter is decided by the
target's *printed* subtype.** After slice 26 that branch reads `Game::has_subtype`, i.e. the
effective subtype. Correct the comment (see round-1 finding 3, which was fixed only in the doc).

## Non-blocking notes

- `80fff73` is described as a "docs/evidence-only follow-up"; it in fact edits `crates/core/src/
  ability.rs`, `catalog.rs`, `engine.rs` and `crates/core/tests/issue17_slice26.rs`. The changes are
  comment-only (verified line by line) and change no behavior, but "comment-only source edits, no
  behavior change" is the accurate wording.
- `evidence/issue17/slice26-journey-driver/Cargo.toml` still names the package
  `issue17-slice25-journey`.
- Retained disclosed debt from round 1 is accepted as debt, not re-raised: `has_subtype` timestamp
  ordering claim, `SubtypeLine::has` whitespace splitting, `once_per_turn` keyed on the object, old
  Rootwalla-era CR 602.5b citations.
- The journey is a 40-card masked-demonstrator package, not an AI-strength or certificate claim,
  and the handoff says so; I reproduce that framing.

## Contract status (original #17, unchanged by this review)

1 of 8 desired 100-card starters certified (Jasmine only). 143-card pool; 459-name union = 131
supported + 328 missing; Ayula 22 short. Desired deck snapshots unchanged, `Jasmine` still the only
certified list, arbitrary Archidekt compatibility still a non-goal.

## Reviewer conduct

No implementation files were edited by me; all my artifacts are under
`evidence/issue17/review280/`, `review280-journey/`, `review280-slice24-head/` and
`review280-slice24-driver/` (untracked). I did not merge, push master, close the issue, or unlock
`t_26a7a4ff` / `t_3c2fe51a`.
