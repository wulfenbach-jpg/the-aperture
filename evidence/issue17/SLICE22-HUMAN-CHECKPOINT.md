# Issue17 slice22 — reviewed rolling checkpoint

Same card t_67778feb / checkout `C:/AI/aperture-worktrees/issue-17` / branch
`aperture/issue-17` / draft PR46.
**Final head of record: 89247d66dd0de8a4923aa6bf3537326ac22d2509**
(c0e0a69 = code+evidence reviewed first; 89247d6 = hybrid-mana-value repair +
review evidence, both reviewed). Local == remote; PR OPEN / draft / MERGEABLE /
BEHIND master. **No merge, no master push, no issue closure, no
certified-downstream unlock.**

## Usable addition

**Dancing from Dark to Dawn**, `{3}{G}{G}` Enchantment — Scryfall **HOB 123**,
immutable `550cd0b6-ca61-4db7-9d20-0b68c48066f9`, oracle
`5ba482e9-fbb0-4d9f-a3a9-414892bcdfed`, retrieved `2026-10-27T06:31:42Z`.
Raw Scryfall records `evidence/issue17/slice22/raw/`, normalized record
`fixtures/starter-corpus/slice22-sourced.json`, catalog text byte-equal to the
printing, `card_printings` chains the new fixture.

Both printed abilities implemented in full:
1. "Whenever you cast a creature spell, put X +1/+1 counters on target creature
   you control, where X is that spell's mana value."
2. "Landfall — Whenever a land you control enters, create a 2/2 green Bear
   creature token."

Reusable surface, not a one-card patch:
* `TriggerEvent::CreatureSpellCast` — separate from the instant/sorcery
  `SpellCast`; a creature spell never triggers that wording and a noncreature
  permanent spell triggers neither.
* `card::cost_mana_value` + `CardDef::mana_value()` — CR 202.3.
* `TriggerRef::spell_mana_value` / `StackItem::spell_mana_value` carry the value
  captured at cast time; both serde-skipped at zero, so no recorded digest moved.
* `Effect::PutPlusOneCountersOnTargetForTriggerSpellManaValue`.

## Defects found and repaired inside the slice

1. **Engine (blocking, found by my own test):** `apply_cast` granted priority
   unconditionally after casting, so a cast trigger waiting for a target left
   `pending = TriggerTarget` **and** a priority holder; the session validator
   refused to load it. Retained RED `slice22/focused-2.log`. Repair: priority
   only when no choice is pending (CR 603.3d).
2. **Mana-value rule (found by the independent reviewer, unreachable today):**
   `mana_value` returned 0 for a hybrid/Phyrexian symbol with no numeric half
   (`{G/W}`, `{W/P}`) instead of 1. Repair: the rule is now the free function
   `card::cost_mana_value` with the numeric-less branch at 1, tested directly on
   the symbol shapes the pool does not print, plus a pool-wide agreement loop.
   Proof the new test discriminates: the reviewer ran both versions in a scratch
   program — 6 shape mismatches before, 0 after; the new test dies on its first
   assert against c0e0a69.

## Reviews (both independent, neither written by the implementer)

* `evidence/issue17/REVIEW22.md` (+ `slice22/REVIEW22.json`) — **VERDICT:
  APPROVED** for the delta `b671414..c0e0a69`. 16 externally written probe cases
  driving real casts/target choices/land plays: X = 2 for `{1}{G}` and 6 for
  `{4}{G}{G}`, opponent's cast 0 triggers, enchantment spell 0 triggers and
  still resolves, real 2/2 green Bear-subtype token, two enchantments → 2
  triggers/4 counters, countered spell still sizes the trigger, target destroyed
  → no panic and no counters, no legal target → `TriggerRemoved`, plain cast
  still yields priority, 134/134 costed pool cards agree with `parse_cost`,
  zero-state serialization has no `spell_mana_value` key, slice-20/21 evidence
  replays still reproduce `72cd69fb1143` / `0d20c7597183`.
  Its two non-blocking notes are now closed or disclosed: the README pool-count
  edit is a root doc outside `crates/` (scope note); `SAVE_VERSION` is not bumped
  — consistent with slices 19/21, which also added serde-skipped fields.
* `evidence/issue17/slice22/REVIEW22-DELTA.md` — **VERDICT: APPROVED** for the
  repair delta `c0e0a69..89247d6`: 9 files (2 product, 7 evidence), CR 202.3
  verified over 17 shapes by extracting both predicate versions into scratch
  code, `issue17_slice22` 9/9, `card_printings` 8/8, `catalog_pool` 5/5,
  `fmt --check` exit 0.

## Executed evidence (mine)

* `slice22/red-card-unsupported.log` — genuine RED before any production edit
  (3 tests, 0 passed / 3 failed, card unregistered).
* `slice22/focused-2.log` — the save-validator RED.
* `slice22/red-mutation-mana-value-constant.log` — mutation RED: a constant mana
  value fails exactly the three size assertions while the four discriminations
  stay green.
* `slice22/focused-3.log` — 8/8; the repair commit's run adds a 9th test
  (`mana_value_counts_the_symbol_shapes_the_pool_does_not_print_yet`).
* `slice22/native-gates.log` — every step's own exit code: `fmt_exit=0`,
  `clippy_exit=0`, `release_exit=0`, and `_exit=0` for slice22,
  `card_printings`, `catalog_pool`, `issue13_commander_combat`,
  `issue17_slice15_corpus`, `abilities`, `issue17_slice20`, `issue17_slice21`,
  `projection`, `timing_mana`, `m1_multiplayer`, plus `workspace_exit=0` with no
  FAILED line in `cargo test --workspace --all-targets --locked --no-fail-fast`.
  Disclosed caveat: that workspace run finished before the `cargo fmt` pass on
  this slice's files; fmt/clippy were re-run green afterwards (also after the
  hybrid repair), but the whole-workspace suite was not repeated at the final
  formatted bytes.
* `slice22/native-journey.log`, `demo-journey.json`, `demo-replay.json`,
  `demo-save.json` — real four-seat / 40-life / 40-card package journey, seed
  2222, exact `decks/package-slice22.txt` snapshot
  `aede2055909b8857db7b1c5b2196e72fdecabe11047ef23bc42c989f1b6c8b81`: terminal
  **Win{P3}**, **1500 actions / 33 turns / 3891 replay events**, digest
  `17f03348d81c1a92a6bd578db9ce2be95737985a6e255ff2fa769389e128a04d`, separate
  replay reproduces it, fresh save roundtrips; 27 triggered abilities staged
  (11 cast + 16 landfall), 11 counter events with the captured X, 16 Bear
  tokens. Evidence-only masked demonstrator, **not** product AI; Jasmine Boreal
  injected into all four command zones; **not** a 100-card certificate.
* `slice22/corpus-audit.log` — `AUDIT OK: catalog 139, union 459 = 127 supported
  + 332 missing, Ayula 26, 1 certified`, per-list `55/0/51/48/26/52/52/53`.

## Exact-head CI

Ten check runs exist for 89247d6. Status at checkpoint time: 2/10 completed
SUCCESS (`board and roadmap`, push and pull_request runs); the six rust jobs and
the two windows-package jobs were still `in_progress`. The capture is written
beside this file as `slice22/exact-head-ci-89247d6.json` if the poll finished;
**if that file is absent, or shows anything other than 10 completed SUCCESS keyed
to 89247d6, this head is unverified — do not inherit green from c0e0a69 or
b671414.**

## Not done / not claimed

* No second 100-card starter: **still 1 of 8 certified** (jasmine only).
* Full original #17 acceptance stays open.
* Implemented **natively** by the sole writer for these files; no OMP
  writer/reviewer session was resumed, so both original JSONL histories are
  unchanged by this slice. The two pre-existing **append-only** session JSONL
  modifications in the worktree were left uncommitted and untouched.
* This checkpoint and `REVIEW22-DELTA.md` sit outside the branch tip so CI stays
  bound to 89247d6; they are delivered as card attachments.

## Next

Preserve approved slices 1–21 without re-audit. Then sourced Ayula work with 26
names left — highest-yield primitives for this list are library search/tutor
(Spirit of the Aldergard, The Earth King, Titania's Command), equipment with
shroud/hexproof (Lightning Greaves, Swiftfoot Boots) and the type / base-P/T
family (Circle of the Moon Druid, Stuffed Bear, Ursine Champion).
