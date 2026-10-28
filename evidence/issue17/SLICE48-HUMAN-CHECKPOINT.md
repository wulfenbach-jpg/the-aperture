# Slice 48 — review-327 record corrections, then Owlbear Cub

Branch `aperture/issue-17`, same draft PR46. **Original #17 remains UNFINISHED: 1 of 8 lists
certified**, Ayula now 5 names missing. Nothing here merges, pushes master, closes an issue or
unlocks a child.

Two commits, deliberately separate:

| Commit | Content |
| --- | --- |
| `2d425b7` | R327-1..3 record corrections, the persistence version decision, one stale whole-field count. **No product code.** |
| `3e7675a` | Owlbear Cub (CLB 246) implemented with its sourced printing, 6 behaviour tests, provenance test, frozen-count and whole-field manifest updates |

## Part 1 — review 327's three record items (R327-1..3)

**R327-1 was factually wrong and is now rewritten in both places it appeared** (checkpoint §2 and
`slice47/RESULT.json` disclosure 3). Executed at this head:

```
$ target/release/aperture-cli.exe replay --file evidence/issue17/slice45/demo-save.json
Error: replay ended early: digest 99bc84466546…8431da != recorded 45d674d2e926…e2143
EXIT=1
```

The save parses and re-executes; it does not verify. The second half comes from the code: a legacy
mid-combat save deserializes `Turn::declared_attackers` as **empty**, so for the rest of that combat
`capture_combat_alone` answers "not alone" and a genuine alone-attack death stops firing Thijarian
Witness. **Version decision taken deliberately and recorded in four places** (checkpoint,
`RESULT.json`, README "Versioned persistence", `docs/issue17-starter-corpus.md` §Slice 47):
`SAVE_VERSION` stays **10**, `REPLAY_VERSION` stays **6** — a bump would reject every older artifact
including the pinned pre-#16 v10 fixture, which is worse than the digest under-report. The
limitation is now stated in user terms: older mid-combat saves replay to a different digest and may
under-report alone-attack triggers for the remainder of that combat. While editing that README
paragraph its stale numbers (replay v5 / save v8) were corrected to the real constants 6 / 10.

**R327-2.** The checkpoint journey row cited `slice47/journey.log`, a `--locked` lock-file failure
with `EXIT=101`. The row now cites `slice47/journey-build.log` (`EXIT=0`); the failed log is kept
but renamed **`journey-locked-attempt-FAILED.log`** with a header saying it is not the gate.

**R327-3.** `RESULT.json.pr_state` published `headRefOid 4c55d94 == local == remote` while the PR
head was `1b1c542`; it now states code head and evidence tip separately.

**Whole-field count (the deferred item).** `starters[elas-sacrifice].missing_interactions[0]` said
"remaining 53" against 52 `missing_cards`; corrected to 52.

**Recorded, not fixed.** The declaration record being outside `GameState::validate` is written up as
a known weakness for a later hardening pass (docs §Slice 47 + checkpoint §2), not dropped.

## Part 2 — Owlbear Cub (CLB 246), Ayula 6 → 5 missing

Printed text, copied verbatim from the captured Scryfall printing (`slice48/sources/` hold the raw
response and its stamp; `fixtures/starter-corpus/slice48-sourced.json` is the catalog-facing record):

> Mama's Coming — Whenever this creature attacks a player who controls eight or more lands, look at
> the top eight cards of your library. You may put a creature card from among them onto the
> battlefield tapped and attacking that player. Put the rest on the bottom of your library in a
> random order.

Four rule decisions, each with the reason the code is shaped this way:

1. **The intervening-if belongs to the defending player** (CR 603.4) and is evaluated twice — when
   attackers are declared and again as the trigger resolves. Seven lands means the trigger never
   reaches the stack, so nothing is looked at and the library is not touched.
2. **A look is not a search** (CR 701.20a). `Pending::SearchLibrary` gained a `looked` list so "the
   rest" means *the examined cards* and nothing else. `finish_look_at_top` randomises only those
   cards and puts them **under** the library (CR 701.20e) — the card that was ninth keeps its place
   and becomes the top card. No shuffle happens, and the rest of the library is never reordered.
3. **The put creature is attacking but was never *declared***. It is appended to `turn.attackers`
   (so it deals combat damage) and deliberately **not** to `turn.declared_attackers`, because
   CR 508.1 makes "attacked" a declaration and CR 506.5 reads "the only creature *declared* as an
   attacker". `slice47/NEXT-CARDS-BRIEF.md` had suggested recording it in both; that would have made
   the Cub stop counting as having attacked alone, so the brief's parenthetical is overridden here
   and the reason is tested.
4. **`GameState::validate` constrains the new pending**: the looked set must be exactly the top of
   the library in order, only creature cards among it may be eligible, and at most one card may be
   put.

### Tests — `crates/core/tests/issue17_slice48.rs`, 6/6

| Test | What it pins |
| --- | --- |
| `owlbear_cub_is_registered_with_its_printed_text` | name, cost, type line, 3/3, printed trigger line |
| `a_defender_with_seven_lands_never_triggers_the_look` | the threshold is 8; no pending, no stack entry, library untouched |
| `eight_lands_looks_at_the_top_eight_and_offers_only_creature_cards` | exactly the top eight; only its creature cards are eligible; the ninth is not offered |
| `declining_puts_the_looked_cards_on_the_bottom_and_keeps_the_rest_in_order` | "may" has a decline; all eight go to the bottom as a set; the ninth becomes the top |
| `the_chosen_creature_enters_tapped_attacking_and_deals_combat_damage` | tapped; in `turn.attackers`; **absent** from `turn.declared_attackers`; 3+2 damage really lands |
| `a_look_with_no_creature_card_puts_everything_on_the_bottom` | no eligible card → no choice offered, whole look still goes to the bottom |

Provenance: `card_printings::slice48_owlbear_cub_matches_its_recorded_printing` asserts the catalog
string equals the printing's `oracle_text` verbatim, the type line/cost/P-T equal the printing, the
printing's single keyword entry is the ability word (so `tokens::keywords` must stay empty), and the
one printed rules line is the one typed ability. `slice48_cards()` is chained into the
"every pool card has a recorded printing" guard.

### The tests discriminate (control executed)

Removing only the printed condition (making the trigger `Always`) and re-running the suite:

```
test a_defender_with_seven_lands_never_triggers_the_look ... FAILED
panicked at issue17_slice48.rs:174: seven lands is not eight: nothing may be looked at
test result: FAILED. 5 passed; 1 failed
```

Log: `slice48/control-always-condition.log` (exit 101). The condition was restored immediately
afterwards (`git diff` clean).

### Whole-field discipline

`decks/starter-corpus/manifest.json` `starters[ayula-bear-counters]` rewritten **as a whole**:
`missing_cards` loses "Owlbear Cub" (5 left) and `missing_interactions` loses its Owlbear entry
(5 entries, each naming one still-missing name). The elas "53 → 52" correction landed in the same
pass.

## Gates executed in this checkout

| Gate | Result | Log |
| --- | --- | --- |
| `cargo test -p aperture-core --locked` | **674 passed / 0 failed** | `slice48/core-tests.log` |
| `cargo test -p aperture-core --test issue17_slice48 --locked` | 6 passed / 0 failed | same file |
| `cargo fmt --all --check` | exit 0 | `slice48/fmt.log` |
| `cargo clippy -p aperture-core --all-targets --locked -- -D warnings` | exit 0 | `slice48/clippy.log` |
| Corpus audit (own parse of `catalog.rs` + the eight lists) | catalog **161**, union **459 = 148 supported + 311 missing**, per list `[55,0,51,48,5,52,52,53]`, **1 certified** | `slice48/corpus-audit.log` (`AUDIT OK`) |

## What this slice does NOT claim

* **No list is certified beyond the one it started with.** Ayula still has 5 missing names
  (Dúnedain Rangers, Evercoat Ursine, Rampaging Yao Guai, Ruxa Patient Professor, Ursine
  Monstrosity); the eight-list gate is still **1 of 8**. There is no 100-card certification.
* **No journey re-execution in this commit.** The slice-46/47 demonstration driver was not re-run
  after this change, and its 40-card package does not contain Owlbear Cub, so **the binary journey
  is not evidence for this card** — the six engine-driven tests are, and they drive real attack
  declarations, real priority rounds and real combat damage rather than a fixed state. Re-running
  the journey (ideally with the Cub in the package) and the exact-head CI readback are the next
  run's first gates.
* **No full-workspace run.** Only `-p aperture-core` was executed at this head; `crates/ai`,
  `crates/ui`, `crates/cli` were not rebuilt after the catalog grew, so a workspace-wide
  `cargo test --workspace --locked` is required before any of this is called reviewed.
* **Implementation-first, disclosed.** The engine code was written before the test file; the RED
  proof above is a post-hoc discriminating control, not a pre-implementation failing test, so this
  increment must not be described as TDD.
* Legend rule (CR 704.5j), the CR 903.9a commander-return over-trigger, the persistence limitation
  above and every earlier known limit are unchanged.
* This run wrote both commits and cannot be its own independent reviewer.

## Next actions (in order, for the next run on the same card/branch/PR46)

1. `cargo test --workspace --locked`, `cargo build --workspace --release --locked`, and the
   exact-head CI readback for `3e7675a` (locally **not yet claimed**).
2. Re-run the four-seat 40-life journey with Owlbear Cub in the package so the new card is covered
   by a real game, with a separate CLI replay.
3. Then the remaining five Ayula names per `slice47/NEXT-CARDS-BRIEF.md` order — Ruxa, Rampaging Yao
   Guai, Ursine Monstrosity, Evercoat Ursine, Dúnedain Rangers — with the same whole-field
   discipline, then the exact 100-card four-seat certification with separate replays.
