# Issue 16 — counters, tokens, keywords, library operations

Branch `aperture/issue-16` off base `a76df88ce9a0e2ad2bba5f12d6deb9a7a146cfa2`.
Rules revision `2026-09-25` (unchanged). Bounded 28-card practice pool; this is
not full counter/token/keyword coverage and makes no Commander claim.

## What is implemented (and where)

| Mechanic | Rules cite | Code |
|---|---|---|
| Typed counters, canonical storage, +1/-1 deltas | CR 122.1a, 122.3, 122.6, 613.4c | `crates/core/src/counters.rs` |
| Counters on a permanent only; clear on any zone change | CR 122.2 | `Object.counters`, `engine::insert_object`, `engine::move_object_to_stack`, `GameState::validate` |
| Paired +1/+1 and -1/-1 annihilation at every SBA checkpoint | CR 704.5q | `Counters::annihilate`, `engine::check_sbas` |
| Counters change power/toughness, combat and SBA | CR 509/510, 704.5f, 704.5g | `Object::power`/`toughness` (no combat code touched) |
| Counter visibility | CR 122.1 | `view::CardView::counters` |
| Token creation, ownership/control, summoning sickness | CR 111.1–111.3, 302.6 | `crates/core/src/tokens.rs`, `GameState::create_token` |
| Token cessation (never graveyard/hand/library/exile/stack) | CR 111.7 | `engine::insert_object`, `move_to_graveyard`, `move_to_hand`, `move_to_exile`, `move_to_top_of_library` |
| Keywords enforced: flying/reach blocking, defender attack | CR 702.9b, 702.17b, 702.3b, 509.1b | `tokens::can_block`, `validate_declare_blockers`, `legal_attackers`, `sample_blocks` |
| Mill, shuffle, reveal, tap/untap | CR 701.13a, 701.20a, 701.16a, 701.26a | `crates/core/src/library.rs` |
| Revealed hidden-zone cards are public, nothing else leaks | CR 401.2, 701.16a | `GameState.revealed`, `GameState::revealed_hidden`, `view::PlayerView::revealed` |
| Pre-#16 v10 saves still load; unchanged games keep their digest | save/replay format | `Object.counters` / `GameState.revealed` serde attributes, `Counters` deserializer |

### engine.rs surface (for the #15 rebase)

`engine.rs` is +133/-12 against the base and touches **no**
replacement/prevention/layered-effect resolution path. The hunks are:

1. `insert_object`: token cessation early-return (CR 111.7) and
   `object.counters.clear()` (CR 122.2).
2. `move_to_top_of_library` / `move_to_hand` / `move_to_exile`: truthful
   "token ceases to exist" events instead of a zone move.
3. `move_to_graveyard`: tokens log cessation instead of `Destroyed`; the
   `Dies` trigger is still queued (CR 700.4), as before.
4. `legal_attackers`: defender filter.
5. `validate_declare_blockers`: block-pair evasion check.
6. `sample_blocks`: never offer an illegal block pair.
7. `check_sbas`: the CR 704.5q annihilation block, inside the existing SBA loop
   and before the lethal-damage sweep. Removing equal numbers of both kinds
   leaves power/toughness unchanged, so it neither causes nor prevents a CR
   704.5f/704.5g death.
8. `move_object_to_stack`: `counters.clear()` beside the existing pump resets, so
   no transition can carry typed counters onto the stack.
9. `Effect::ExileAndGainLife` (Swords to Plowshares): the effect line says
   "ceases to exist instead of being exiled (CR 111.7)" when the target was a
   token, instead of contradicting the cessation event.

## Review 183 corrections (each fixed with a focused test)

* **R183-1 save compatibility — fixed, with a real pre-#16 fixture.**
  `Object.counters` now carries `#[serde(default, skip_serializing_if)]` exactly
  like `revealed`, so an absent field defaults to no counters. New test
  `a_real_pre16_v10_save_still_loads_and_replays` loads
  `crates/core/tests/fixtures/continuity/pre16-v10-save.json` through
  `SessionSave::from_json` — a genuine save written by the base commit `a76df88`
  through its own save path (provenance in
  `evidence/issue16/pre16-save-provenance.txt`: 45 recorded steps, 132 logged
  events, digest `282eea63…39e80`) — and asserts counters/revealed default
  empty, the state validates, no data is lost, and the recorded replay still
  reproduces the recorded digest.
  The digest basis did not move either: empty `counters`/`revealed` are skipped
  when serializing, so an unchanged game keeps the base serialized form. Base
  `aperture-cli selfplay --seed 4242` and this branch both report
  `64858f984f19` (315 steps, 14 turns, P0). `SAVE_VERSION` therefore stays 10 and
  `REPLAY_VERSION` stays 6: v10 saves still load and v6 replay digests still
  verify. A malformed counters payload is a load error instead of a silent
  repair (`a_save_with_malformed_counters_is_refused_not_coerced`: zero-count
  kind, unknown kind, negative count, wrong JSON type), because the `Counters`
  deserializer enforces the canonical form the engine writes.
* **R183-2 CR 704.5q — implemented.** `Counters::annihilate` removes the smaller
  count of each kind; `check_sbas` applies it at every checkpoint and logs a
  cited SBA event. Test
  `paired_plus_and_minus_counters_annihilate_at_the_next_checkpoint`: a 3/3 with
  2 +1/+1 and 1 -1/-1 keeps its transient 4/4 read and then persists as
  `1 +1/+1`; a 1/1 that gained +1/+1 then -1/-1 ends with no counters and is
  still on the battlefield. The projection test no longer blesses coexistence: it
  asserts a single-kind permanent and that a checkpoint leaves it alone.
* **R183-3 Swords to Plowshares on a token — fixed.** Test
  `swords_to_plowshares_reports_a_token_truthfully`: no event claims the token
  "is exiled"; the effect line states the cessation and the 1-life gain, and P1
  ends at 21.

Recorded, not fixed (as requested): no player counters exist (see limitations);
`move_object_to_stack` now clears `counters` like the other transition paths.

## Bounded scope and honest limitations

* No card in the 28-card pool creates a counter, creates a token, prints a
  keyword, mills, shuffles a library, reveals from a library or untaps a
  permanent. The mechanics are therefore driven through the rules primitives
  (`GameState::create_token`, `Object::counters`, `GameState::mill`,
  `GameState::reveal_top`) and then executed by the **real engine** with real
  pool cards (Unsummon, Shock, Swords, combat, SBA), not by mocks.
* Because no card produces them, the new events that appear in real games are
  cessation/SBA messages, not creation messages. Creating a counter/token from
  card text needs the corpus work in #17 (a pool card may only ever be added
  with its printed text).
* **No player counters.** The issue text names "object and player counters"; only
  object counters are typed and stored. Nothing in the supported pool puts a
  counter on a player (no poison, experience, energy or commander-tax counter is
  modelled), so player counters are deliberately absent rather than guessed.
* `GameState::mill` stops before a commander card rather than silently moving
  it: the CR 903.9a owner choice belongs to the engine's replacement flow.
* `Object.plus_one_counters` (UI sandbox display) is unchanged and remains
  storage-only; the rules layer reads `Object.counters` exclusively. The UI was
  not touched.
* Counter kinds are exactly +1/+1 and -1/-1. No other counter kind is modelled.

## Executed verification (actual output)

```
cargo test -p aperture-core --test counters_tokens_library --locked
  -> test result: ok. 16 passed; 0 failed; 0 ignored
cargo test -p aperture-core --lib counters --locked
  -> test result: ok. 5 passed; 0 failed (2 filtered out)

cargo test --workspace --locked
  -> 67 test binaries, 434 passed, 0 failed   (428 at the previous head)

cargo fmt --all --check        -> PASS
cargo clippy --workspace --all-targets --locked -- -D warnings -> PASS
cargo build --workspace --release --locked -> Finished `release` profile in 33.18s

target/release/aperture-cli harness --seats 2,3,4 --seed-base 4242 --games-per-seat 2
  -> 12 run(s) — terminal 12, capped 0, failed 0
target/debug/aperture-cli selfplay --seed 4242
  -> digest 64858f984f19 | replay verified | casts 9 attacks 3 blocks 0 counters 0
     (identical to the base commit's run: the digest basis did not move)
```

The 16 focused tests cover: counter characteristics and public projection;
-1/-1 killing a 1/1 through CR 704.5g; CR 704.5q annihilation with the surplus
kept and a balanced 1/1 pair cleared; counters not following a card out of the
battlefield (real `Unsummon`); combat damage from a counter-modified power (real
combat, 3 damage from a 2/2 with one +1/+1 counter); token
characteristics/ownership/visibility; flying blocked only by flying/reach (and
`legal_actions` never offering an illegal pair); a `Shock`-killed token ceasing
to exist with no graveyard entry; a bounced token ceasing with no hand entry;
mill/shuffle/reveal/untap determinism and visibility; a real seed-4242 practice
game keeping its digest and round-tripping through save/load; counters plus a
token surviving save/load exactly; an illegal save (counters in a graveyard)
being refused; a genuine pre-#16 v10 save loading, validating and replaying; a
malformed counters payload being refused; and Swords to Plowshares reporting a
token's cessation truthfully.

## Not done here

* No merge, no master push, no issue closure, no successor activation.
* The issue's "required starter-corpus counters" / "keyword set required by
  certified decks" cannot be exercised from card text until #17 lands cards that
  use them; that gap is disclosed above rather than papered over with a new
  card invented for this branch.
