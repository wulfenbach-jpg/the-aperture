# Slice 47 (R325-1: the "attacking or blocking alone" reading) — human checkpoint

**Original #17 UNFINISHED: 1 of 8 lists certified, Ayula 6 names missing.** Same card, branch
`aperture/issue-17`, same draft PR46. No merge, master push, issue closure, downstream unlock, new
team or global config change. This run wrote the change, so it is **not** its own independent
reviewer.

## What this slice publishes

Review 325's one required correction, **R325-1**, is closed at the source, and the non-blocking
rules finding it raised is recorded as a visible known limit. No new card is added: the pool stays
at **160** cards and Ayula keeps its six remaining names.

### 1. The predicate is now the CR 506.5 declaration record, in code as well as in prose

The card's printed intervening-if ("if it was attacking or blocking alone") was *described* as the
declaration reading but *computed* from the live combat lists. Those lists are pruned whenever a
creature leaves combat (`detach_object` at `engine.rs:1694`, `prune_combat`, the post-SBA retain), so
the description and the behaviour diverged in exactly the case review 325 probed:

> two attackers declared → one killed in the declare-blockers step → the other killed later in the
> same combat. The survivor was the only creature left in `turn.attackers`, so it read as "attacking
> alone" and triggered — although it was declared beside the first attacker.

What changed:

* `Turn` now carries `declared_attackers` and `declared_blockers` (`crates/core/src/state.rs`): the
  declaration record, written once when the declaration is submitted
  (`apply_declare_attackers` / `apply_declare_blockers`), **never pruned** when a creature leaves
  combat, and cleared with the rest of the combat state at end of combat (CR 511.3) and at turn
  start.
* `capture_combat_alone` reads that record instead of the live lists, so every death in a combat —
  simultaneous or sequential — sees the same answer.
* The engine comment, the `DeathRecord`/`CombatAloneSnapshot` docs and
  `docs/issue17-starter-corpus.md` §Slice 46 now all say the same thing, and the docs state the
  sequential-mid-combat consequence in plain words.

Two regressions pin the discriminating case, one per side, and both were shown to fail on the old
reading rather than assumed to be discriminating:

* `attacker_killed_after_its_companion_left_combat_is_not_alone`
* `blocker_killed_after_its_companion_left_combat_is_not_alone`

The retained RED (`slice47/native-red-live-state-reading.log`, exit 101) runs those two tests
against the old live-state `capture_combat_alone`: both fail, and the attacker test panics with the
run's own symptom — `left: Exile / right: Graveyard`, i.e. the survivor's death was exiled by the
trigger that must not fire. The fix's diff is `crates/core/src/{state,engine}.rs` plus the two tests.

### 2. The digest consequence, disclosed rather than papered over

Recording the declaration changes the canonical JSON of a state that has a live declaration, so a
**replay whose terminal state is a combat kill with declared attackers no longer reproduces its
recorded digest**. Executed here, both ways:

| Recording | Under this head |
| --- | --- |
| `slice46/demo-replay.json` (game ends on an empty-library draw; no live declaration) | **matches**: `da4eacd5…b995bde`, 8098 events |
| `slice45/demo-replay.json` (game ends on combat damage with `"attackers": [128,129,134,…]`) | actions and outcome identical, digest now `99bc8446…` where `45d674d2…` was recorded |

Log: `slice47/prior-recordings.log`. This is not avoidable without keeping the defect: the
declaration is public game state that must survive a save/load, and the alternative — omitting it
from serialization — would make a reloaded mid-combat save forget which creatures were declared (a
rules error in the opposite direction). Saves recorded before this change still load and still
verify their own stored digest, because an absent field stays absent and serializes identically
(`skip_serializing_if` empty); the pre-#16 fixture test proves that path in the workspace suite.

### 3. CR 903.9a commander return recorded as a known limit

Under CR 903.9a a commander that would be put into a graveyard may be put into the command zone
*instead*, so in a real game it never dies and no dies trigger fires. This engine moves it to the
graveyard, records the death, queues `Dies`/`AnotherCreatureDies` watchers, and only then runs the
return (`apply_commander_return_batch`). Thijarian Witness therefore over-triggers on a returned
commander — it finds nothing to exile and still investigates. Pre-existing, not introduced here; it
is now written up with the CR citation and the correct shape (a replacement effect on the graveyard
move, CR 614.1a) in `docs/issue17-starter-corpus.md` §Slice 46 and in README's
"What is explicitly NOT implemented". No test pins the deviant behaviour, so fixing it later needs
no test rewrite.

### 4. The six remaining Ayula names now have a written implementation brief

`slice47/NEXT-CARDS-BRIEF.md` inventories, per card, the printed text with printing/oracle id and
retrieval stamp (sources preserved in `slice46/sources/`), the primitives that already exist, and
the primitives that do not (verified by grep, not assumed): no Ring, no face-down exile, no
play-from-exile, no X on a creature spell, no multi-target sum cap, no "no abilities" predicate, no
damage-as-unblocked, no indestructible, no random-opponent chooser, no forced attack, no
"put a creature onto the battlefield tapped and attacking". It also orders the six by cost, with
Owlbear Cub first and the Ring last as its own issue.

## Verification (all executed in this checkout, this run)

| Gate | Result | Log |
| --- | --- | --- |
| `cargo test -p aperture-core` focused set (17 suites) | **130 passed / 0 failed** | `slice47/focused-tests.log` |
| `cargo test --workspace --locked` | **926 passed / 0 failed / 1 ignored over 133 binaries**, exit 0 | `slice47/workspace-tests-final.log` |
| `cargo fmt --all --check` | exit 0 | `slice47/fmt.log` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | `slice47/clippy.log` |
| `cargo build --workspace --release --locked` | exit 0 | `slice47/release-build.log` |
| Four-seat 40-life journey, re-executed from the committed driver | terminal `Win{P1}`, 3073 steps, 54 turns, **digest `da4eacd5363f…b995bde` — identical to the slice-46 recording**, 8098 replay events, 38 solo attacks, 104 Clues, 14 exiles | `slice47/demo-journey.json`, `slice47/journey.log` |
| Separate release-CLI replay of that recorded line | `replayed digest da4eacd5363f \| matches recorded \| events 8098` | `slice47/cli-replay.log` |
| Replay/save agreement inside the driver | `replay.run()` digest equal; fresh `SessionSave` reload equal to the live state | driver assertions |
| Corpus audit (own parse of `catalog.rs` + the eight lists) | catalog **160**, union **459 = 147 supported + 312 missing**, per list `[55,0,51,48,6,52,52,53]`, **1 certified** | `slice47/corpus-audit.log` |
| Exact-head CI on the code head `4c55d94` | recorded in `slice47/exact-head-ci-4c55d94.json` + `slice47/ci-watch.log` | see metadata |

The journey being **byte-identical** to slice 46 is the useful part of that gate: the fix changes
only the case it must change. The demo driver declares one attacker per combat, so its game never
depends on the discarded live-state reading.

## What this slice does NOT claim

* **No starter list is certified.** Ayula still has **6** missing names (Dúnedain Rangers, Evercoat
  Ursine, Owlbear Cub, Rampaging Yao Guai, Ruxa Patient Professor, Ursine Monstrosity); the gate
  stays **1 of 8** and `missing_cards` was not touched because no card left the gap list.
* **No 100-card four-seat certification.** The journey is a 40-card package of four cards driven by
  an evidence-only masked demonstrator.
* **No new card, no new primitive beyond the declaration record.** Catalog 160 before and after.
* **`crates/ai/src/search.rs` gained two literal fields** so the workspace compiles; they are read
  from `real_state` like the neighbouring public combat facts (`blocked`, `defender`) because the
  declaration was announced to every seat. No search policy, budget or hidden-information rule was
  changed, and this is the only non-core file touched.
* The `slice45` replay digest divergence above — a real consequence of recording public state, not a
  defect being hidden.
* The legend rule (CR 704.5j), the CR 903.9a return shape, and every limitation listed in the slice-46
  checkpoint remain exactly as they were.

## Honest authorship

This run wrote the state change, both regressions, the RED proof and the docs; it did not implement
any of the six remaining Ayula cards, and it did not re-audit any previously approved increment. It
therefore co-authored this increment and cannot be its independent reviewer.

## Next gate

Independent practical review of this delta against `aab47c1` (code head `4c55d94`), with the
declaration reading and the two discriminating regressions as the focus and the digest divergence
above as a disclosure to judge. Then the same writer/history/card/PR46 takes
`slice47/NEXT-CARDS-BRIEF.md` in order — Owlbear Cub first — with the whole-field
`missing_cards`/`missing_interactions` rewrite the moment a card leaves the gap list, and only when
both reach zero the exact 100-card four-seat certification with separate replays.
