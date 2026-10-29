# Slice 23 — independent practical review record

**Verdict: APPROVED for the delta `89247d6..632c5e7`, no blocking defect.**

Reviewer: a fresh delegated agent with no access to this worker's reasoning, given
only the commit range, the printed card text and the writer's claims to falsify.
It is **not** one of the original OMP JSONL review histories: this slice's writer
and reviewer ran natively, and both pre-existing append-only session logs are
unchanged by it (disclosed, not claimed as history continuity).

## What the reviewer executed (its own report, condensed)

* `git diff --stat 89247d6..632c5e7` plus a full read of the changed `crates/core`
  source and the new test file; card text checked against both
  `fixtures/starter-corpus/slice23-sourced.json` and the committed raw Scryfall
  record (CMM 293, `{3}{G}`, 4/3, oracle identical).
* `issue17_slice23` 11/11; `card_printings` 8, `catalog_pool` 5,
  `issue13_commander_combat` 43, `issue17_slice15_corpus` 2, `abilities` 13,
  `issue17_slice20` 9, `issue17_slice21` 8, `issue17_slice22` 9, `projection` 4,
  `timing_mana` 12, `m1_multiplayer` 28 — all green.
* `fmt --check`, `clippy -D warnings`, locked release build: exit 0.
* Full workspace suite: exit 0, 110 binaries, 775 passed / 0 failed / 1 ignored.
* `python evidence/issue17/slice23/corpus-audit.py` → 140 / 459 = 128 + 331,
  Ayula 25, 1 certified.
* Release-CLI replays of slice 17–22 still "match recorded" (slice22
  `17f03348d81c`); slice 23 replay matches `e2ccc113bceb`; save recomputes.
* Rebuilt and re-ran the journey driver in a scratch copy: identical digest,
  1322 steps, 45 grants, 12 staged triggers, and byte-identical regenerated
  `demo-replay.json` / `demo-save.json` / `demo-journey.json`.
* **15 of its own probes** in an out-of-tree scratch crate (repo tests untouched):
  no reduction for a noncreature spell or another seat's creature; no reduction
  below the coloured pips; power 3 refused at the reduced price while power 4 is
  accepted; a Goreclaw in hand/graveyard/library discounts nothing; the cut is
  exactly {2}, not {4}, and a rejected attempt spends nothing; a Rootwalla
  activation is not reduced; real combat 40→31 with the trigger versus 40 without
  a trigger on the same board; a 3/3 attacker stays 3/3 with no grant and the
  opponent's 6/4 is untouched; grant and pump are gone after cleanup and stay gone
  the following turn; the grant is pruned when the creature leaves and an object
  reusing the same id gains nothing (CR 400.7); commander tax plus reduction
  reports the reduced total; `Layer::Ability < Layer::Modifying`; a live grant
  survives save/load and still tramples after reload; `Object::has_trample` is
  still printed-only; two copies stack to {4}.

## Non-blocking findings (stale comments the delta created — carried forward)

1. `README.md:303` still lists "layers 4–6/7b/7d" as not implemented although a
   layer 6 ability grant now exists.
2. `crates/core/src/layer.rs:3-8` and `:31-37` still assert that only layer 7c is
   implemented and that layers 4–6 are not.
3. `crates/core/src/engine.rs:446-448` ("Only layer 7c exists in this starter
   set") is stale for the same reason.
4. `crates/core/src/state.rs:543-546`: `Object::has_trample`'s comment says
   "nothing in this pool grants or removes trample", which is now false. The
   method is printed-only and combat uses `Game::has_trample`, so there is no
   behaviour bug today, but any future UI/coach/AI caller would silently miss a
   granted keyword.

None of these change behaviour; they are documentation debt recorded for the next
slice rather than fixed in an evidence-only head after the reviewed commit.

## Not verified by the reviewer

* The recorded mutation-RED logs and `native-gates.log` (re-running the mutation
  script mutates the worktree, outside a read-only review).
* Actual UI rendering of keywords; the "view does not show granted keywords"
  limit was confirmed only at projection level.
* Remote CI/PR state (no network actions); the writer captured 10/10 exact-head
  SUCCESS separately.
