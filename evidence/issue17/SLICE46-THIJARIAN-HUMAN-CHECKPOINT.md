# Slice 46 (Thijarian Witness) — human checkpoint

**Original #17 UNFINISHED: 1 of 8 lists certified, Ayula 6 names missing.** Same card, branch
`aperture/issue-17`, same draft PR46. No merge, master push, issue closure, downstream unlock, new
team or global config change. This run is the implementation side; it is **not** an independent
review of its own work.

## What this slice publishes

**Thijarian Witness** (`{1}{G}`, 0/4 Creature — Alien Cleric, WHO 111) is now a supported card, and
the preserved WIP from the previous run is finished rather than re-derived. It is the pool's first
"another creature dies" watcher and its first `investigate`.

Sources: `fixtures/starter-corpus/slice46-sourced.json` (Scryfall printing
`da05e23b-419f-4023-9ad4-1227bb1b53be`, oracle `d273033b-3b8b-4ce9-afe8-208d82dd20f1`, retrieved
2026-10-28T13:49:35Z). Both printings of the card (WHO 111 and WHO 716) carry identical oracle text;
the catalog string equals that text verbatim, and `card_printings` enforces it.

### The two harness paths the previous run named are fixed, not worked around

The previous attempt failed its two real-combat tests inside the *test harness* and said so:

1. **After `DeclareAttackers` the declare-blockers step is not pending yet.** Correct — CR 508.2
   needs both players to pass first. The tests now submit the attackers, pass P0 then P1, and only
   then assert `Pending::DeclareBlockers { player: P1 }`; combat is driven to
   `Step::PostcombatMain` by priority passes, so the death trigger resolves through the engine's own
   stack.
2. **The out-of-combat burn control asked for offers while P0 held priority.** Correct — CR 117.1d
   gives the active player priority first. The test now passes P0 to hand priority to P1 and asserts
   `decision_owner() == Some(P1)` before searching P1's legal actions.

### Three real defects in the WIP implementation, found and fixed here

These are product bugs, not harness bugs, and each one alone would have made the card silently
useless while every fixture test written the WIP's way would still have passed in isolation:

1. **`death_watchers` never snapshotted the card.** The watcher list is filtered by trigger event and
   did not include the new `AnotherCreatureDies`, so a Witness on the battlefield was invisible to
   every death batch and the trigger could never queue. Added to the filter.
2. **"Attacking or blocking alone" was read after the zone change.** `detach_object` prunes a
   departing creature from `turn.attackers` / `turn.blocked` / `turn.blockers` as part of the move, so
   the predicate evaluated after the move reported "not in combat" for *every* creature that died in
   combat. The fact is now captured before the move, in `move_to_graveyard_inner`.
3. **A simultaneous batch made the last death look alone.** Because each move prunes one attacker, a
   per-death read would report the *second* of two attackers that die together as "the only attacker"
   and fire the trigger on a combat that was never alone. This was caught by the deliberate control
   test (`death_beside_another_attacker_does_not_trigger`) failing with
   `[GrizzlyBears]` already exiled. The combat declaration record is now snapshotted once per batch
   (`CombatAloneSnapshot` carried on the new `DeathRecord`) so every death in a batch sees the same
   combat — CR 704.3.

The exile also had to address the *graveyard* object rather than the pre-move id (they differ for a
commander, whose zone change creates a new object, CR 400.7), and the duplicate exile event the WIP
emitted was removed in favour of the one `move_to_exile` already records.

## Verification (all executed in this checkout, this run)

| Gate | Result | Log |
| --- | --- | --- |
| `cargo test -p aperture-core` focused set | **129 passed / 0 failed** over 14 suites | `slice46/focused-tests.log` |
| `cargo test --workspace --locked` | **924 passed / 0 failed / 1 ignored** over 133 binaries, exit 0 | `slice46/workspace-tests.log` |
| `cargo fmt --all --check` | exit 0 | `slice46/fmt.log` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | `slice46/clippy.log` |
| `cargo build --workspace --release --locked` | exit 0 | `slice46/release-build.log` |
| Four-seat 40-life journey | terminal `Win{P1}`, 3073 steps, 54 turns, 8098 replay events, digest `da4eacd5363fc16b6242d1c52fe86c2157c556d5e66a01696a6d6ba15b995bde` | `slice46/demo-journey.json` |
| Journey replay inside the driver | `replay.run()` digest equal; fresh `SessionSave` reload equal to live state | driver assertions |
| **Separate** release-CLI replay of the recorded line | `replayed digest da4eacd5363f \| matches recorded \| events 8098` | `slice46/cli-replay.log` |
| Corpus audit | catalog **160**, union **459 = 147 supported + 312 missing**, per list `[55,0,51,48,6,52,52,53]`, **1 certified** | `slice46/corpus-audit.log` |

The journey is a 40-card package (`decks/package-slice46.txt`), not a desired starter and not a
strength claim. Its driver is an evidence-only masked demonstrator: it attacks with the *weakest*
eligible creature that is not the 0/4 Witness so an attack declared alone actually trades, and it
cracks Clue tokens. In that game the Witness was cast **11** times, **38** solo attacks were
declared, **104** Clue tokens were created and **14** creature exiles were recorded.

**One integration break was caught by the full gate and fixed in this slice**: two tests carried a
frozen pool size (`issue13_commander_combat.rs`, `issue17_slice15_corpus.rs`). The first failing
workspace log is retained (`slice46/workspace-tests-failure-159.log`); the numbers now track the
compiled pool (160) and the Ayula gap (6).

## What this slice does NOT claim

* **No starter list is certified.** Ayula still has **6** missing names (Dúnedain Rangers, Evercoat
  Ursine, Owlbear Cub, Rampaging Yao Guai, Ruxa Patient Professor, Ursine Monstrosity) and the
  eight-list gate remains **1 of 8**. `missing_cards` and `missing_interactions` for Ayula were both
  rewritten as a whole fields (6 ↔ 6, no stale entry).
* **No 100-card four-seat certification.** The journey above is a 40-card package of four cards,
  played by one demonstrator policy.
* **Not a generic "dies" engine.** `AnotherCreatureDies` is implemented for this card's printed
  condition only; no other pool card uses it.
* The legend rule (CR 704.5j) is still unimplemented; the Witness is not legendary, so it adds
  nothing to that gap.
* A commander that dies alone in combat and is moved to the command zone is a new object, so this
  trigger finds nothing to exile there and still investigates (disclosed in
  `docs/issue17-starter-corpus.md` §Slice 46, not hidden as a convention).
* The remaining six Ayula names are heavy mechanics (the Ring tempts you, hideaway, X-cost
  multi-target destruction, "no abilities" filtering with combat-damage redirection, mill-driven
  attack restriction with indestructible). Their authoritative sources for all seven originally
  missing names are preserved in `slice46/sources/` from the previous run, and this run re-fetched
  and re-verified Thijarian Witness live (2 printings, 1 distinct text).

## Honest authorship

This run resumed the previous run's preserved WIP, fixed its two harness paths and three product
defects, wrote two new tests (the blocking-alone case and the tightened batch control), and produced
every gate above. It therefore co-authored the increment and cannot be its independent reviewer.

## Next gate

Independent practical review of this delta against `6bbe221`, then the same writer/history/card/PR46
continues with the six remaining Ayula names and, only when `missing_cards` **and**
`missing_interactions` both reach zero, the exact 100-card four-seat certification with separate
replays.
