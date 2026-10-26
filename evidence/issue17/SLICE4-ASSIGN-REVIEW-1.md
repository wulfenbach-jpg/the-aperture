# Slice 4 independent practical review (bounded)

You are the SAME read-only reviewer history that reviewed the issue-17 corpus
increments. This is a **bounded delta review** of one new commit on the SAME
draft PR46. Do not re-audit the already-approved increments and do not review
any other lane.

## Candidate

* Worktree: `C:/AI/aperture-worktrees/issue-17` (branch `aperture/issue-17`)
* Draft PR: https://github.com/wulfenbach-jpg/the-aperture/pull/46
* Reviewed head: `86aebf9b1acf298a8dd9e0f0c908d2a62bfe63bc`
* Previous approved head: `9cecd62` (approved as a bounded increment)

## What changed (verify it yourself, do not trust this text)

1. `crates/core/src/card.rs`, `catalog.rs`, `state.rs` — `CardDef` gained
   `flash` and `trample`, with `casts_at_instant_timing()` and
   `Object::has_trample()`.
2. `crates/core/src/engine.rs` — three bounded rules changes:
   * Flash: `check_timing` allows a card that prints Flash to be cast in an
     instant-timing window (CR 702.8a); legal-action rule citations follow.
   * Trample: in `deal_combat_damage`, a trampling attacker assigns lethal
     damage to each blocker and the surplus to the defending player
     (CR 702.19b/510.1c); non-trample assignment is unchanged.
   * `enter_battlefield` queues `TriggerEvent::AnotherCreatureEnters` on other
     battlefield permanents (CR 603.6a, printed "another").
3. `crates/core/src/ability.rs`, `catalog.rs` — four new catalog cards:
   Ordinary Bear, Ashcoat Bear (Flash), Owlbear (Trample + enters draw),
   Soul Warden ("another creature enters" → gain 1 life). Pool 93 → 97.
4. `fixtures/starter-corpus/slice4-sourced.json` — live Scryfall records.
5. `crates/core/tests/issue17_slice4.rs` — 10 focused tests.
6. `decks/package-ayula-slice4.txt` — 40-card package fixture (NOT a desired
   starter) and its four-seat journey.
7. `decks/starter-corpus/manifest.json`, `docs/issue17-starter-corpus.md`,
   `README.md` — corpus counts and support statements.
8. `crates/core/tests/catalog_pool.rs`, `card_printings.rs`,
   `issue13_commander_combat.rs` — pool-size and keyword-coverage guards.

## Required checks

Run them, don't read them:

* `cargo test -p aperture-core --test issue17_slice4 --locked`
* `cargo test --workspace --locked`
* `cargo fmt --all --check`
* `cargo clippy --workspace --all-targets --locked -- -D warnings`
* the four-seat journey:
  `cargo run --release --bin aperture-cli -- selfplay --seed 17 --deck decks/package-ayula-slice4.txt` ×4 with
  `--commander-combat-practice` and `--commander "N:Jasmine Boreal"`, then
  `replay --file` on the written replay.

Judge specifically, with your own probes or readings:

* Is the trample assignment correct for one blocker, several blockers, and a
  blocker that already has damage? Does a non-trample attacker still pile all
  damage on the last blocker?
* Does Flash let *only* printed-Flash cards cast at instant timing, and does it
  not let a Flash creature be cast without priority or without mana?
* Does "another creature enters" fire for an opponent's creature (trigger
  controlled by the Warden's controller), never for the Warden itself, and never
  for a permanent that is not a creature?
* Are the four printed oracle strings, costs, type lines and P/T really the live
  Scryfall values, and is each keyword flag backed by a printed keyword?
* Did the eight desired 100-card list files change? (They must not have.)
* Did the docs/manifest claims match what the code and catalog actually do?

Write your verdict to `evidence/issue17/slice4/slice4-review-1.json` and
`slice4-review-1.md` with the fields used in earlier rounds (`review_outcome`:
`approve_increment` or `request_changes`, `reviewed_head`, `findings`,
`checks`). Then STOP.

## Boundaries

Read-only review: no implementation edits, no commits, no merge, no master
push, no issue closure, no new cards or automation. A bounded increment
approval is NOT full issue-17 acceptance: eight certified lists remain the
gate. Keep the response under 60 lines.
