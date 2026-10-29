# Issue 17 slice 2 assignment 2 — token repair handoff

Both observed token defects were fixed test-first. The Krenko package remains
implemented; the exact 100-card Krenko list remains `planned`. Talrand was **not**
registered (see below). No commit/push/merge/closure. No #16 integration claim.

## Defect 1 — Krenko's X was silently clamped at 255

**RED (before production edit):**

```
cargo test -q -p aperture-core --test issue17_tokens krenko_never_silently_truncates_a_goblin_count_above_255
  assertion failed: the printed X is not clamped
    left: 255
   right: 301
```

The regression stages 300 other Goblin permanents plus Krenko, activates the real
`{T}` ability through normal stack resolution, and requires 301 new tokens.

**Fix:** `goblin_count` and `create_goblin_tokens` now use `usize`; the
`.min(u8::MAX)` clamp was deleted. Fixed-count spells still pass their printed
`u8` count through `usize::from`, so Dragon Fodder/Krenko's Command/Hordeling
Outburst are unchanged.

**GREEN:** 301 new tokens, 602 Goblins total (`issue17_tokens` 10/10).

## Defect 2 — token cessation was a replacement effect

**RED (before production edit):**

```
token_death_is_a_real_graveyard_change_then_cessation
  "the token really dies (CR 700.4)" failed
  events: ... StateBasedAction "Goblin token ceases to exist" ...
          StateBasedAction "1 creature(s) ... put into the graveyard"

token_bounce_and_exile_record_the_destination_before_cessation
  "bounce records the hand zone change" failed
  events: ... StateBasedAction "Goblin token ceases to exist" ... Resolve Unsummon ...
```

**Fix (CR 111.7/111.8/700.4):** removed `cease_token_if_needed` interception from
`move_to_graveyard`, `move_to_hand`, `move_to_exile` and
`move_to_top_of_library`. Tokens now perform the real zone change first (including
`Destroyed` and the death trigger queue for battlefield→graveyard, and the real
hand/exile/library record). `check_sbas` then removes every token in a
non-battlefield, non-`Removed` zone to `Zone::Removed` as the CR 111.7 state-based
action. This preserves CR 111.8’s “cannot move again” behavior and leaves no
graveyard/hand/exile residue after the SBA checkpoint. Swords to Plowshares still
gains the token’s last-known power (1) before the exile record is removed.

**GREEN:** destination/`Destroyed` events precede cessation; no residue; Swords
life gain verified.

## Tests and gates

```
cargo test -q -p aperture-core --test issue17_tokens \
  -p aperture-import --test starter_corpus_planned --test starter_corpus_analyzer
  -> 10 / 1 / 3 passed
```

Earlier affected tests (catalog, printings, command-zone, starter) stayed green.
`cargo fmt --all --check`, `cargo clippy -q --all-targets -- -D warnings` and
release CLI build exit 0 (`slice2-repairs-gates.log`, `slice2-repairs-focused.log`).

## Executable journey (rerun because token death semantics changed)

Four seats, exact supported `decks/package-krenko-tokens.txt`, four Krenko
commanders, Commander-combat 40 life, seed 7:

* terminal, P0 wins, 1805 steps / 40 turns
* new digest `e2827d82da51…` (previous `f4768c8a3bd2…`; expected change)
* 109 token-cessation events, including real graveyard changes before cessation
* separate replay and `--save-out` save both reproduce the digest
  (`slice2-repairs-replay.log`, `slice2-repairs-save-replay.log`)

The unchanged Jasmine replay still reproduces `d7f8c2adaa17…`
(`slice2-repairs-jasmine-replay.log`).

Desired list content was not touched: `git diff --stat` over
`decks/starter-corpus/planned/` and the Jasmine list is empty; the coordinator’s
`slice2-input-list-hashes.json` baseline remains valid for content.

## Talrand — deliberately not registered

The required token repairs consumed the runtime budget. A complete Talrand
requires a spell-cast trigger that is put above the spell even if countered,
controller-only filtering, a blue 2/2 Drake token and real flying block
restriction. None was left partly registered; no flying or spell-copy text stub
exists. This is the next concrete package.

## Remaining honest gaps

* Talrand, Sky Summoner not implemented.
* Tokens are still only the 1/1 red Goblin type.
* Legend rule absent; Krenko is commander-only.
* The exact 100-card Krenko desired list stays `planned` (52 missing names).
* `tokens.rs` from a separate #16 lane does not exist here; no overlap or
  approval is claimed.
* Assignment-1’s implementation-first/no-RED disclosure is retained unchanged;
  these two corrections are the first genuine RED→GREEN cycles in slice 2.

Files changed this assignment: `crates/core/src/engine.rs`,
`crates/core/tests/issue17_tokens.rs`, plus evidence
`SLICE2-IMPLEMENTATION-HANDOFF-2.md`, `slice2-result-2.json`,
`slice2-repairs-*`.
