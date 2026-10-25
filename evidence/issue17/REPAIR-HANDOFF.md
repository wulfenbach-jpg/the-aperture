# Issue 17 — repair handoff (review-1 corrections)

Both blockers from `evidence/issue17/review-1.json` / `review-1.md` are fixed.
The implemented Jasmine package, lists, sourcing and analyzer artifacts are
unchanged. No commit/push; no reviewer reports overwritten.

## 1. Default CLI deck identity

**RED (before production change):**

```
cargo test -q -p aperture-cli non_explicit_default_loads_keep_the_human_fallback_names
  panicked at crates/cli/src/decks.rs:64:9:
  assertion `left == right` failed
    left: "temur-beats"
   right: "Temur Beats"
```

**Fix (minimal):** `crates/cli/src/decks.rs` now uses `file_stem()` only when
`explicit == true`; non-explicit loads keep `fallback_name` exactly as before.
The explicit-starter identity test is retained.

**GREEN:** `cargo test -q -p aperture-cli` -> exit 0, 6 + 8 + 1 tests passed
(including `non_explicit_default_loads_keep_the_human_fallback_names` and
`an_explicit_starter_file_keeps_its_own_identity`).

**Actual default probe** (intentionally capped, not a completed game):

```
target/release/aperture-cli.exe selfplay --seed 1 --max-steps 50 \
  --out evidence/issue17/repair-default-names.json
```

Report: `status capped`, `truncated "step cap of 50 reached before the game
ended"`, seats `[("Temur Beats","decks/temur-beats.txt"),
("Izzet Control","decks/izzet-control.txt")]`. Log:
`evidence/issue17/repair-default-names.log`.

**Explicit path probe** (also capped): 2 seats, explicit
`decks/starter-corpus/jasmine-vanilla-stompy.txt`, two Jasmine commanders.
Seats both `("jasmine-vanilla-stompy", 99 cards, same source)`, commanders
Jasmine. Report `evidence/issue17/repair-explicit-name.json`, log
`evidence/issue17/repair-explicit-name.log`.

**Unchanged Jasmine replay:**

```
target/release/aperture-cli.exe replay --file evidence/issue17/jasmine-starter-game.replay.json
  replay seed 1 — 4 seat(s), 2364 action(s) ...
  replayed digest d7f8c2adaa17 | matches recorded | events 6061
```

Exit 0; log `evidence/issue17/repair-replay.log`.

## 2. Corpus documentation corrected

* `docs/issue17-starter-corpus.md` line ~10 now says planned entries have a
  non-null `deck_path` pointing at an exact sourced **desired** list under
  `decks/starter-corpus/planned/`, and that this is not a support/playability/
  certification claim.
* Policy item 3 now says every target has an exact desired list, explicitly
  engine-unsupported, non-playable and non-certified; Jasmine is the only list
  built entirely from supported cards.
* `evidence/issue17/IMPLEMENTATION-HANDOFF.md` now labels its old corpus-plan
  section as a **historical snapshot superseded by assignment 2**, keeps the old
  text for history, and adds the current schema/status (seven planned lists with
  non-null `deck_path`, 386/447 union names missing, no certification).

## Gates run

| gate | command | result |
| --- | --- | --- |
| focused CLI | `cargo test -q -p aperture-cli` | exit 0 (6 + 8 + 1 passed) |
| fmt + release build | `cargo fmt --all --check && cargo build --release -p aperture-cli` | exit 0; `evidence/issue17/repair-gates.log` |
| default-name probe | `selfplay --seed 1 --max-steps 50` | capped; names `Temur Beats` / `Izzet Control` |
| explicit-name probe | capped 2-seat Jasmine selfplay | names `jasmine-vanilla-stompy`, 99 cards |
| unchanged replay | `replay --file jasmine-starter-game.replay.json` | digest matches, exit 0 |

No full suite was repeated and no other engine/card/list mechanics were edited.

## Changed files

* `crates/cli/src/decks.rs` (explicit-only file-stem naming + default regression)
* `docs/issue17-starter-corpus.md` (planned-list schema/policy correction)
* `evidence/issue17/IMPLEMENTATION-HANDOFF.md` (history labeled superseded +
  current status)
* Added repair evidence: `evidence/issue17/repair-gates.log`,
  `repair-default-names.json/.log`, `repair-explicit-name.json/.log`,
  `repair-replay.log`, this handoff.

`review-1.json`, `review-1.md`, `REVIEW-1.md` and `review1-*` evidence were not
modified. Reporter pointer remains old #20 and untouched.
