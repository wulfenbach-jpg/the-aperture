# Issue17 — slice19 human checkpoint

## Increment, not whole-issue acceptance

Same branch `aperture/issue-17`, draft PR46. Resumed from the approved slice-18 head
`21fdefb`. One sourced card, and it is a real subsystem rather than a stat line:

**Spectral Bears** `{1}{G}` — Creature — Bear Spirit 3/3.
Printed: "Whenever this creature attacks, if defending player controls no black
nontoken permanents, it doesn't untap during your next untap step."
Immutable printing `https://api.scryfall.com/cards/e56143db-0274-4878-9d54-619e7f037a33`
(me1 131, Masters Edition), retrieved `2026-10-27T00:54:48Z`, recorded in
`fixtures/starter-corpus/slice19-sourced.json`. The oracle text in the catalog is
byte-equal to that printing.

## What is new in the engine (all required by the printed text)

1. **`TriggerEvent::Attacks` — the pool's first attack trigger.** Declaring
   attackers now queues and stages "whenever this creature attacks" triggers in the
   declare attackers step (CR 508.1), using the existing trigger queue, APNAP order
   and stack machinery. There was no attack-trigger concept before this slice; five
   other Ayula-list cards and most of the corpus need it.
2. **`TriggerCondition` / `ability::trigger_condition` — a printed intervening-if**
   (CR 603.4). It is evaluated *before* the trigger is queued, so a false clause
   never reaches the stack (a `TriggerRemoved` event records why), and it is
   re-checked at resolution, so a clause that stops being true removes the ability.
   It is held per card rather than as a new `TriggeredAbility` field, so all 31
   pre-existing trigger literals are untouched.
3. **`CardDef::is_black`** — printed color from the mana cost (CR 202.2a) or a
   token's own `colors`. Nothing in the pool changes a permanent's color, so this is
   the whole color model this card reads.
4. **`Effect::SkipNextUntapStep` + `GameState::skip_next_untap`** — a one-shot
   "doesn't untap during your next untap step" marker (CR 502.3). The untap step
   consumes it for the affected seat only, leaves the permanent tapped, does *not*
   make it summoning sick, keeps other seats' markers queued, and drops markers for
   objects that already left the battlefield. The set is serde-skipped when empty,
   so every previously recorded digest is unchanged.

## Genuine RED evidence (all three retained)

Each mutation was applied to the committed source, the new suite was run, and the
source was restored from the committed file:

1. `evidence/issue17/slice19/red-mutation-condition-always-true.log` — forcing the
   pre-queue intervening-if to `true` fails **exactly**
   `a_black_nontoken_permanent_on_defense_stops_the_trigger` (4 passed, 1 failed).
2. `evidence/issue17/slice19/red-mutation-untap-ignores-restriction.log` — making
   the untap step ignore the marker fails the two tests that assert the restriction
   (3 passed, 2 failed).
3. `evidence/issue17/slice19/red-mutation-resolution-recheck-disabled.log` — forcing
   the resolution-time re-check to `true` fails **exactly**
   `the_printed_condition_is_rechecked_when_the_trigger_resolves` (5 passed,
   1 failed). This branch was untested in the first cut and an independent review
   caught it; it now has its own discriminating control.

So the tests discriminate the printed semantics, not merely the presence of a card.

## Executed verification (native, this run)

- `crates/core/tests/issue17_slice19.rs` — 6 focused tests: registration against the
  printed text, the trigger firing against a black-free defender (attacker stays
  tapped through exactly one untap step while a bystander untaps, then untaps
  normally on the following turn), a black **nontoken** permanent stopping the
  trigger, a black **token** not stopping it, a nonattacking bear never being
  marked, and the resolution-time re-check removing the ability when the defending
  board gains a black nontoken permanent while the trigger is on the stack.
- Workspace gates: `cargo test --workspace --all-targets --locked --no-fail-fast`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo build --workspace --release --locked`, `cargo fmt --all --check`.
  `evidence/issue17/slice19/gates.log` is produced by
  `evidence/issue17/slice19/run-gates.sh` and records a per-step exit code; no gate
  result is inherited or asserted by hand.
- Real four-seat journey (evidence-only masked demonstrator, **not** product AI):
  `evidence/issue17/slice19/demo-journey.json` — exact 4 seats / 40 life / 40-card
  package `decks/package-slice19.txt` / seed 1919, **1773 actions, 42 turns**,
  terminal `Win{P3}`, **4709 replay events** with a matching separately-run replay
  digest `5fd841b2…a19f5d`, and a fresh-save roundtrip asserted in the driver.
  Spectral Bears was cast 5 times, its attack trigger was staged 14 times, and the
  untap restriction was observed in 13 real untap steps (it is not a paper claim:
  the driver asserts all three counts).
- Ayula, Queen Among Bears is injected as the commander for all four seats by the
  demonstrator; that is a demonstration choice, not product behavior.

## Independent review and the corrections it forced

An independent reviewer with no part in writing the code reviewed the delta
`21fdefb..e8f8b18` with the execution lens and returned **CHANGES_REQUESTED**. Its
findings were correct and are fixed here rather than argued with:

1. **The reviewed head did not compile its test suite.**
   `crates/core/tests/catalog_pool.rs` matches `Effect` exhaustively, so the new
   `Effect::SkipNextUntapStep` broke it (`E0004`) — the fix existed only as an
   uncommitted working-tree edit. Fixed and committed.
2. **`card_printings.rs` had no slice-19 record**, so
   `every_pool_card_added_after_issue_15_has_a_recorded_printing` failed at the
   reviewed head for the same reason (fix uncommitted). Fixed and committed.
3. **`issue13_commander_combat` pinned `pool_size() == 135`** — a previously green
   suite the delta broke. Updated to 136 with the history comment extended.
4. **`issue17_slice15_corpus` pinned `missing.len() == 30` / `all().len() == 135`** —
   same class of breakage. Updated to 29 / 136.
5. **`gates.log` was not honest evidence**: it contained a failing line from the
   author's own run and a mangled clippy exit code, so "green" was unevidenced. It is
   now produced by `run-gates.sh`, which records `fmt_exit`, `clippy_exit`,
   `release_exit` and `test_exit` from the commands themselves.
6. **The reviewed commit was not the code that was tested** (four uncommitted edits).
   Everything is committed; the reviewed head is the pushed head.

The reviewer also identified a genuine coverage gap that was **not** cosmetic: the
resolution-time re-check of the intervening-if had no test, so disabling it stayed
green. `the_printed_condition_is_rechecked_when_the_trigger_resolves` plus the third
mutation log close it.

Accepted as non-blocking and recorded rather than fixed: the untap restriction is
surfaced as `Event::StateBasedAction` prose (a typed event can wait for a second
consumer), object ids are never reused (`next_object` is monotonic, and markers for
objects off the battlefield are dropped at the next untap step), and
`ability::trigger_condition` is a card-keyed table beside `abilities(card)` — the
same pattern as `static_abilities(card)`, deliberately deferred generalisation.

## Corpus state (independently recomputed this run)

`evidence/issue17/slice19/corpus-audit.py` (own parser: catalog source + all eight
desired deck files + manifest) →
**136 catalog cards**, union **459 names = 124 supported + 335 missing**,
per-list missing `55 / 0 / 51 / 48 / 29 / 52 / 52 / 53`; every desired library still
parses to exactly 99 cards, every `missing_cards` entry is genuinely absent from the
catalog, and the desired snapshots are unchanged except the Ayula manifest entry.

## Original gate remains open

Eight-list gate unchanged: **1 of 8 desired starters certified**. Ayula still has 29
unsupported names; equipment, soulbond, hideaway, saddled mounts, Ring temptation,
trample/vigilance/reach grants and library search in that list remain unimplemented.
No arbitrary Archidekt compatibility, no full Commander, no v1 claim. The journey
package is explicitly **not** a 100-card certificate. No merge, no master push, no
issue closure, no certified-downstream unlock.

## Deviations disclosed

- Production code was written before the new tests (scaffold-first, **not** strict
  TDD). The two mutations above are the honest compensating evidence.
- `is_black` derives catalog color from the mana cost. That is the printed-color
  rule, but a card whose printed color came from a color indicator instead of a cost
  would need its own field; no pool card does.
- The intervening-if is modelled as one enum variant. A general condition expression
  is deliberately deferred until a second card needs it.
- The journey driver is an evidence-only demonstrator: it prefers casting the new
  card and attacking all-out. Its result is not strength, AI-quality or product
  evidence.

## Resume

Same card / PR46 / branch / saved histories. Next: continue sourced Ayula-list cards
(the remaining cheap semantics are `Vivien's Grizzly`, `Dragon-Scarred Bear`,
`Kamahl's Summons`) and then genuine certification of one whole desired 100-card
list. Do not re-audit approved increments. Current-lane target only; preserve the
other agent's build-cache cleanup; no retired-worktree builds or global config edits.
