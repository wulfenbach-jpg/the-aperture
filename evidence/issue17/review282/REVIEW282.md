# REVIEW 282 — slice 27 record repair (review round 4, contract lens)

Task: `t_67778feb` ("#17 Certify the v1 Commander starter corpus end to end")
Reviewer run: 282, claimed from the `review` lane. Round 4 (three prior `changes_requested` runs), so the
lead lens was **contract**: re-read the ORIGINAL four acceptance criteria, then audit the deliverable
strictly against them and confirm that every item from every prior `kanban_request_changes` round landed.

Head under review: **27d8f483cf431626c0eb95f308e6e328cf8353c3**
(local `HEAD` == `origin/aperture/issue-17` == PR46 `head.sha`; PR OPEN, draft, mergeable).
Delta under review: `80fff73..27d8f48`, 13 files, +321/−21.

Reviewer made **no implementation edits**; the only writes are this untracked evidence directory and
out-of-tree scratch copies. No merge, no master push, no issue closure, no downstream unlock.

## 1. Review-280 blocking items — 4/4 verified PASS, by execution

| # | Review-280 requirement | How I verified it | Result |
|---|---|---|---|
| 1 | `layer.rs` header + `Layer` doc and `engine.rs::recompute_layers` must state the four implemented layers and scope the gap to 5/7a/7d | Read the current bytes; `Layer` has exactly `Type`(4)/`Ability`(6)/`Setting`(7b)/`Modifying`(7c) with matching `label()`/`rules()`, and both doc blocks now enumerate those four and scope the gap to 5 (colour), 7a (CDA) and 7d (counters) | PASS |
| 2 | `docs/issue17-starter-corpus.md:456-458` must stop calling 7b a missing colour layer | Read the corrected paragraph: "Slice 25 added the layer 7b base power/toughness setter and slice 26 the layer 4 subtype line, so layers 4, 6, 7b and 7c now exist; layer 5 colour changes and the animation effects ... are still missing" | PASS |
| 3 | The shipped slice-24 journey driver must reproduce **from the record's own bytes** | `git archive HEAD` of `evidence/issue17/slice24-journey-driver` into an out-of-tree fake repo root (only `Cargo.toml` path deps made absolute; `src/main.rs` bytes untouched), `cargo run --release --locked` | PASS — exit 0 (review-280 saw panic at `src/main.rs:140`, exit 101) |
| 4 | `card.rs` must say Little Bear's counter reads the *effective* subtype | Read the doc comment and the code it describes: `engine.rs:6262` `let is_bear = self.has_subtype(probe, "Bear")` | PASS |

Driver reproduction detail (mine, not the handoff's):

```
Win{P3} · 1602 steps · 37 turns · 4127 replay events
digest a41bbfd2760034f791a3616bd5d6faa0fd79ad8e9cee300eb69cc38c5a1ec631
snapshot sha256 feb9033b925198a0eae49b117c858b626e968a4e7691c8e70c7e93bffc4d7455
casts Surrak 6 · Centaur Courser 6 · Grizzly Bears 6 · Craw Wurm 5 · Runeclaw Bear 3
entry_buff_events 11 · haste_grant_events 11 · blocked_trample_damage_events 20
```

Regenerated artifacts are **byte-identical** to the committed ones:

| artifact | my run | committed (`git show HEAD:`) |
|---|---|---|
| demo-journey.json | 2d6ec0ed6a6f…45b4 | 2d6ec0ed6a6f…45b4 |
| demo-save.json | 857a6ca7bcdb…940d | 857a6ca7bcdb…940d |
| demo-replay.json | 681a633735d1…cd90 | 681a633735d1…cd90 |

Separate replay through the product binary: `aperture-cli replay --file demo-replay.json` →
"recorded outcome Some(Win { winner: 3 … })", 1602 actions, **replayed digest a41bbfd2 matches recorded**, 4127 events, exit 0.

## 2. Delta character — comment-only, no behaviour change

`git diff 80fff73..27d8f48 -- crates/` filtered to lines that do not begin with `//`, `/*` or `*` is **empty**.
`git diff --stat 80fff73..27d8f48 -- crates/ai crates/ui` is empty: the AI and UI lanes are untouched.
So the handoff's "comment-only source edits, no behaviour change" is accurate (it is *not* the earlier, overstated
"evidence-only" wording).

## 3. Gates I ran at 27d8f48

* `cargo test --workspace --locked` → **113 suites, 798 passed, 0 failed**, exit 0.
* `cargo fmt --all --check` → exit 0. `git diff --check -- crates/` → exit 0.
* `cargo clippy --workspace --all-targets --locked -- -D warnings` → exit 0.

## 4. Independent corpus audit (my own script, not the lane's)

Sources of truth: `crates/core/src/catalog.rs` parsed with a different rule (any `name!(Id, "Name"` macro form)
and cross-checked against the **built CLI's own** `pool` output ("THE APERTURE supported pool — 143 cards"),
plus the eight desired deck files and `decks/starter-corpus/manifest.json` (asserting 99 library cards +
1 commander per list and comparing my computed missing set to the manifest's `missing_cards`).

```
catalog_parse_names = 143        (CLI header agrees: 143 cards)
union_names         = 459
union_supported     = 131
union_missing       = 328
isamaru-white-aggro   planned    missing=55   manifest_agrees=True
jasmine-vanilla-stompy certified missing= 0   manifest_agrees=True
talrand-draw-go       planned    missing=51   manifest_agrees=True
krenko-goblin-tokens  planned    missing=48   manifest_agrees=True
ayula-bear-counters   planned    missing=22   manifest_agrees=True
elas-sacrifice        planned    missing=52   manifest_agrees=True
tatyova-landfall      planned    missing=52   manifest_agrees=True
zada-spells           planned    missing=53   manifest_agrees=True
per_list_missing = [55, 0, 51, 48, 22, 52, 52, 53]   certified = 1
```

Desired-list bookkeeping changed since the last approved head (`ea10693..27d8f48`) in exactly one place:
`Ursine Champion` was removed from Ayula's `missing_cards` (correct — slice 26 implemented it), plus the new
evidence-only `decks/package-slice26.txt`. No desired snapshot was edited.

## 5. Sourcing claim spot-checked against live Scryfall

`evidence/issue17/slice27/ayula-missing-sources/` holds one `/cards/collection` call
(stamp `2026-10-27T11:39:32.989392+00:00`, `not_found: []`) and 22 normalized rows. I re-fetched four
printings live and compared name/set/number/cost/type/P-T/oracle text: `afr/177` Circle of the Moon Druid,
`blb/183` Lumra, `spm/114` Spider-Ham, `dsc/36` Ursine Monstrosity — all **exact**. The work order's
"blocked" verdicts (search, equipment, Ring, CDA/7a, additive type change) match the primitives the tree
actually ships.

## 6. CI at this exact head — read precisely, and the failure is external

`gh api .../commits/27d8f48.../check-runs` → 10 checks, every `head_sha = 27d8f483`:

* push run 40493923124 — **SUCCESS 5/5** (rust ×3, windows package, board and roadmap).
* pull_request run 40493931766 — **FAILURE**: `rust (windows)`/`rust (ubuntu)`/`rust (macos)` fail,
  `windows package` and `board and roadmap` succeed.

I pulled the job logs myself rather than trusting the handoff: all three rust jobs fail with
`error[E0308]: mismatched types` at `crates/ai/tests/review.rs:60` because `crates/core/src/mana.rs:86`
here is `pub fn add(&mut self, color: ManaColor, amount: u32)`. `crates/ai/tests/review.rs` **does not exist
on this branch** (`git ls-tree HEAD` empty, no history) — it arrives in the synthetic merge with master,
so this is master-drift incompatibility (#28/PR56's `u8` call site vs this branch's widening `e300bdd`),
not a slice-27 defect. PR46's required contexts therefore cannot go green until the mana API is reconciled
or the merge order changes; that reconciliation is the supervisor's, and the hotspot is already flagged on
the card. This review does **not** count the push-only green as a required-check pass.

## 7. Contract audit against the ORIGINAL acceptance criteria

| Original criterion | State at 27d8f48 |
|---|---|
| 1. At least eight explicitly supported 100-card Commander lists | **NOT MET** — 1 of 8 certified (`jasmine-vanilla-stompy`); seven are `planned` |
| 2. Every included card/interaction implemented with conformance coverage | **NOT MET** — 328 of 459 union names unsupported (Ayula 22 short is the nearest list) |
| 3. Four-seat games on exact deck snapshots, never bundled replacements | **PARTIAL** — the four-seat/40-life journeys use exact snapshots but of 40-card *packages*; only one 100-card list is certified |
| 4. Per-card/interaction exclusions documented; Archidekt compatibility an explicit non-goal | **MET in substance** — `docs/issue17-starter-corpus.md` + per-list `missing_cards`; one imprecise clause noted below |

Full #17 is therefore **unfinished**, and this card must return to implementation — not complete.

## 8. Findings

Blocking delta defects: **none.** The four review-280 record defects are corrected and evidenced.

Non-blocking, recorded (not a hold):

1. `docs/issue17-starter-corpus.md:192` still reads "Haste and wider keyword semantics remain unimplemented"
   while `Game::has_haste` (engine.rs:662) gates summoning-sickness legal actions at five call sites and
   slice 24 adds `ContinuousKind::GrantsHaste` (CR 613.4d/702.10a). Narrowly true only for a *printed* Haste
   keyword (no supported card prints one), but as written it reads as "haste is not supported". Recommend the
   word "printed" when this document is next edited; raised as optional in run 278 and still open.
2. `evidence/issue17/slice25-journey-driver/src/main.rs:4` still describes the slice-25 driver as casting
   Surrak first (slice-24 text). Untouched file, carried from run 278's optional list.
3. The slice-24 evidence driver emits one `unused_imports` warning (`ContinuousKind`); the workspace itself is
   clippy-clean.
4. External: PR46 required checks red from the `mana.rs`/master drift described in §6.

## Verdict

**APPROVED_INCREMENT at 27d8f48** (4/4 review-280 corrections verified by execution, comment-only delta,
no functional defect), **returned to implementation** because the original #17 contract is unchanged and
unfinished. Next step is slice 28 per `evidence/issue17/slice27/AYULA-NEXT-PACKAGE.md`: Circle of the Moon
Druid (AFR 177) — the only remaining Ayula card expressible with shipped primitives.
