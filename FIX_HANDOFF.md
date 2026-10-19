# FIX_HANDOFF — supervisor acceptance fixes for THE APERTURE

Baseline: commit `265fd19` (109 tests, fmt/clippy/release clean). This round
closes the seven verified acceptance blockers below. Scope was deliberately kept
to those blockers; nothing was rebuilt from scratch, no other repo/config/model
service was touched, and the local Decider-2B service was never restarted.

Evidence for every RED→GREEN cycle: `evidence/supervisor-fixes-tdd.log`.
Runtime reports: `evidence/fix-*.json`, `evidence/screenshots/ui-supervisor-fixes.png`,
`evidence/screenshots/ui-import-refusal.png`.

## Result summary

| gate | result |
| --- | --- |
| `cargo fmt --check` | 0 — clean |
| `cargo test --all-targets` | 0 — **134 passed / 0 failed** (was 109; 14 test binaries) |
| `cargo clippy --all-targets -- -D warnings` | 0 — clean |
| `cargo build --release` | 0 |
| `selfplay --seed 101` | 0 — P0 wins, 452 steps, digest `233431d963e5`, replay verified |
| swapped-deck seeds 11,22,33,44,55,66,77,88 | 0 — 8/8 terminal, replay verified, **P1 6 wins / P0 2 wins** |
| live Archidekt import 26206384 | 0 — 28 unsupported with TRUE reasons (15 multi-face, 13 not-in-pool), launch refused |
| real Decider-2B probe (seed 21, seat 0, 120 steps) | 0 — **13 decider-2b decisions, clean usage, 0 truncation fallbacks** |
| native `--smoke` screenshots | 0 — board + refusal-banner/import-panel captures |

## 1. Archidekt normal cards were falsely unsupported — FIXED

`crates/import/src/lib.rs::classify` rejected every entry with `faces == []`.
A normal single-face card legitimately has `layout: "normal"` and an empty
`faces` array with its metadata (name/layout) on the top-level `oracleCard`.
Now: an entry is only "no provider card data" when the *name* is empty; a
multi-face/meld layout (`faces.len() > 1 || layout != "normal"`) is still
refused as not modelled; otherwise the name is looked up in the pool.

The real deck 26206384 (28 cards) is still unsupported, now with the true
reasons: 15 multi-face (including the two `meld` entries that arrive with empty
`faces`) and 13 ordinary cards "not in the supported pool (15 cards)".

Regression: `crates/import/tests/normal_cards.rs` + a synthetic fixture that
reproduces the real provider shape (`layout: "normal"`, `faces: []`) for a
fully-supported 40-card deck — it now certifies and plans. `normalize_real.rs`
asserts the real reasons (the old "no card data" assertion was provably invalid
and was updated with a comment saying why).

## 2. URL slugs + silent quantity clamping — FIXED

* `parse_deck_url` accepts the canonical `/decks/<digits>/<human_slug>` form
  (and the `/api/decks/…` form) with optional query/fragment; the slug is
  non-authoritative and the request URL is still rebuilt from the numeric id
  against the allowlist. Rejections kept: non-https, non-allowlisted host,
  credentials, ports, `.`/`..`/`%` segments, >1 extra segment.
* `normalize` no longer clamps to `u8` or skips zero: a quantity that is
  missing, non-integer, `0` or `> 255` is a clear `ImportError::Parse` naming
  the value; valid values are retained exactly.

Regressions in `crates/import/tests/url_guard.rs` and `normal_cards.rs`. The old
`'/decks/26206384/extra' is rejected` assertion was invalid under the new (real)
link form and was moved to the accepted list with a comment.

## 3. UI import did not apply a deck — FIXED (vertical path complete)

`crates/ui/src/session.rs` + `app.rs`:

* Two explicit seats (P0 `your seat` / P1 `AI seat`), each with an Archidekt URL
  box, a pasted-text fallback **and** a text-file fallback.
* Each import is certified independently and shows name, supported/total counts,
  the practice-profile line, the refusal reason and the unsupported cards.
* `Start imported match (P0 vs P1)` is enabled **only when both seats are
  certified**. It applies the exact planned `DeckList`s (no substitutions) and
  calls `new_game`, so the engine state and the replay both embed the two lists
  as snapshots and the log/replay restart from revision 0.
* An explicitly requested `--deck-a/--deck-b` file that is missing or invalid is
  refused loudly (`load_explicit_deck`); the bundled deck is never substituted.
  Built-in default loading uses the compiled practice bundle.
* Missing/invalid explicit deck and two-seat coverage in
  `crates/ui/tests/import_and_async.rs` (two distinct lists enter game and
  replay; an unsupported seat cannot start; text fallback for both seats).

## 4. UI blocked on network — FIXED (all work off the frame callback)

`poll_workers` drains an mpsc channel; every model/network/file operation runs on
a `std::thread` and returns a proposal, never a mutation. The engine stays
authoritative on the main thread.

* AI: `request_ai_decision{,_default}` captures **generation + engine revision +
  decision serial + seat**; a result is applied only if all four still match.
  Generation is bumped by `invalidate_pending_work` on new game/load/branch
  return/sandbox/import start/settings change — including a new game with the
  SAME seed and digest, which only the generation token can distinguish.
* Import, health, **save and load**: background workers with busy flags and
  spinners. `request_load` parses on the worker and applies the snapshot on the
  main thread; a load that completes after the session changed is discarded as
  stale. `poll_workers` continues to be scheduled (and repaints requested) even
  after the game is over, so an import/health/save/load started on the final
  position still completes visibly.
* `--smoke` disables the decider for the entire capture (not just the warm-up)
  even when `--decider` is passed, and prints a note; a smoke run can neither
  fire model calls nor show a misleading "opt-in ON".
* Tests use injected delayed work (a closure blocked on a test channel): poll
  returns promptly (< 500 ms asserted) while the worker is blocked, and after the
  game is replaced with the same seed the released stale completion is discarded
  with the game untouched (digest/revision/pending/replay all unchanged). A
  fresh completion still reaches the engine.

## 5. Wrong seat life labels in the Decider request — FIXED

`decision_request` keyed `state.life` with a literal `{"P0": own_life, "P1":
opponent_life}`, correct only for a P0 viewer (so a P1 viewer was told its own
life was 3 when it was 17). Now keyed by the real absolute seats. Regression
`crates/ai/tests/decider_usage.rs` (viewer P1, P0 = 3, P1 = 17).

## 6. Truncated model responses were accepted — FIXED

`DeciderClient::choose` now refuses a response whose `usage` indicates
truncation (`truncated == true`, `state_tokens_dropped > 0`, or a non-empty
`truncated_questions`), via a new `DeciderError::Truncated`, before the choice is
resolved; `DeciderPolicy` then emits a visible heuristic fallback. HTTP status,
missing `answers.decision`, missing `choice`, unknown key and unparsable-body
handling are unchanged.

Live consequence found while probing: at the old 512-token budget the service
truncated **every** request (`state_tokens_dropped = 54..184`), so the model was
effectively unusable. `DeciderConfig::default().max_len` is now 1024 (documented,
inside the service bound), which yields 13 genuine model answers with clean usage
and zero fallbacks in the re-probe. Loopback regression servers cover the
truncated and the healthy path (`crates/ai/tests/decider_usage.rs`).

## 7. Rule coaching + operator-control leak — FIXED

* `RULE_NOTES["510.1c"]` no longer teaches "lethal first"; it now states the
  pinned rule (2026-09-25): a blocked creature's controller divides its damage
  among the blockers as they choose, with no lethal-first requirement. `510.1d`
  added. `coach::explain` emits a `combat damage assignment` disclosure that the
  engine's automatic lethal-first assignment is a bounded slice simplification,
  and the engine comment says the same. The narrower behavior is documented in
  README and HANDOFF rather than exposed as a typed assignment UI (the "modest"
  option was not taken; this is the explicitly allowed documented route).
* Pending-choice controls render only for the seat that owns the decision; other
  seats see "WAITING ON THE OTHER SEAT" plus an AI-step button. `submit_human`
  refuses any UI action whose decision belongs to another seat ("not your seat"),
  so the UI can never act as the opponent via `owner()`.

## Remaining scope / honest limits

* No typed combat damage assignment UI (see 7) — automatic lethal-first only.
* Commander, multiplayer, triggered/activated/static abilities, tokens,
  planeswalkers, layers, counters in rules math: still absent (README §"NOT
  implemented").
* `--deck-a/--deck-b` loading is startup-time only; fixing a refused deck is done
  through the Import panel.
* The two-seat import/play path was verified at the session layer and by the
  refusal-banner smoke capture; a human should still click the import boxes once
  in the real window.
* Nothing here claims upstream engine parity, search strength, or full Commander
  support.

## Independent review round

A read-only `reviewer` task agent (`FixReview`) reviewed the whole change set
(diff + untracked tests). It confirmed the import, decider, coaching and
staleness work and raised three points, all now fixed with regressions:

1. **Save/Load still did blocking file IO in the frame callback** (requirement
   gap). Fixed: `UiSession::request_save` / `request_load` run the write/read on
   a worker and apply the parsed snapshot on the main thread; a load that
   completes after the session changed is discarded as stale. Tests:
   `save_and_load_run_on_workers`,
   `a_load_completed_after_the_game_changed_is_discarded`.
2. **Worker polling stopped when the game ended** — `tick_ai` returned early, so
   an import/health job started on a finished game was never polled promptly.
   Fixed: `poll_workers` plus the worker-busy repaint request moved out of
   `tick_ai` into the frame body, which always runs.
3. **`--smoke --decider` could still make model calls during the capture frames**
   (the warm-up was heuristic-only but the flag was restored afterwards). Fixed:
   the decider is disabled for the entire smoke run and a note is printed.

The same review noted, correctly, that the URL guard still accepts an explicit
`:443` port. That is intended and pre-existing (`443` is the https default; the
blocker asked to *restrict* ports, and `:8443` etc. remain rejected), so no
change was made.
