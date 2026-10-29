## Current candidate — slice57 Rampaging Yao Guai (`{X}` + aggregate target set)

**Original #17 remains unfinished: 1 of 8 starters certified, Ayula 2 names.** Same draft PR46; independent practical increment review requested, not merge or eight-list acceptance. Master untouched at `73820d7`; no merge, master push, issue closure, dependent unlock, new team or global-config change.

### Runnable change

**Rampaging Yao Guai** (PIP 82, printing `63619236-a27a-4fdd-add6-7a295901f906`, oracle `d37e8f75-c7cb-4270-bb2f-0125e972ed15`, live Scryfall fetch 2026-10-29T02:29:04Z, `cmc` 3, printed text verbatim in `fixtures/starter-corpus/slice57-sourced.json`) is sourced and enabled with all three printed clauses:

1. **`{X}{G}{G}{G}`** — `Action::CastSpellWithX` announces X (CR 107.3a). Plain `CastSpell` is refused for an X card and the X cast is refused for a card that does not print `{X}`, so X is never a silent zero. X is paid as additional generic mana (the CR 601.2f reduction applies to the *total* generic), counts in the spell's mana value while it is on the stack (CR 202.3), and rides on `StackItem::x_value` through a suspended resolution and a save/load round trip.
2. **"This creature enters with X +1/+1 counters on it"** (CR 614.1c) — placed on the new battlefield object after the zone change, because `insert_object` clears counters (CR 122.2). The first implementation placed them before that call; the focused test caught it and the failed attempt is disclosed.
3. **"When this creature enters, destroy any number of target artifacts and/or enchantments with total mana value X or less"** — a target *set* under one aggregate restriction (CR 115.1, 601.2c): new `TargetSpec::ArtifactOrEnchantment`, `Pending::TriggerTarget{aggregate: Option<u8>, chosen: Vec<Target>}` (serde-defaulted and skipped when empty, so every older save and replay is unchanged), `ChooseTriggerTarget` adds a target while the running total fits, `FinishTriggerTargets` commits it (empty set legal), and resolution destroys the set as one instruction (CR 701.7a, 704.3). A target that left the battlefield does nothing and the ability still resolves for the survivors; an entirely illegal set fizzles (CR 608.2b).

Six forged aggregate pendings (budget ≠ the trigger's X, unoffered target, duplicate, non-artifact/enchantment, total over the budget, a set on an ability that does not print it) are refused by **both** `GameState::validate` and a real `SessionSave::from_json` load, while the honest pending validates, loads and continues.

Whole-field discipline: the card left `missing_cards` **and** its matching `missing_interactions` entry in the same change; the audit recomputes all eight lists from `catalog.rs` and the deck files.

### Exact heads and verification

- **Record commit (review-356's R356-1/2/3):** `4d519a8` — opening commit of this slice, as directed.
- **Product head:** `68fc8f18ed49d494152c25ef841ac62bb0fdb708` — **10/10 completed SUCCESS CI, every check keyed to this full SHA, 0 cancelled, 0 failure**. Receipt: `evidence/issue17/slice57/exact-head-ci-68fc8f1.json`. The first attempt at those runs was 4 success / 6 cancelled by this lane's own evidence push under the repository concurrency group; the six cancelled jobs were **re-run on the same SHA** and completed green, so this is a real 10/10 on the code head.
- **Evidence-only tip:** every commit after the code head is evidence-only (checkpoint, receipts, PR body text). The PR head therefore moves as evidence lands and **claims no CI of its own**; the gate is the 10/10 on `68fc8f1` above.
- Local gates at the code head: fmt 0; `clippy --workspace --all-targets --locked -- -D warnings` 0; focused **7 suites / 64 passed / 0 failed** (14 new in `issue17_slice57.rs`); separate `aperture-ai --test search` **9 passed / 0 failed** (253 s, not counted in the 64); whole workspace `cargo test --workspace --all-targets --locked` **139 result lines / 993 passed / 0 failed / 1 ignored, exit 0**; locked release 0; whole-field corpus `AUDIT OK`.
- **One real four-seat, 40-life journey** on the exact 40-card package (snapshot `12ab766e…951004`, seed 5757): **Win{P2}, 1960 actions, 47 turns, 5052 events**, digest `8a34da9bb066dd3ba18606043a65e0f4a39a32d60e3a181714f967c0ecb07253`; **10** Yao Guai casts with X `[0,0,0,0,1,3,4,5,3,8]`; **6** aggregate sets `[1,2,2,2,2,4]`, **13** permanents destroyed in 6 destroy instructions; max **8** counters; fresh non-sandbox save/load equality and a **separate** release-CLI replay (`digest 8a34da9b… | events 5052 | reproduces the save`, exit 0). This is a **40-card evidence package, not a 100-card certificate, strength or GUI claim**.

### Remaining contract

Catalog **164**; fixed eight-list union **459 = 151 supported + 308 missing**. Per-list gaps `[55,0,51,48,2,52,52,53]`; certified **1/8**. Ayula still needs **Dúnedain Rangers** (the Ring and the Ring-bearer) and **Evercoat Ursine** (two hideaway resolutions with linked face-down exile and free play), identically named in both gap fields. Only Jasmine is certified; the inherited one-defender-per-declaration combat simplification remains explicit.

All original gates remain: eight explicitly supported 100-card lists of different styles; implementation/conformance for every included card and required interaction; four-seat games from exact starter snapshots with no bundled replacements; documented per-card/interaction exclusions, with arbitrary Archidekt compatibility a non-goal. Once both Ayula gap fields reach 0, run its exact 100-card four-seat certification with separate replays; **2 of 8 only on actual success**. Evaluation/soak children stay gated.

Authoritative handoff: `evidence/issue17/SLICE57-HUMAN-CHECKPOINT.md`; structured evidence: `evidence/issue17/slice57/RESULT.json`; gates/journey receipts in `evidence/issue17/slice57/`. This increment was written by the lane and does **not** claim independent approval.
