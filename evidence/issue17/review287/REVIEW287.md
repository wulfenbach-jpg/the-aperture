# REVIEW 287 — slice 31 (Beorn the Fierce)

Head reviewed: `e53b987e50a46d0cddb44665eb6d94f9cfe7fefc` (draft PR46; local == remote == headRefOid).
Range: `073fd19eca457868bbd0d6be28026fb3b4071086..e53b987` (41 files, +32413/−24).
Mode: read-only. Every build/test run was executed in a scratch tree produced by
`git archive HEAD` at `C:/Users/alex3/AppData/Local/hermes/cache/scratch/r287`; the worktree was
never modified, no `cargo fmt` (only `--check`), no mutation script was run in the worktree.

## 1. Commands run, with exit codes

| # | command (cwd = scratch copy of e53b987 unless stated) | exit |
|---|---|---|
| 1 | `git rev-parse HEAD` / `git status --porcelain` (worktree) | 0 / 0 |
| 2 | `git diff --stat 073fd19..e53b987` | 0 |
| 3 | `git archive HEAD \| tar -x -C <scratch>` | 0 |
| 4 | `cargo test --locked -p aperture-core --test issue17_slice31` | 0 (8 passed / 0 failed) |
| 5 | `cargo test --workspace --locked` | 0 (117 suites / 827 passed / 0 failed) |
| 6 | `cargo fmt --all --check` | 0 |
| 7 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| 8 | `cargo build --release --locked` | 0 |
| 9 | `cargo run --release --locked` in `evidence/issue17/slice31-journey-driver` | 0 |
| 10 | `aperture-cli replay --file evidence/issue17/slice31/demo-replay.json` | 0 |
| 11 | same CLI replay over all 18 `evidence/issue17/slice*/demo-replay.json` | 16×0, 2×1 |
| 12 | release build of **073fd19** in a second scratch tree; replay slice15+16 there | 0 build; same 2 failures |
| 13 | `python evidence/issue17/slice31/corpus-audit.py` | 0 |
| 14 | my own audit script (independent regex + deck/manifest equality) | 0 |
| 15 | `sha256sum crates/core/src/engine.rs` (worktree) | 0 → `8fe51710…30d2` |
| 16 | my own 6 probes (`tests/review287_probe.rs`, scratch only) | 0 (6 passed) |
| 17 | my own single mutation (drop trample-counter read) + byte restore | test 101 (expected RED), restore verified |
| 18 | `gh api …/commits/e53b987/check-runs` | 10/10 success, all keyed to e53b987 |
| 19 | `gh pr view 46 --json headRefOid,isDraft,state,mergeStateStatus` | head e53b987, draft true, OPEN, BEHIND |
| 20 | `curl https://api.scryfall.com/cards/hob/119` and `/mom/337` | 200, 200 |

## 2. Reproduced myself (not accepted from logs)

* Focused suite 8/8, workspace 117/827/0, fmt, clippy `-D warnings`, locked release build: all green.
* Journey: terminal `Win{P2}`, seed 3131, 4 seats / 40 life, **1917 actions, 46 turns, 4930 replay events**,
  digest `36e181859f…da79f`, snapshot sha `370cfe3bcd…bc22a`, 7 Beorn casts, 13 triggers, 13 trample
  counters, 13 Bear additions, 8 failed Bear-count gates — and `demo-journey.json`, `demo-replay.json`,
  `demo-save.json` regenerated **byte-identical** to the committed files
  (sha256 `0808bd32…`, `a6d172e5…`, `aa071b30…`). Separate release-CLI replay matches.
* Mutation claim: my own mutation of the counter read fails exactly
  `trample_counter_lets_the_creature_trample_over_a_blocker` (7 passed / 1 failed) and restores
  `engine.rs` to `8fe51710…30d2`; the committed file already carries that hash, so no mutation residue.
* Corpus: `147 catalog / 459 union = 135 supported + 324 missing`, per-list
  `[55,0,51,48,18,52,52,53]`, 1 of 8 certified, and computed missing == manifest `missing_cards`
  for all eight lists — recomputed twice (their script and mine) with the same answer.
* No digest drift beyond the documented one: slice replay digests for 12,14,17–26,28,29,30 all still
  match their recordings at this head (e.g. slice30 `b711d966f9bd`, slice29 `ac16e510ca46`,
  slice24 `a41bbfd27600`). slice15/slice16 fail (Bearscape legality; digest mismatch) **identically at
  073fd19 and at e53b987** — pre-existing artifact rot, not caused by this increment.
* Provenance: live Scryfall HOB 119 matches the fixture on name, `{3}{G}{G}`, type line, 6/6, oracle id
  `30ca8e92…`, scryfall id `367d5f8b…`, set/collector number and `keywords:["Trample"]` exactly.

## 3. Specific questions

(a) **Once per turn, active player only.** `queue_beginning_of_combat_triggers` filters
`zone==Battlefield && controller==active` and is called only from the `Step::BeginCombat` arm; nothing is
stored. My probe walked four seat-turns in a 4-seat game: 4 `StepBegin{BeginCombat}` events, exactly
**1** `TriggerStaged` for Beorn (its controller's turn only). `BeginningOfCombat` is used by no other card.

(b) **"Up to one" is honest.** Probe: with exactly **one** legal target (Beorn itself) the offer still
contains `DeclineTriggerTarget` — the committed suite only covered two. Decline resolves the trigger with
no target (`apply_trigger_target_none` → `push_trigger(.., Vec::new())`). Mandatory triggers are refused:
injecting a `Pending::TriggerTarget` for Little Bear (`TargetSpec::CreatureYouControl`) offers no decline
and `submit_action` returns "no triggered ability with an optional target…"; the real target choice still
works. Ayula's fight mode (`BearYouControl`) goes through the same spec-driven
`trigger_target_is_optional`, so decline is refused there for the same reason (code-inspected, not card-special-cased).

(c) **Counter vs subtype.** A trample counter alone (no Beorn, no layer-6 grant) makes a 2/1 assign 1
damage over a blocking 1/1; the identical creature without the counter deals 0 over (control case). Their
test 7 shows the counter survives cleanup while the layer-4 Bear line expires (2/1 again next turn).

(d) **Anthem.** Own Bear 4/4, opponent's Bear 2/2, Beorn itself 6/6 after a real recompute — subtype-keyed,
controller-scoped, self-excluded.

(e) **Draw after the subtype change.** Beorn + one Human → no draw (2 Bears); Beorn + 2 Runeclaw Bear with
the target **declined** → 2 cards (3 Bears, decline does not suppress the draw); their test 3 covers the
targeted-into-Bear path.

(f) **No silent behaviour change found.** Diff is Beorn-gated except the `Step::BeginCombat` arm, whose
extra `put_triggers` is a no-op when the queue is empty and whose `pending`/`is_over` guards are
equivalent to the old `grant_priority`. 827 workspace tests and 16 historical replay digests agree.

(g) **Docs vs audit: consistent.** `docs/issue17-starter-corpus.md` and `SLICE31-HUMAN-CHECKPOINT.md`
say 147 / 135 / 324 / Ayula 18 / 1-of-8 — exactly what I recomputed.

## 4. Defects

None functional. I could not construct a failing case for any claim, and the one apparent failure in my
first probe run (`own Bear 2/2 instead of 4/4`) was a fixture artifact: a hand-built `Fixture` state has
never run `recompute_layers`; once the game passes through `settle`→`check_sbas` (which recomputes before
any priority) the value is 4/4. Not reachable in play.

## 5. Non-blocking observations

1. **PR body is one slice stale and numerically wrong for this head.** It still says "Catalog **146**",
   "459 = **134** supported + **325** missing", per-list `…/19/…`, "Ayula: **19**", "Last behavior change:
   slice 30", "Latest increment: slice 30". At e53b987 the computed values are 147 / 135 / 324 / 18 and
   slice 31. This is the only inconsistent record I found; it must be refreshed in finalisation, together
   with the usual `pr-body-*.md` receipt.
2. `evidence/issue17/slice15/demo-replay.json` and `slice16/demo-replay.json` no longer verify
   (pre-existing, proven identical at 073fd19). They should be marked stale rather than left looking green.
3. Printed additive 7c statics (of which Beorn's anthem is the first that buffs *other* permanents) apply
   only at recompute points. Disclosed, and safe in play, but a fixture-level test must drive a recompute;
   worth a line in the layer docs so the next 7c card's tests do not read stale P/T.
4. The Surrak and Goreclaw gap (prints Trample, catalog row is a plain `card!` so `has_trample()` is false,
   and its static excludes self) is real and is disclosed in the doc and the checkpoint — but it exists
   only in prose, not as a tracked issue. Recommend filing one.
5. `queue_beginning_of_combat_triggers` runs before the `pending_loss_exists()` guard that
   `grant_priority` would have applied. Unreachable today; a comment would keep it that way.
6. `stage_triggers` duplicates the `Pending::TriggerTarget` construction in the optional branch; a helper
   would keep the `first`/`queued` contract in one place.

## 6. Disclosures confirmed as declared, not hidden

* Journey = 40-card package, **not** a 100-card starter certificate (driver header, report `kind` field,
  checkpoint, docs). 1 of 8 lists certified, unchanged.
* Driver = evidence-only masked demonstrator, **not** product AI or strength evidence (asserted in its
  header and in the report payload).
* Surrak and Goreclaw conformance gap: declared in `docs/issue17-starter-corpus.md` §Slice 31 and in
  `SLICE31-HUMAN-CHECKPOINT.md`, with the reason. Confirmed true against Scryfall MOM 337.
  **Deferring it is the correct call for this slice**: repairing it changes an already-approved card and
  would move the slice-24/26/28/30 journey digests inside a card-addition increment, which is exactly the
  kind of silent cross-slice movement this review would otherwise have to reject.

## 7. CI

`repos/wulfenbach-jpg/the-aperture/commits/e53b987/check-runs`: **10 runs, all `completed`/`success`, all
keyed to e53b987** (rust ×3 and windows package and board ×2 each). PR46: OPEN, draft, `mergeStateStatus`
BEHIND (base moved; not a property of this delta).

VERDICT: APPROVED_INCREMENT
