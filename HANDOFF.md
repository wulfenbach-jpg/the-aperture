# HANDOFF — THE APERTURE (practice slice)

Scope delivered: an original Rust rules engine, a headless CLI, and a native
desktop card table for a 1v1 Magic: The Gathering **practice slice** — 15-card
implemented pool, 20 life, 40-card decks, London mulligan. Commander and
everything outside the pool is refused, never faked. Pinned rules revision:
**Comprehensive Rules 2026-09-25** (`RULES_REVISION`).

Every number below comes from a command that was actually run; raw output is in
`evidence/` (`verification.log`, `tdd.log`, `tdd-import-red.log`,
`tdd-ai-red.log`, the per-seed JSON reports, and the screenshot).

## 1. Contents

```
Cargo.toml / Cargo.lock        workspace, 5 crates, resolver 3, pinned deps
crates/core      aperture-core   rules engine + 11 integration test files
crates/import    aperture-import Archidekt URL guard/fetch/cache/normalize/report + 4 test files
crates/ai        aperture-ai     heuristic policy, Decider-2B adapter, self-play + 5 test files
crates/cli       aperture-cli    selfplay, replay, import, import-txt, pool, decks, coach,
                                 decider-health
crates/ui        aperture-ui     eframe/egui card table (bin `aperture`) + lib + 1 test file,
                                 `--smoke` framebuffer capture
decks/temur-beats.txt            bundled 40-card deck A (Temur Beats)
decks/izzet-control.txt          bundled 40-card deck B (Izzet Control)
README.md  HANDOFF.md  launch.cmd  verify.cmd  .gitignore
evidence/  cache/                reports, logs, screenshot; cache is git-ignored
```

54 Rust files. Nothing outside `C:/AI/mtg-slice` was created or modified; the
only toolchain change was `rustup component add clippy` (the standard component,
needed for the required clippy gate).

## 2. Verification (all commands run in `C:/AI/mtg-slice`)

| command | exit | result |
| --- | --- | --- |
| `cargo fmt --check` | 0 | clean |
| `cargo test --all-targets` | 0 | **107 passed / 0 failed** across 26 result lines (11 test binaries + lib/doc targets) |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo build --release` | 0 | `target/release/aperture.exe`, `target/release/aperture-cli.exe` |
| `aperture-cli selfplay --seed 1..8` | 0 | 8/8 terminal games, every digest replay-verified |
| `aperture-cli replay --file evidence/replay-seed6.json` | 0 | `548 action(s) … matches recorded: true, events 1577` |
| `aperture-cli import --url https://archidekt.com/decks/26206384` | 0 | honoured report, launch refused |
| `aperture-cli decider-health` | 0 | loopback service reachable |
| `aperture.exe --smoke evidence/screenshots/ui.png` | 0 | real window rendered, captured, exited |

Full log: `evidence/verification.log`.

## 3. Self-play (heuristic vs heuristic, bundled decks, final binaries)

| seed | winner | steps | turns | digest | casts | attacks | blocks | counters | truncated |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | P0 (A) | 598 | 25 | b823599d4b73 | 23 | 5 | 2 | 0 | no |
| 2 | P0 (A) | 425 | 18 | 6065dab7e738 | 14 | 5 | 0 | 1 | no |
| 3 | P0 (A) | 285 | 13 | d3b7b9f9a54d | 6 | 4 | 0 | 1 | no |
| 4 | P0 (A) | 330 | 15 | d4ea5ed1674b | 5 | 4 | 1 | 0 | no |
| 5 | P0 (A) | 438 | 18 | 46e264d70b08 | 18 | 6 | 1 | 1 | no |
| 6 | P0 (A) | 548 | 22 | eaf15318b35e | 26 | 10 | 1 | 2 | no |
| 7 | P0 (A) | 556 | 23 | 8ba1ce99c5c7 | 22 | 8 | 1 | 2 | no |
| 8 | P0 (A) | 419 | 19 | 9e0b5f6c0751 | 9 | 3 | 0 | 2 | no |

Every run reports `replay_digest_matches: true`; `truncated` is `null`. Deck A
wins all eight — a property of this shallow policy in this matchup, not a
strength claim. Capped runs are reported as `TRUNCATED` with a reason and never
as a winner (test `a_run_cap_is_reported_as_truncation_not_a_result`, plus a
state-repetition guard).

## 4. Live Archidekt import (real HTTPS, this machine)

`https://archidekt.com/decks/26206384` → `evidence/import-live.json`:

* 28 card entries / 28 cards: **0 supported, 28 unsupported**, launch refused
* 15 entries had no provider card data and are listed as unsupported, not guessed
* 13 multi-face entries (transform/flip/split/meld/modal) refused as not modelled
* excluded category honored: Maybeboard; provider format code 7 recorded as
  metadata only (this deck has no commander category, and none was invented)
* response cached: `cache/archidekt-26206384.json` (+ `.normalized.json`, `.report.json`)
* the sha256 in the report is of *that fetch*: two fetches seconds apart differ
  only in `viewCount` (139 → 140, verified), so the response is not byte-stable

Text fallback: `aperture-cli import-txt --file decks/temur-beats.txt` → 40/40
supported, playable.

## 5. Local Decider-2B adapter — real tested status

The service was **already running** on `http://127.0.0.1:8765`; nothing was
started, stopped, restarted or reconfigured. `aperture-cli decider-health`
reports `accepted: true, reachable: true`.

Bounded real probe (150 decisions, seed 21, seat 0, `evidence/decider-probe.json`):

```
decision_sources: { "decider-2b": 12, "forced (single legal option)": 65, "heuristic": 73 }
policy_notes: [ "model decider-2b chose Keep hand (7 cards) (preference 0.600; not a win probability)",
                "model decider-2b chose Play Forest (preference 0.412; …)",
                "model decider-2b chose Tap Forest for {G} (preference 0.708; …)" ]
```

* 12 genuine model answers; the 73 `heuristic` decisions are the *other* seat,
  which is heuristic by design. **Zero** fallbacks in this probe.
* Request shape: `POST /v1/systemone` with `state`, one `choice` question with
  named `criteria`, `model: "decider-2b"`, `max_len`; the answer is read from
  `answers.decision.choice` + `confidence`/`answer_confidence`. Only the masked
  projection of the acting seat is sent (hidden zones absent — tested).
* **Live finding, now fixed:** the typed readout rejects a one-option choice with
  `HTTP 422 "Choice decision requires 2 to 100 named criteria"`. The adapter now
  treats a single legal action as `forced (single legal option)` and makes no
  call, instead of reporting a bogus fallback. Regression test
  `decider::a_single_legal_option_is_forced_and_never_asked_of_the_model`.
* Confidence is labelled preference concentration everywhere; it is never a win
  probability.
* Redirects are disabled and environment proxies are ignored on this client
  (`.redirect(Policy::none())`, `.no_proxy()`), so a hostile loopback endpoint
  cannot replay the masked payload elsewhere (regression test
  `decider_redirect::the_decider_client_does_not_follow_a_redirect`).

## 6. Independent reviews and what they changed

Two task-agent reviews ran against the finished code; every finding was addressed
with a regression test (all in `crates/core/tests/review_regressions.rs` unless
noted, plus `evidence/tdd.log` recording the observed RED).

Rules/engine review (4 high, 3 medium):

1. **Combat with departed creatures (high)** — a creature killed before combat
   damage still dealt/absorbed damage, and a dead attacker could still be
   blocked. Fixed with `Turn.blocked` (CR 509.1h), `Game::prune_combat`
   (CR 506.4) and a blocked-attacker rule that assigns nothing to the player when
   every blocker has left. Tests: `a_destroyed_attacker_assigns_no_combat_damage`,
   `a_blocker_that_died_absorbs_nothing_and_the_attacker_stays_blocked`,
   `a_dead_attacker_cannot_be_blocked`.
2. **Private scry/surveil leaked through the log (high)** — `Event::OrderedCard`
   now carries owner + mode; only the owner (or a card that actually reached a
   public graveyard) sees the name. Test:
   `the_opponent_does_not_learn_a_private_scry_or_surveil_choice`.
3. **Replay seed rendered into player logs (high)** — with public deck lists the
   seed reconstructs both libraries, so it is no longer rendered. Test:
   `the_game_start_log_does_not_reveal_the_replay_seed`.
4. **Coaching ignored the viewer's boundary (high)** — `coach::explain(game,
   viewer, legal)` now describes a hand only for the seat that owns the decision.
   Test: `coaching_never_describes_another_seats_hand`.
5. **First player did not skip their first draw step (medium, CR 103.8a)** —
   `GameState.first_player` added; test
   `the_player_who_plays_first_skips_their_first_draw_step`.
6. **Unanswerable mulligan deadlock (medium)** — a mulligan that could never be
   bottomed is refused and only "keep" is offered; test
   `a_mulligan_that_could_never_be_bottomed_is_refused`.
7. **Mana ability accepted during a pending choice (medium, CR 605.3a)** — now
   requires priority and no pending choice; test
   `a_mana_ability_is_refused_while_a_choice_is_pending`.

Security review (1 medium, 1 low):

8. **Decider client followed redirects (medium)** — masked payload could be
   replayed to another host. Fixed (`Policy::none()`), regression test as above.
9. **Import client honoured `HTTP(S)_PROXY`/`ALL_PROXY` (low)** — fixed with
   `.no_proxy()` on the import client; the allowlist claim now holds in code.

The UI session test also caught a real bug: `UiSession::save` wrote the AI
settings from the replay's construction rather than the live settings, so
decider/autoplay flags were lost on load. Fixed.

`SAVE_VERSION` is now 2 (the state shape gained `first_player` and
`Turn.blocked`); old save files are refused with a clear message.

## 7. Supported pool (all printed abilities implemented)

Island, Mountain, Forest, Grizzly Bears, Runeclaw Bear, Centaur Courser,
Craw Wurm, Hill Giant, Giant Growth, Shock, Lightning Strike, Counterspell,
Divination, Opt, Consider — `aperture-cli pool` prints this list plus the
not-implemented notice.

## 8. How to run

```
launch.cmd                                  builds if needed, starts the card table
cargo run --release -p aperture-ui          same thing by hand
cargo run --release -p aperture-cli -- pool
cargo run --release -p aperture-cli -- selfplay --seed 1 --verbose
cargo run --release -p aperture-cli -- replay --file evidence/replay-seed1.json
cargo run --release -p aperture-cli -- import --url https://archidekt.com/decks/26206384 --plan
cargo run --release -p aperture-cli -- decider-health
target/release/aperture.exe --smoke evidence/screenshots/ui.png
verify.cmd                                  the whole verification battery
```

UI flags: `--seed N --deck-a <txt> --deck-b <txt> --ai-seat 0|1 --autoplay
--ai-both --decider --smoke OUT.png`.

## 9. Native UI: what was verified, and what the supervisor should still click

* `--smoke` opened a real 1750×1175 window (1400×940 logical at 1.25 DPI), played
  the game forward to a developed board, captured the actual framebuffer through
  `egui::Context::request_screenshot` and exited cleanly.
* The PNG is the raw framebuffer, straight RGBA, fully opaque (every pixel α=255).
  Sampled pixels: panel `(20,21,24)`, card faces `(36,37,42)`, muted text
  `(154,149,138)` — the dark charcoal/brass palette. **Caveat:** the image preview
  in this session displayed it inverted (light); the decoded bytes above are the
  truth, so decode the PNG if the preview looks wrong.
* Interaction (clicking action buttons, the target picker, the sandbox toggle,
  the import panel, attacker/blocker multi-select, save/load buttons) was
  exercised by tests at the session layer
  (`crates/ui/tests/session.rs`) but not by an automated click pass — a human
  should click through those by hand.

## 10. NOT IMPLEMENTED (exact limits)

* Commander in any form, multiplayer pods, sideboard/companion play, and any
  format profile other than 1v1 / 20 life / 40 cards / London mulligan.
* Triggered abilities, activated abilities other than basic-land mana abilities,
  static abilities, replacement/prevention effects, tokens, planeswalkers,
  battles, milling/tutoring/searching, library manipulation beyond scry/surveil
  1, multi-face cards.
* Counters are stored and displayed but do not affect rules math
  (`Object.plus_one_counters`); no layers — a single until-end-of-turn pump only.
* Object identity is stable across zone changes (real Magic creates a new object
  per zone change); unobservable in this pool but relevant before scaling cards.
* Damage assignment for a multi-blocked attacker is automatic (lethal-first in
  object-id order); no manual damage-order UI.
* Cleanup-step priority exception (CR 514.3a) is not implemented; the cleanup
  step never grants priority. Draw-from-empty is a state-based action and is
  implemented (CR 104.3c/704.5b).
* Fewer than two legal actions never consult the model (forced); more than 90
  legal actions in one decision are refused rather than truncated.
* Sandbox tools (draw/mill/reveal/+1/+1 storage) bypass the rules by design and
  permanently mark the session `SANDBOX-MODIFIED`; certified replay claims end
  there.
* No undo; branching is done by loading a save or re-running a replay.
