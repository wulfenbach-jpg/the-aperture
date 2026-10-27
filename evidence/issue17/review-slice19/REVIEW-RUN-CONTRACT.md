# Slice 19 — independent review, round 4 (contract lens)

Reviewer: independent session `sdlc-review`, read-only in
`C:/AI/aperture-worktrees/issue-17`. No implementation file was edited.
Scope: original issue #17 acceptance criteria first, then the slice-19 delta
`21fdefb..7dc933a`, then every item from the prior `changes_requested` rounds.

## Verdict

**Increment APPROVED. Original #17 remains UNFINISHED and the card must return to
implementation.** One record correction is required (AI-crate compatibility touch is
undisclosed); no code defect was found. Full issue acceptance is **not** satisfied:
1 of 8 desired 100-card starters is certified and 335 of the 459 referenced names
are still unimplemented.

Reviewed head `7dc933a632f37392731dfd702284e2206d7eeb82` == `origin/aperture/issue-17`
== PR46 head (OPEN, draft, MERGEABLE, BEHIND master).

## Original acceptance criteria, mapped

| Criterion | State at this head | Evidence |
|---|---|---|
| 8 supported 100-card lists | **OPEN — 1 of 8** | manifest: only `jasmine-vanilla-stompy` has `status: certified`; seven are `planned` |
| Every included card/interaction implemented + conformance coverage | holds for the one certified list; open for the other seven | 136-card pool, per-list `missing_cards`; 6 slice-19 tests + `catalog_pool` printed-ability mapping + `card_printings` provenance record |
| Four-seat games on exact deck snapshots, never bundled replacements | **re-verified for the certified list** | my own `aperture-cli certify --starter jasmine-vanilla-stompy --seats 4 --seeds 1,2,3` → `certified`, games terminal, replays verified, snapshot sha256 `4312ab2a90a94a4f…` == manifest |
| Document per-card/interaction exclusions; arbitrary Archidekt is a non-goal | holds | `manifest.json` `missing_cards`/`missing_interactions` + `certification.interaction_exclusions`; docs "Phased gaps" |

## What I ran myself (not read from the handoff)

- `cargo test -p aperture-core --test issue17_slice19 --locked` → **6 passed / 0 failed**.
- `cargo test -p aperture-core --test card_printings --test catalog_pool
  --test issue13_commander_combat --test issue17_slice15_corpus --locked` → **8 / 5 / 43 / 2 all pass**
  (the four suites round 1 proved broken at its reviewed head).
- `cargo test --workspace --all-targets --locked --no-fail-fast` → **exit 0, no failure line** at this head.
  `gates.log` aggregates to **103 suites / 736 passed / 0 failed**; my re-run agrees it is green.
- `gh api …/commits/7dc933a/check-runs` → **10/10 completed SUCCESS, every head_sha = 7dc933a**
  (rust ubuntu/macos/windows ×2, windows package ×2, board and roadmap ×2).
- `aperture-cli replay --file evidence/issue17/slice19/demo-replay.json` → seed 1919, 4 seats,
  **1773 actions, 4709 events, digest 5fd841b2 matches, Win{P3}**.
- `replay --verbose` → exactly **13** "doesn't untap during this untap step (CR 502.3)" events.
- **Rebuilt and re-ran the journey driver myself** (`cargo run --release
  --manifest-path evidence/issue17/slice19-journey-driver/Cargo.toml`, current lane target):
  identical JSON — 1773 steps, 42 turns, 4709 events, digest `5fd841b2…`, Spectral Bears
  cast 5×, trigger staged 14×, 13 skip-untap events, `snapshot_sha256` ==
  sha256(`decks/package-slice19.txt`) = `b5d65619…c1b1` (recomputed by me).
- Wire compatibility by execution: slice-18 replay → digest `09e6bb25…` and slice-17
  replay → `85fabffc…` both still verify, so the `skip_next_untap` serde addition moved
  no recorded digest.
- Live Scryfall read of `e56143db-0274-4878-9d54-619e7f037a33` → **me1 131**, `{1}{G}`,
  "Creature — Bear Spirit", 3/3, oracle text, oracle id and Commander legality all
  byte-equal to `fixtures/starter-corpus/slice19-sourced.json`.
- My own corpus audit (independent parser, catalog source + all eight desired decks +
  manifest): **136 catalog, 459 union = 124 supported + 335 missing**, per-list
  `55/0/51/48/29/52/52/53`, every library 99 cards, manifest `missing_cards` ==
  names − catalog for all eight, certified list == jasmine only.
- `git diff --name-status 21fdefb..7dc933a`: no desired-starter deck touched; only
  `decks/starter-corpus/manifest.json` (Ayula entry) changed under `decks/starter-corpus`.
  `f80ef86..7dc933a` is docs/evidence only plus untracking `.starter-run.yml`.

## Prior `changes_requested` items — all landed

- Run 240 #1 (docs Ayula count): docs now says 29 consistently with manifest/checkpoint.
- Run 240 #2 (disclose Krenko injection in the slice-14 checkpoint): disclosed at
  `SLICE14-HUMAN-CHECKPOINT.md:51-57`.
- Run 240 #3 (CR 702.14c): every enforcement site cites 702.14c; the only 702.14b left is
  the definitional "landwalk is an evasion ability" (`card.rs:420`), which matches the
  pinned text (702.14b), and `SLICE11-HUMAN-CHECKPOINT.md:72` now says **6** focused tests.
- Run 240 #4 (slice-14 fixture note): now "four records, one per sourced card".
- Round-1 findings on this delta (test suite did not compile, missing printing record, two
  pinned-count suites, dishonest `gates.log`, tested≠committed): all fixed at `f80ef86`,
  independently confirmed above.
- No stale `135/123/336/30 missing/42 missing` remains in README or docs.

## Required record correction (only actionable item)

1. **`crates/ai/src/search.rs` and `crates/ai/tests/common.rs` are edited by this delta
   and nowhere disclosed.** `b04aa1e` adds
   `skip_next_untap: real_state.skip_next_untap.clone()` to `reconstruct()` and
   `skip_next_untap: Default::default()` to the AI test fixture. Both are mechanically
   forced by the new `GameState` field (struct-literal exhaustiveness) and look inert —
   no policy, search or model logic changed — but this card's ownership says
   "core/corpus/deck fixtures only. No … AI-search or model edits", and the slice-17
   precedent was to disclose exactly this class of touch as a compatibility exception
   subject to review. `SLICE19-HUMAN-CHECKPOINT.md` lists four engine changes and its
   deviations section, and neither mentions the AI crate. Add the disclosure (one
   sentence, with the two line references and "no policy change") to the checkpoint and
   the handoff metadata; no code change is requested.

## Non-blocking notes

- `gates.log` is bound to `f80ef86` (it prints `commit:` itself); the final head is
  evidence-only, and exact-head CI re-runs fmt/test/clippy at `7dc933a`, so this is
  recorded rather than a gap.
- The untap restriction surfaces as `Event::StateBasedAction` prose and the journey
  driver injects Ayula for all four seats; both are disclosed and are demonstration,
  not product, behavior.
- CR citations used for the new surfaces are defensible against the pinned text
  (508.1 declare attackers, 603.4 intervening-if, 502.3 untap step, 202.2a colour from
  mana symbols, 111.6 "a token isn't a card" for the nontoken clause).

## Next gate (unchanged)

Same card / PR46 / branch / saved histories. Continue sourced Ayula-list cards and then
genuine certification of a whole desired 100-card list; 335 union names are missing and
seven starters remain uncertified. No merge, no master push, no issue closure, no
certified-downstream unlock. `full_issue17_accepted = false`.
