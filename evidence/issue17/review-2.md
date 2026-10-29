# Issue 17 slice 1 — exact-history re-review (review 2)

Verdict: **approve_increment**. This approves the bounded partial increment only;
it does **not** approve all of issue #17, eight implemented styles, merge, release
or closure. `full_issue17_accepted: false`.

Reviewed head: `86db7d316edd6200f7f598849018e428d11b69eb` (one commit over prior
reviewed `0a7d781c41d1799a79c8ef41e6888d81ab7f9e6d`). Tracked delta contains only
the loader fix, corrected corpus/handoff text, and repair/review evidence. No
engine, card/catalog, deck-list or source files changed.

## Prior findings retested

1. **Default CLI deck identity — resolved.**
   `crates/cli/src/decks.rs` applies `file_stem()` only when the path is
   explicit; non-explicit default loads keep `fallback_name`. The new
   `non_explicit_default_loads_keep_the_human_fallback_names` test and the
   existing explicit-starter identity test both pass. A real release-binary
   probe confirms the unchanged default invocation again reports
   `Temur Beats` / `Izzet Control`.

2. **Tracked docs vs manifest — resolved.**
   `docs/issue17-starter-corpus.md` now states that planned entries have non-null
   `deck_path` values pointing at exact sourced desired lists, that those lists
   are engine-unsupported/non-playable/non-certified, and that `deck_path` is not
   a support claim. Gap 3 now distinguishes desired planning lists from support.
   `evidence/issue17/IMPLEMENTATION-HANDOFF.md` fences the old `deck_path: null`
   paragraph as an assignment-1 historical snapshot and follows it with the
   current state. No stale current-state claim remains.

## Checks actually run

| command | result |
| --- | --- |
| `git rev-parse HEAD`; `git diff --name-status 0a7d781..HEAD` | `86db7d3…`; only `decks.rs`, issue17 doc, implementation handoff, repair/review evidence |
| `cargo test --locked -p aperture-cli` | exit 0; lib 6/6 (both deck-name tests pass), commander_check 8/8, explicit_decks 1/1 |
| `cargo fmt --all -- --check`; `git diff --check` | both exit 0 |
| `cargo build --release --locked -p aperture-cli` | exit 0 |
| `aperture-cli selfplay --seed 1 --max-steps 50` | exit 0; seats `Temur Beats` / `Izzet Control`; status capped at 50, correctly not a victory |
| `aperture-cli replay --file evidence/issue17/review1-selfplay.replay.json` | exit 0; 2364 actions / 6061 events; digest `d7f8c2adaa17…` matches recorded |
| `aperture-cli replay --file evidence/issue17/jasmine-starter-game.replay.json` | exit 0; tracked Jasmine recording still matches the same recorded digest and event count |

## Reused evidence

Review-1's full partial-package inspection remains applicable because this delta
touches no engine/card/list/source content: exact Jasmine 99-card library plus
commander (41 basics, 58 singleton nonlands), four-seat 40-life terminal game,
all four embedded libraries equal to the exact text list, save/replay roundtrip,
analyzer legal/supported with `gameplay.available = false`, planned lists reported
unsupported, and manifest missing-card counts/union (447 = 61 supported + 386
missing) matching the files. Review-1's live Scryfall spot checks also remain
applicable.

## Limitations

* Only the bounded partial increment is approved; seven styles remain planned and
  unsupported. The eight-style issue #17 gate is still open.
* Full suite, full game and network/source matrix were not rerun per assignment;
  exact-head CI is checked separately by the coordinator.
* Native UI is unchanged and still uses its existing bounded-profile setup; the
  starter corpus remains CLI-exercised.
* Untracked coordinator/assignment files in the worktree are outside the reviewed
  tracked delta.
