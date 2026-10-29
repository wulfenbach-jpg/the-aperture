# Slice 57 human checkpoint — Rampaging Yao Guai enabled (announced X + aggregate target set)

**Original issue #17 remains unfinished: 1 of 8 lists certified, Ayula 2 missing
names.** This slice adds one whole card; it does not complete the issue, merge,
push master, close the issue or unlock `t_26a7a4ff` / `t_3c2fe51a`.

Same writer, same checkout, same branch `aperture/issue-17`, same draft PR46.
No provider/model/global config change, no new team, no new workers.

## 0. Where the head is

| ref | value |
| --- | --- |
| base | `0e16435` (slice-56 evidence tip; code `2ede45a`) |
| opening record commit (review-356's R356-1/2/3) | `4d519a8` |
| product commit (card + engine + corpus + docs) | `58a14e5` |
| self-review commit (set-commit guard, per-combination X offers) | `68fc8f1` |
| evidence-only tip | recorded in `slice57/RESULT.json` after CI |

The product commit is the only tree that changes behaviour. Every citation below
belongs to `58a14e5` unless it names a different head.

## 1. Why this slice ships a card

Review 356's directive: the next slice must enable Rampaging Yao Guai as one
package, or name a concrete external blocker at the top of the handoff. There is
no external blocker: the three clauses need engine surface this repository owns,
so the surface was built and the card enabled. It is not a record-only slice.

## 2. The card

Rampaging Yao Guai, PIP 82 (Fallout), `{X}{G}{G}{G}`, 2/2 Bear Mutant,
vigilance and trample. Printed text fetched live from Scryfall this run —
printing `63619236-a27a-4fdd-add6-7a295901f906`, oracle
`d37e8f75-c7cb-4270-bb2f-0125e972ed15`, HTTP 200 at 2026-10-29T02:29:04Z,
`cmc` 3 — and recorded verbatim in `fixtures/starter-corpus/slice57-sourced.json`.
The catalog string is that record's `oracle_text`, unmodified:

```
Vigilance, trample
This creature enters with X +1/+1 counters on it.
When this creature enters, destroy any number of target artifacts and/or enchantments with total mana value X or less.
```

## 3. The three clauses, and what enables each

**(1) `{X}{G}{G}{G}` — announced X.** `Action::CastSpellWithX { card, x, targets }`.
`CastSpell` is refused for a card that prints `{X}` (it would be a silent X=0)
and `CastSpellWithX` is refused for a card that does not print it; the supported
announced range is `0..=20`. X is added to the generic part of the cost before
the static cost reduction (CR 601.2f reduces the total generic), paid through the
ordinary payment plan, and stored on the stack item (`StackItem::x_value`,
serde-defaulted so every older save and replay is byte-identical). The spell's
mana value while on the stack is `3 + X` for a "you cast a creature spell"
trigger (CR 202.3).

**(2) "enters with X +1/+1 counters".** `enter_battlefield` now takes the
entering X, and places the counters on the **new battlefield object** after the
zone change — `insert_object` clears counters (CR 122.2), which is exactly the
bug the first implementation had and the reason the state, not the intent, is
stated here — and before the entry is announced, so an entry trigger already sees
them.

**(3) the aggregate destroy.** "Any number of target artifacts and/or
enchantments with total mana value X or less" is a target *set* under one
aggregate restriction, not a single target:

* `TargetSpec::ArtifactOrEnchantment` is the printed candidate set (every
  artifact and/or enchantment on the battlefield, either controller);
* `Pending::TriggerTarget` gained `aggregate: Option<u8>` (the announced budget)
  and `chosen: Vec<Target>`, both serde-defaulted and skipped when empty, so the
  pre-existing single-target path serializes and hashes exactly as before;
* `Action::ChooseTriggerTarget` adds one target while the running total stays
  within the budget, `Action::FinishTriggerTargets` commits the set (the empty
  set is legal — "any number" includes zero);
* resolution destroys the committed set as **one** instruction (CR 701.7a,
  704.3 — one simultaneous batch, death watchers see every death first); a
  committed target that left the battlefield does nothing (CR 608.2b) and the
  ability still resolves for the survivors; if no target survives the ability
  does not resolve.

## 4. Refusals — validate **and** a real load

`GameState::validate` now rejects an aggregate pending when:

1. the budget is not the trigger's announced X;
2. a chosen target is not one of the offered targets;
3. the same target appears twice;
4. a chosen target is not a battlefield artifact/enchantment;
5. the chosen total exceeds the budget;
6. `aggregate` is set on an ability that does not print the aggregate
   specification (and, symmetrically, `chosen` is non-empty on a single-target
   choice).

Each of the six is exercised twice in `issue17_slice57.rs`: once through
`GameState::validate` and once through a real `SessionSave::from_json` load of the
same forged state, with the honest pending validating, loading and continuing.

## 5. Gates at `68fc8f1` (all receipts in `slice57/`)

| gate | command | result | receipt |
| --- | --- | --- | --- |
| fmt | `cargo fmt --all --check` | 0 | `gates.log` (`fmt_exit=0`) |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `gates.log` (`clippy_exit=0`) |
| focused, this card | `cargo test -p aperture-core --test issue17_slice57 --locked` | 14 passed / 0 failed | `gates.log` (`focused_slice57_exit=0`) |
| focused, neighbours | slice55, slice54, slice53_ruxa, card_printings, catalog_pool, slice15_corpus | 50 passed / 0 failed across 6 suites (64 passed across 7 suites including this card) | `gates.log` (`focused_neighbours_exit=0`) |
| separate AI search suite | `cargo test -p aperture-ai --test search --locked` | 9 passed / 0 failed (253.36 s) | `gates.log` (`ai_search_exit=0`) |
| corpus | `python evidence/issue17/slice57/corpus-audit.py` | `AUDIT OK` 164 / 459 = 151 + 308 | `corpus-audit.log` |
| locked release | `cargo build --workspace --release --locked` | 0 | `gates.log` (`release_build_exit=0`) |
| whole workspace | `cargo test --workspace --all-targets --locked` | 139 result lines / 993 passed / 0 failed / 1 ignored, exit 0 | `workspace-all-targets.log` |

The focused/corpus totals above are the slice's own changed-behaviour gates; the
workspace line is the full local gate at the same tree.

## 6. One real four-seat journey, and a separate replay

`run-journey.sh` (driver `evidence/issue17/slice57-journey-driver`, exact 40-card
package `decks/package-slice57.txt`, snapshot sha256
`12ab766ed8be59ef74c0aaf7c222d735e269ab6b504c88d0281b31d5ef951004`, four seats,
40 life, seed 5757, Jasmine Boreal injected for all four seats by the
demonstrator):

* terminal **Win{P2}** (`opponents' life totals reached 0 or less`, CR 704.5a),
  **1960** accepted actions, **47** turns, **5052** replay events,
  digest `8a34da9bb066dd3ba18606043a65e0f4a39a32d60e3a181714f967c0ecb07253`;
* **10** Rampaging Yao Guai casts with X values `[0, 0, 0, 0, 1, 3, 4, 5, 3, 8]`
  — X=0 is played, not only inferred from a test;
* **6** aggregate target sets committed, sizes `[1, 2, 2, 2, 2, 4]`,
  **13** permanents destroyed by **6** destroy instructions;
* largest +1/+1 counter count on a Yao Guai at the end: **8**;
* the driver asserts a fresh non-sandbox `SessionSave` round trip preserves the
  exact state, and the committed replay reproduces the digest;
* **separate** release-CLI replay of `slice57/native/demo-save.json`:
  `replay of 1960 action(s) -> digest 8a34da9bb066 | events 5052 | reproduces the
  save`, exit 0.

This is a 40-card evidence package, **not** a 100-card certificate, not the
product heuristic and not a playing-strength or GUI claim.

## 7. Whole-field discipline

Rampaging Yao Guai left `missing_cards` **and** its matching
`missing_interactions` entry in `decks/starter-corpus/manifest.json` in the same
change. The audit recomputes all eight lists from `catalog.rs` and the deck files
and asserts: catalog 164, union 459 = 151 supported + 308 missing, per-list
`[55, 0, 51, 48, 2, 52, 52, 53]`, Ayula's two gap fields naming exactly
{Dúnedain Rangers, Evercoat Ursine}, no `missing_interactions` entry citing an
already-supported card, every deck exactly 99 + 1 commander, 1 of 8 certified.
README and the corpus doc were moved to 164 / 151 / 308 in the same commit.

## 8. Deliberate limits (also in `docs/issue17-starter-corpus.md`)

* The aggregate total is enforced at targeting (CR 601.2c) and not re-derived at
  resolution; a target that leaves does nothing, and only an entirely illegal set
  fails to resolve. Mana value is the printed cost with `{X}` = 0 (CR 202.3); no
  copy effects or cost modification of a permanent's mana value exist in the pool.
* `{X}` is bounded to `0..=20`; payment comes from the pool and untapped lands.
* The entry counters are ordinary +1/+1 counters read by the characteristic path
  (CR 613.4d); there is still no full ordered layer-7d pass, which is a
  pre-existing limit of the pool and not a claim this card changes.
* Vigilance is the keyword-table entry the attack path consults before tapping an
  attacker; trample is the `CardDef` flag the damage assignment reads. Nothing
  else about either keyword is claimed.
* The demonstrator is not a policy: it only chooses among projected legal
  actions, and its X/target preferences are its own, not the product heuristic's.

## 9. What is still missing for issue #17

Ayula now needs **two** cards: **Dúnedain Rangers** (the Ring and the
Ring-bearer), **Evercoat Ursine** (two hideaway resolutions, linked face-down
exile and free play). The other six lists are untouched. The exact 100-card
four-seat certification and its separate replays are still blocked on those two
cards, so the counter stays **1 of 8** and must be reported as such until the
certification actually passes.

## 10. Exact-head CI, stated exactly

`gh api .../commits/68fc8f1/check-runs` → **10 runs, every `head_sha` =
`68fc8f18ed49d494152c25ef841ac62bb0fdb708`, 10 completed `success`, 0 cancelled,
0 failure** (receipt `slice57/exact-head-ci-68fc8f1.json`, runs
`slice57/exact-head-runs-68fc8f1.json`).

Disclosed rather than glossed: the **first** attempt at those runs ended with 4
success and 6 cancelled, because the evidence push `5812114` started a second
workflow run for the new head and the repository's concurrency group cancelled
the in-flight jobs of the older head — the same operational mistake review 355
named, made again. The six cancelled jobs were re-run on the **same** `68fc8f1`
SHA (`gh api -X POST .../actions/runs/<id>/rerun-failed-jobs` for both Rust
quality runs) and completed green, so the number above is a real 10/10 on the
code head and not an inherited one. The evidence-only tip `5812114` claims no CI
of its own.

The green gate therefore rests on: the ten checks above, the local gates in §5,
and the journey in §6 — no claim depends on a cancelled job, a different SHA, or
the evidence tip.
