# Issue 16 — counters, tokens, keywords, library operations

Branch `aperture/issue-16` off base `a76df88ce9a0e2ad2bba5f12d6deb9a7a146cfa2`.
Rules revision `2026-09-25` (unchanged). Bounded 28-card practice pool; this is
not full counter/token/keyword coverage and makes no Commander claim.

## What is implemented (and where)

| Mechanic | Rules cite | Code |
|---|---|---|
| Typed counters, canonical storage, +1/-1 deltas | CR 122.1a, 122.3, 122.6, 613.4c | `crates/core/src/counters.rs` |
| Counters on a permanent only; clear on any zone change | CR 122.2 | `Object.counters`, `engine::insert_object`, `GameState::validate` |
| Counters change power/toughness, combat and SBA | CR 509/510, 704.5f, 704.5g | `Object::power`/`toughness` (no combat code touched) |
| Counter visibility | CR 122.1 | `view::CardView::counters` |
| Token creation, ownership/control, summoning sickness | CR 111.1–111.3, 302.6 | `crates/core/src/tokens.rs`, `GameState::create_token` |
| Token cessation (never graveyard/hand/library/exile/stack) | CR 111.7 | `engine::insert_object`, `move_to_graveyard`, `move_to_hand`, `move_to_exile`, `move_to_top_of_library` |
| Keywords enforced: flying/reach blocking, defender attack | CR 702.9b, 702.17b, 702.3b, 509.1b | `tokens::can_block`, `validate_declare_blockers`, `legal_attackers`, `sample_blocks` |
| Mill, shuffle, reveal, tap/untap | CR 701.13a, 701.20a, 701.16a, 701.26a | `crates/core/src/library.rs` |
| Revealed hidden-zone cards are public, nothing else leaks | CR 401.2, 701.16a | `GameState.revealed`, `GameState::revealed_hidden`, `view::PlayerView::revealed` |

### engine.rs surface (for the #15 rebase)

`engine.rs` is +93/-6 and touches **no** replacement/prevention/layered-effect
resolution path. The hunks are:

1. `insert_object`: token cessation early-return (CR 111.7) and
   `object.counters.clear()` (CR 122.2).
2. `move_to_top_of_library` / `move_to_hand` / `move_to_exile`: truthful
   "token ceases to exist" events instead of a zone move.
3. `move_to_graveyard`: tokens log cessation instead of `Destroyed`; the
   `Dies` trigger is still queued (CR 700.4), as before.
4. `legal_attackers`: defender filter.
5. `validate_declare_blockers`: block-pair evasion check.
6. `sample_blocks`: never offer an illegal block pair.

## Bounded scope and honest limitations

* No card in the 28-card pool creates a counter, creates a token, prints a
  keyword, mills, shuffles a library, reveals from a library or untaps a
  permanent. The mechanics are therefore driven through the rules primitives
  (`GameState::create_token`, `Object::counters`, `GameState::mill`,
  `GameState::reveal_top`) and then executed by the **real engine** with real
  pool cards (Unsummon, Shock, combat, SBA), not by mocks.
* Because no card produces them, the new events that appear in real games are
  cessation/SBA messages, not creation messages. Creating a counter/token from
  card text needs the corpus work in #17 (a pool card may only ever be added
  with its printed text).
* `GameState::mill` stops before a commander card rather than silently moving
  it: the CR 903.9a owner choice belongs to the engine's replacement flow.
* `Object.plus_one_counters` (UI sandbox display) is unchanged and remains
  storage-only; the rules layer reads `Object.counters` exclusively. The UI was
  not touched.
* Counter kinds are exactly +1/+1 and -1/-1. No other counter kind is modelled.

## Executed verification (actual output)

```
cargo test -p aperture-core --test counters_tokens_library --locked
  -> test result: ok. 12 passed; 0 failed; 0 ignored

cargo test --workspace --locked
  -> passed 428 failed 0   (was 391 before this branch's 12 new tests)

cargo fmt --all --check        -> PASS
cargo clippy --workspace --all-targets --locked -- -D warnings -> PASS
cargo build --workspace --release --locked -> Finished `release` profile in 3m 05s
```

The 12 focused tests cover: counter characteristics and public projection;
-1/-1 killing a 1/1 through CR 704.5g; counters not following a card out of the
battlefield (real `Unsummon`); combat damage from a counter-modified power
(real combat, 3 damage from a 2/2 with one +1/+1 counter); token
characteristics/ownership/visibility; flying blocked only by flying/reach (and
`legal_actions` never offering an illegal pair); a `Shock`-killed token ceasing
to exist with no graveyard entry; a bounced token ceasing with no hand entry;
mill/shuffle/reveal/untap determinism and visibility; a real seed-4242 practice
game keeping its digest and round-tripping through save/load; counters plus a
token surviving save/load exactly; and an illegal save (counters in a graveyard)
being refused.

## Not done here

* No merge, no master push, no issue closure, no successor activation.
* The issue's "required starter-corpus counters" / "keyword set required by
  certified decks" cannot be exercised from card text until #17 lands cards that
  use them; that gap is disclosed above rather than papered over with a new
  card invented for this branch.
