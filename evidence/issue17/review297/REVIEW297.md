# Review 297 — issue 17 slice 35 (Words of Wilding, ONS 305)

Reviewer: `default` (independent review run, same card / PR46 / branch `aperture/issue-17`).
Reviewed head: `ec8dac23ff219f844c86acc803da20a1d5397fe7` (code head `ac3b422b066355c8f5e14e98dd78a9ff6fcb3e89`).
Delta: `6575c8b..ec8dac2` (16 files outside `evidence/`+`docs/`; 37 files in total).
Lens: execution (checkout-and-run), with a contract re-audit against the original issue-17 criteria.

## Verdict

**CHANGES_REQUESTED — evidence only. No functional defect found.**

The card's implementation is correct, and I verified it by running it myself in real
four-seat games (below). What the deliverable does not carry is the delivery-bar
evidence the supervisor set for a *replacement effect*: a committed driver that casts
the card in a four-seat exact-snapshot game and consumes a replacement, with the event
evidence. The lane disclosed this gap honestly (checkpoint line "the heuristic policy
drew Words of Wilding three times and never cast it"), which is why this is a
correctable omission rather than a false claim.

Everything else in the increment is verified and should be preserved unchanged.

## What I verified by execution (out of tree, `git archive ec8dac2` → `C:/AI/r297a`)

* `cargo test -p aperture-core --test issue17_slice35 --locked` → **6 passed / 0 failed**;
  `card_printings` → **13 passed / 0 failed**.
* `cargo test --workspace --locked` → **120 suites / 853 passed / 0 failed / 1 ignored**,
  exit 0 (`workspace-summary.txt`).
* `cargo fmt --all --check` exit 0; `cargo clippy --workspace --all-targets --locked
  -- -D warnings` exit 0.
* **Journey reproduced byte-for-byte**: `selfplay --seed 3535 --seats 4
  --commander-combat-practice` on `decks/package-slice35.txt` ×4 with Jasmine Boreal →
  terminal **Win{P2}**, **1360 steps / 32 turns / 3564 events**, digest
  `59f8bf201768…`, `replay_digest_matches: true`, `pool_size 150`, starting life 40,
  21 casts / 10 ability activations — identical to the record. The verbose log also
  confirms the disclosed gap: three `P0 draws Words of Wilding` and no cast of it.
* **Recorded digests are preserved** (the sweep the lane did not run): replaying the
  committed replay files at this head reproduces the recorded digests exactly —
  slice 33 `3dfb14d4ce76` (1117/2967), slice 31 `36e181859f39` (1917/4930),
  slice 30 `b711d966f9bd` (2653/6968), slice 35 `59f8bf201768` (1360/3564).
  The `serde(default, skip_serializing_if = "Vec::is_empty")` claim holds in practice.
* **Live source**: Scryfall `ons/305` → id `fdb9565f…`, oracle `888582df…`,
  `{2}{G}`, "Enchantment", oracle text
  "{1}: The next time you would draw a card this turn, create a 2/2 green Bear creature
  token instead." — identical to the fixture, the catalog row and the ability text.
  One live ruling only (multiple Words: the player chooses which to use up per draw).
* **Independent corpus audit** (my own script over `crates/core/src/catalog.rs`, the
  eight desired deck files and the manifest): **150 catalog / 459 union = 138 supported
  + 321 missing**, per list `55/0/51/48/15/52/52/53`, Ayula **15**, **1 of 8 certified**,
  manifest `missing_cards` set-equal on all eight. Matches the checkpoint, the docs and
  `evidence/issue17/slice35/corpus-audit.log` exactly. No stale 149/16 figure survives
  in `README.md`, `docs/` or `crates/`.
* **CI read back live**, not inherited: `ac3b422` → 10/10 completed SUCCESS all keyed to
  it; the **docs tip `ec8dac2` also has its own 10 check runs, now 10/10 completed
  SUCCESS** (at review time two were still `board and roadmap`/`rust (ubuntu)` green and
  eight in progress; the final readback is 10/10). PR 46 is OPEN / draft / BEHIND master
  at head `ec8dac2` = local = remote.
* **Both of the lane's mutations reproduce from committed bytes** in my own tree:
  disabling the replacement consumption in `draw_with_resume` fails exactly
  `the_armed_replacement_turns_that_turns_draw_step_into_a_bear`,
  `one_armed_replacement_replaces_exactly_one_of_two_draws` and
  `two_armed_replacements_replace_both_draws_and_protect_an_empty_library` (3 pass / 3
  fail, exit 101); removing the cleanup clear fails exactly
  `an_unspent_replacement_ends_with_the_turn` (5 pass / 1 fail, exit 101). `engine.rs`
  restored and hash-verified both times (`8750301f…` in the archive tree; the record's
  `7a833ec2…` is the git blob and the worktree checkout, which I also confirmed).
* **My own probe** (`probe.rs` here, own crate outside the worktree, public API only,
  path dep on the reviewed core): three real four-seat / 40-life commander-combat games
  from `decks/package-slice35.txt` driven by a policy that deliberately casts and
  activates Words of Wilding — **casts 4 / 5 / 6, activations 2252 / 2346 / 2254,
  armings 2252 / 2346 / 2254, consumed replacements 601 / 588 / 604 (1793 total),
  unspent replacements cleared 1648 / 1755 / 1647 (5050 total)**. Every consumption was
  instrumented: library and hand unchanged, **no `Event::Draw` for that seat**, no
  `DrewFromEmpty`, and exactly one new token that is a creature, 2/2 and green. Every
  clear was an end-of-turn cleanup with no token and no draw. No unexplained
  `draw_replacements` transition occurred.

## Blocking item (the only required change)

1. **The replacement effect is not exercised by any committed evidence artifact.**
   The four-seat journey in `evidence/issue17/slice35/` never casts the card, so the
   record demonstrates the new rules surface only through unit tests, which is exactly
   what the supervisor excluded for a replacement effect. My probe shows the behaviour,
   but a reviewer's out-of-tree probe is not the deliverable's evidence, and the reviewer
   must not commit implementation or evidence artifacts on the implementer's behalf.

   Minimum outcome: commit a driver (the `evidence/issue17/slice33-journey-driver` pattern)
   that plays a real four-seat / 40-life game from `decks/package-slice35.txt` with the
   `commander-combat-practice` profile and Jasmine Boreal in every seat, **casts Words of
   Wilding, activates it, and asserts the replaced draw from the engine's own events** —
   no `Event::Draw` for that seat, library and hand unchanged, exactly one 2/2 green Bear
   creature token created, no `DrewFromEmpty` and no loss — plus its run log and the
   regenerated `demo-journey`/`demo-replay` receipts. No engine change is expected; if the
   driver's assertions cannot be made to pass, that is a new defect and must be reported
   rather than patched silently.

## Non-blocking notes

2. `Effect::ReplaceNextDrawWithBearToken`'s description (`crates/core/src/card.rs:437`)
   reads "the next time **that player** would draw this turn…"; the printed subject for
   this activated ability is "you". Sibling self-scoped effects use "you". Wording only,
   but it is player-facing text.
3. `docs/issue17-starter-corpus.md` slice-33 "Known limits" now says "Ayula has **15**
   missing names" — this delta retro-edited a historical section's own-time figure (it
   was 16 when slice 33 landed, and the neighbouring slice sections still carry their
   own-time counts 21/20/18/17). The slice-35 section already states 15 correctly.
4. The record's CI line cites only `ac3b422`; the docs tip `ec8dac2` now has its own
   10/10. Worth recording at the evidence commit so the claim binds to the shipped tip.
5. `crates/ai/src/search.rs` gains a forced initializer that copies `draw_replacements`
   into the search projection. It is truthfulness plumbing, not a search change, and it is
   disclosed; the lane's "no AI-search edits" constraint is materially respected, but the
   projection now simulates armed replacements — which is the correct projection.

## Answers to the four review-focus questions (and the commander-draw ordering)

1. **Replacing the draw before the empty-library check is the printed reading, and
   "no `Event::Draw`" is the right observable.** CR 614.1a makes this a replacement
   effect and CR 614.6 says a replaced event "never happens", so no card is drawn; CR
   704.5b/121.4 act on a player who *attempts* to draw from an empty library, and the
   attempt is part of the replaced event. The engine's `drew_from_empty` flag is set only
   inside the branch the replacement precedes (`engine.rs:2111-2112`). Verified by test 3
   and by 1793 probe consumptions with zero `DrewFromEmpty` and zero losses.
2. **An off-turn activation expires at the end of the active seat's turn, and that is what
   "this turn" means.** CR 514.2 ends all "this turn" effects at that turn's cleanup,
   whoever armed them. Observed directly: 5050 clears, all at `EndStep`, including
   non-active seats' armed replacements, with no token and no draw.
3. **The armed replacement is correctly not pruned when the source leaves.** Only three
   sites touch the field: arm (`engine.rs:2032-2037`), consume (`:2043`), cleanup clear
   (`:5360-5363`); nothing binds it to an object, so departure cannot prune it. That
   matches a resolved one-shot continuous effect, which is not tied to its source
   (CR 611.2). (No supported card in the pool can destroy the enchantment, so this is
   established by construction plus those three call sites.)
4. **The serde attributes do preserve every previously recorded journey digest** — proven
   above against four committed replay files, which is stronger than the "no sweep was
   run" disclosure in the lane's record.
5. **Commander-draw ordering (supervisor's question).** The replacement is consumed
   before the CR 903.9b commander-draw suspension, so an armed replacement pre-empts the
   commander reveal. Both are replacement effects on the same draw, and CR 616.1 gives the
   affected player the choice; this engine has no such choice surface. It is unreachable
   with the current pool (no supported effect puts a commander into a library), so this is
   a defensible simplification — but it should be a one-line limits note, not silence.

## Scope, preservation, and what this review does not do

* Approved increments 1–34 were not re-audited. The delta is `crates/core` (card, state,
  engine, ability, catalog, tests), docs/fixtures/decks/evidence, README, and the two
  forced initializer-only lines outside `crates/core`. No `crates/ui`, no import path, no
  model behaviour, no desired starter snapshot changed (`decks/starter-corpus/` untouched
  apart from the manifest's single removal of "Words of Wilding" from Ayula's
  `missing_cards`).
* Original issue 17 remains **unfinished**: 1 of 8 desired 100-card lists certified,
  Ayula 15 names short, 321 of 459 union names unsupported, and the journey is a 40-card
  evidence package, not a certificate.
* This review did not merge, rebase, push master, close the issue, or unlock
  `t_26a7a4ff` / `t_3c2fe51a`. No implementation file was edited by the reviewer; the only
  files I added are untracked review evidence under `evidence/issue17/review297/`.

## Artifacts in this directory

`probe.rs` + `probe-Cargo.toml` (the driver), `probe-run.log`, `journey35.json`,
`audit297.py` / `audit297.out`, `slice35-tests.log`, `printings.log`, `workspace-summary.txt`,
`fmt.log`, `clippy.log`, `mut-consumption.log`, `mut-cleanup.log`, `ons305.json`,
`ons305-rulings.json`.
