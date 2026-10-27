# REVIEW22 — issue 17 slice 22 (Dancing from Dark to Dawn), delta b671414..c0e0a69

Independent review. No implementation file was edited; nothing was merged, pushed or
commented on. Every number below comes from a command run in
`C:/AI/aperture-worktrees/issue-17` or from the reviewer's own probe crate.

## Head under review

```
$ git rev-parse HEAD
c0e0a69ad430e73d47b718ac8973ef464b32b632

$ gh pr view 46 --repo wulfenbach-jpg/the-aperture --json headRefOid,headRefName,state,isDraft
{"headRefName":"aperture/issue-17","headRefOid":"c0e0a69ad430e73d47b718ac8973ef464b32b632","isDraft":true,"state":"OPEN"}
```

Local HEAD == PR 46 head SHA: **confirmed**. Reviewed at 2026-10-27T06:57Z.

VERDICT: APPROVED

Non-blocking notes are listed under "Findings that do not block".

## 1. Diff scope and changed files

```
$ git diff --stat b671414..c0e0a69 | tail -1
 61 files changed, 23522 insertions(+), 23 deletions(-)
```

All 61 files are inside `crates/core`, `crates/core/tests`, `fixtures/`,
`decks/`, `docs/` and `evidence/` — **except `README.md`**, which is at the
repository root and therefore outside the allowlist stated for this lane. The
README edit is documentation only (pool count 138 → 139 in two sentences, matching
`catalog::pool_size() == 139`), so it is recorded as a scope note rather than a
defect.

Source files touched (6): `crates/core/src/ability.rs`, `card.rs`, `catalog.rs`,
`engine.rs`, `state.rs`; plus `README.md`.
Test files touched (9): `card_printings.rs`, `catalog_pool.rs`,
`issue13_commander_combat.rs`, `issue17_slice12.rs`, `issue17_slice15_corpus.rs`,
`issue17_slice22.rs` (new, 389 lines), `m1_multiplayer.rs`, `projection.rs`,
`timing_mana.rs`.
Data/doc files (3): `decks/package-slice22.txt`, `decks/starter-corpus/manifest.json`,
`docs/issue17-starter-corpus.md`. Fixture (1):
`fixtures/starter-corpus/slice22-sourced.json`. Evidence (42 files, all under
`evidence/issue17/slice22*`).

Every changed file:

```
README.md
crates/core/src/ability.rs
crates/core/src/card.rs
crates/core/src/catalog.rs
crates/core/src/engine.rs
crates/core/src/state.rs
crates/core/tests/card_printings.rs
crates/core/tests/catalog_pool.rs
crates/core/tests/issue13_commander_combat.rs
crates/core/tests/issue17_slice12.rs
crates/core/tests/issue17_slice15_corpus.rs
crates/core/tests/issue17_slice22.rs
crates/core/tests/m1_multiplayer.rs
crates/core/tests/projection.rs
crates/core/tests/timing_mana.rs
decks/package-slice22.txt
decks/starter-corpus/manifest.json
docs/issue17-starter-corpus.md
evidence/issue17/slice22-journey-driver/Cargo.toml
evidence/issue17/slice22-journey-driver/src/main.rs
evidence/issue17/slice22/demo-journey.json
evidence/issue17/slice22/demo-replay.json
evidence/issue17/slice22/demo-save.json
evidence/issue17/slice22/first-green.log
evidence/issue17/slice22/focused-1.log
evidence/issue17/slice22/focused-2.log
evidence/issue17/slice22/focused-3.log
evidence/issue17/slice22/native-gates.log
evidence/issue17/slice22/native-gates.sh
evidence/issue17/slice22/native-journey.log
evidence/issue17/slice22/probe.json
evidence/issue17/slice22/raw/Beorn's_Hospitality.json
evidence/issue17/slice22/raw/Beorn_the_Fierce.json
evidence/issue17/slice22/raw/Circle_of_the_Moon_Druid.json
evidence/issue17/slice22/raw/Dancing_from_Dark_to_Dawn.json
evidence/issue17/slice22/raw/Drover_Grizzly.json
evidence/issue17/slice22/raw/Druid's_Familiar.json
evidence/issue17/slice22/raw/Dúnedain_Rangers.json
evidence/issue17/slice22/raw/Evercoat_Ursine.json
evidence/issue17/slice22/raw/Exuberant_Wolfbear.json
evidence/issue17/slice22/raw/Goreclaw,_Terror_of_Qal_Sisma.json
evidence/issue17/slice22/raw/Kamahl's_Summons.json
evidence/issue17/slice22/raw/Lightning_Greaves.json
evidence/issue17/slice22/raw/Lumra,_Bellow_of_the_Woods.json
evidence/issue17/slice22/raw/Owlbear_Cub.json
evidence/issue17/slice22/raw/Rampaging_Yao_Guai.json
evidence/issue17/slice22/raw/Ruxa,_Patient_Professor.json
evidence/issue17/slice22/raw/Spider-Ham,_Peter_Porker.json
evidence/issue17/slice22/raw/Spirit_of_the_Aldergard.json
evidence/issue17/slice22/raw/Stuffed_Bear.json
evidence/issue17/slice22/raw/Surrak_and_Goreclaw.json
evidence/issue17/slice22/raw/Swiftfoot_Boots.json
evidence/issue17/slice22/raw/The_Earth_King.json
evidence/issue17/slice22/raw/Thijarian_Witness.json
evidence/issue17/slice22/raw/Titania's_Command.json
evidence/issue17/slice22/raw/Ursine_Champion.json
evidence/issue17/slice22/raw/Ursine_Monstrosity.json
evidence/issue17/slice22/raw/Words_of_Wilding.json
evidence/issue17/slice22/red-card-unsupported.log
evidence/issue17/slice22/red-mutation-mana-value-constant.log
fixtures/starter-corpus/slice22-sourced.json
```

## 2. Test suites

```
$ cargo test -p aperture-core --test issue17_slice22 --locked
running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Pinned suites the slice says it kept green (all run at c0e0a69, `--locked`):

| suite | result line | exit |
|---|---|---|
| `card_printings` | `test result: ok. 8 passed; 0 failed` | 0 |
| `catalog_pool` | `test result: ok. 5 passed; 0 failed` | 0 |
| `issue13_commander_combat` | `test result: ok. 43 passed; 0 failed` | 0 |
| `issue17_slice15_corpus` | `test result: ok. 2 passed; 0 failed` | 0 |
| `abilities` | `test result: ok. 13 passed; 0 failed` | 0 |

`cargo test --workspace` was deliberately not run (author already did; out of scope
for this review).

## 3. Independent behaviour probe (outside the slice's test file)

A path-dependency probe crate was built in a scratch directory
(`%LOCALAPPDATA%/hermes/cache/scratch/r22probe`, dependency
`aperture-core = { path = "C:/AI/aperture-worktrees/issue-17/crates/core" }`), so it
compiles against the reviewed worktree without touching it. It builds boards through
the public `GameState`/`Game::from_state` API and drives **real** actions
(`Action::CastSpell`, `Action::ChooseTriggerTarget`, `Action::PlayLand`,
`Action::Pass`, opponent `Action::CastSpell`), never inserting a hand-made stack item.
Bots-free, 16 cases, `SUMMARY failures=0`.

```
CASE X_grizzly_bears_is_2 PASS counters=2 (mana_value of {1}{G} = 2), staged=1
CASE X_craw_wurm_is_6 PASS counters=6 (mana_value of {4}{G}{G} = 6)
CASE opponent_cast_does_not_trigger PASS staged=0 counters=0 (P1 cast, P0 owns the enchantment)
CASE noncreature_permanent_does_not_trigger PASS staged=0 counters=0 and the enchantment spell resolved
CASE landfall_makes_real_bear PASS token=Bear Token 2/2 green=true bear=true, counters_on_watcher=0
CASE no_legal_target_removes_trigger PASS removed=1 and the spell resolved; game digest still valid 5a86ea75cbf3
CASE two_enchantments_both_trigger PASS staged=2 counters=4 (2 triggers x X=2)
CASE countered_spell_still_sizes_trigger PASS countered spell still size-the-trigger: counters=2
CASE target_gone_at_resolution_is_harmless PASS target died before resolution (zone Graveyard); engine did not panic, counters on corpse=0
CASE serde_zero_is_byte_identical PASS clean json == pre-slice shape (no key), sized json has 1 key, digest stable 0089e3dbcf58
CASE slice20_demo_replay_digest PASS recorded digest reproduced 72cd69fb1143
CASE slice21_demo_replay_digest PASS recorded digest reproduced 0d20c7597183
CASE mana_value_matches_printed_cost PASS 134 costed cards agree with parse_cost; no hybrid/X/Phyrexian symbols in the pool
INFO hybrid_and_x_costs {G/W}=0 {2/W}=2 {W/P}=0 {X}{G}=1 {10}{G}=11
INFO dancing_mana_value=5 printed_cost=Some("{3}{G}{G}") pool_size=139
CASE pending_choice_has_no_priority_holder PASS pending=TriggerTarget priority=None validate=Ok reload_digest=afe8802cc0eb
CASE priority_after_plain_cast PASS priority=Some(0) Priority events=1 legal_actions=1
SUMMARY failures=0
```

Point by point:

* (a) X equals the cast spell's mana value for two different creatures: **2** for
  `{1}{G}` Grizzly Bears, **6** for `{4}{G}{G}` Craw Wurm, reached through a real
  cast plus a real `ChooseTriggerTarget`.
* (b) An opponent's creature spell does not trigger it: 0 triggers staged, 0
  counters, no pending choice appears (P1 cast, P0 owns the enchantment).
* (c) A noncreature permanent spell does not trigger it: `Bearscape`
  (enchantment spell) staged 0 triggers, left 0 counters, and still resolved onto
  the battlefield normally.
* (d) The landfall half makes a real token: one `Bear Token`, 2/2, green, has
  subtype `Bear`, and the cast-trigger watcher gains no counters from it.

## 4. Mana-value rule vs CR 202.3

`CardDef::mana_value()` sums the printed cost. For `{3}{G}{G}` it returns **5**;
`{1}{G}` → 2, `{4}{G}{G}` → 6, `{R}` → 1, `{U}{U}` → 2, lands (`cost: None`) → 0.

Pool-wide check (executed, not read): all **134** costed cards in the 139-card
catalog agree with `mana::parse_cost` (`generic + pips.len()`), and **no** cost
string in the pool contains a hybrid, Phyrexian, `{X}`, `{S}` or `{C}` symbol —
the pool is generic-plus-single-color only, so no pool card can make
`CardDef::mana_value` wrong today.

`{X}` is disclosed rather than silently wrong: the function maps `{X}` to 0, the
doc comment says so ("its value is fixed only while the spell is on the stack, and
no X-cost spell in this pool is a creature") and `docs/issue17-starter-corpus.md`
repeats it ("`{X}` counts 0"). Scryfall's ruling on this card
(`https://api.scryfall.com/cards/550cd0b6-ca61-4db7-9d20-0b68c48066f9/rulings`)
requires the chosen X, so the mapping is a **known, documented, currently
unreachable** limitation — not a silent error.

Card identity cross-checked against Scryfall: `hob` #123, `{3}{G}{G}`,
`Enchantment`, mythic, oracle text byte-identical to the catalog string
(including the em dash in "Landfall —"). The fixture records the immutable
printing `550cd0b6-ca61-4db7-9d20-0b68c48066f9` / oracle
`5ba482e9-fbb0-4d9f-a3a9-414892bcdfed`.

## 5. Serde / digest claim

* Both new fields are `#[serde(default, skip_serializing_if = "is_zero_u8")]`
  (`state.rs`: `TriggerRef::spell_mana_value`, `StackItem::spell_mana_value`).
* Executed: serializing a game with no sized trigger produces JSON that does
  **not** contain the substring `spell_mana_value` at all; after a real cast that
  stages the trigger, the JSON contains exactly one `"spell_mana_value":2`
  occurrence, and reloading that state reproduces the same digest.
* No recorded replay digest moved: two pre-slice replays from the evidence
  directory replay byte-exactly against this head —
  `evidence/issue17/slice20/demo-replay.json` → `72cd69fb1143…`,
  `evidence/issue17/slice21/demo-replay.json` → `0d20c7597183…`, each equal to the
  digest recorded in the file.
* `SessionSave::from_json` → `Game::from_state` round-trip of the pending-trigger
  state validates and keeps the digest (`reload_digest=afe8802cc0eb`).
* Note (not blocking): `SAVE_VERSION` stays 10. A save captured mid-pending with a
  non-zero value is only interpretable by this revision; an older build would
  ignore the unknown field (no `deny_unknown_fields`) and resolve with X = 0. No
  version was bumped, so this is forward-compat only.

## 6. Defect-class hunt (reproducible results)

| scenario | result |
|---|---|
| two copies of the enchantment on the battlefield | both trigger: `staged=2`, `counters=4` (2 × X=2) on the one chosen creature |
| the triggering creature spell is countered before the trigger resolves | trigger still resolves with **X=2**, taken from `StackItem::spell_mana_value`, not from the (gone) spell |
| the chosen target is gone at resolution (Shock kills the 2/2 in response) | no panic, no counters created, target in graveyard; effect's `zone == Battlefield` guard holds |
| the cast trigger has **no** legal target (Dancing is the only permanent) | `TriggerRemoved` fires once, no unanswerable pending choice, spell still resolves, digest valid |
| pending target choice + priority holder coexistence (the repaired defect) | after the cast, `pending=TriggerTarget` and `priority=None`, `state.validate()` is `Ok`, reload digest stable — the RED condition is gone |
| plain cast with no pending trigger | priority still granted to the caster (`Priority events=1`, decision owner P0) — the fix did not starve the ordinary path |

The repaired engine defect is real: `apply_cast` at b671414 set
`self.state.turn.priority = Some(player)` unconditionally, while `stage_triggers`
leaves `priority = None` when it opens `Pending::TriggerTarget`; `state.validate()`
rejects the combination with `"a pending choice cannot coexist with a priority
holder (CR 608.2d)"`. The retained RED log
`evidence/issue17/slice22/focused-2.log` records exactly that failure
(`save loads: Setup { source: InvalidState { reason: "a pending choice cannot coexist with a priority holder (CR 608.2d)" } }`),
and the same file now passes.

## 7. PR checks

```
$ gh api repos/wulfenbach-jpg/the-aperture/commits/c0e0a69…/check-runs --jq '.total_count'
10
```

10 check runs, **all `completed`/`success`**, all keyed to
`c0e0a69ad430e73d47b718ac8973ef464b32b632` — five job names
(`board and roadmap`, `windows package`, `rust (ubuntu-latest)`,
`rust (windows-latest)`, `rust (macos-latest)`) across two workflow runs on that
same SHA (run 40463221552 `push`, run 40463228154 `pull_request`), both
`conclusion=success`. Not running, not queued: green at review time.

## Findings that do not block

1. **Scope:** `README.md` (repo root) is edited, outside the stated allowlist
   (`crates/core` + tests/fixtures/decks/docs/evidence). Content is a correct
   138 → 139 count update.
2. **Latent doc/code mismatch in the new public API:**
   `CardDef::mana_value`'s doc comment and `docs/issue17-starter-corpus.md` both
   say a hybrid/Phyrexian symbol "adds its numeric half when it prints one and
   otherwise 1", but the code does `half.parse::<u8>().unwrap_or(0)`, i.e. a
   colour hybrid contributes **0**. Executed against synthetic defs:
   `{G/W}=0`, `{2/W}=2`, `{W/P}=0`, `{X}{G}=1`, `{10}{G}=11`.
   Expected per its own doc (and CR 202.3b: `{G/W}` = 1, `{W/P}` = 1).
   Unreachable today — no pool card has a hybrid or Phyrexian cost — so nothing
   in the shipped pool is wrong; the comment promises behaviour the code does not
   have, which will bite whoever adds the first hybrid Bear.
3. `SAVE_VERSION`/`REPLAY_VERSION` unchanged while a new non-zero save field
   exists (see §5) — acceptable, worth knowing.

## What remains unverified

* **The enchantment leaving the battlefield while its own trigger is on the
  stack.** Not reproducible with this 139-card pool: the only enchantment
  destruction in the pool is `Fade from History`, a sorcery, and the bounce effect
  targets creatures. Reasoning only (not executed): `StackItem` carries
  `spell_mana_value` and `targets`; the effect reads only those, never the
  source's zone, and the countered-spell case demonstrates source/most-recent-
  information independence for the spell half.
* **A creature spell with `{X}` in its cost** — no such card in the pool, and none
  can be added without revisiting `mana_value`.
* **Hybrid/Phyrexian costs** — latent mismatch above, unreachable in this pool.
* **Full workspace / native UI / cross-platform runs** — not run here by
  instruction; CI (both push and PR workflows) covers linux/macos/windows plus the
  Windows package job at the exact head.
* **Authoritative printed-text provenance beyond Scryfall** — the catalog string
  matches the Scryfall oracle text and the recorded immutable printing id; no
  independent second source was consulted.
* The lane's own claim that the increment is not yet accepted and the issue is
  unfinished is not something a review of this delta can confirm or refute; only
  the delta itself was reviewed.

## Reproduce

```
cd C:/AI/aperture-worktrees/issue-17
git rev-parse HEAD                                     # c0e0a69…
git diff --stat b671414..c0e0a69
cargo test -p aperture-core --test issue17_slice22 --locked
for t in card_printings catalog_pool issue13_commander_combat issue17_slice15_corpus abilities; do
  cargo test -p aperture-core --test $t --locked
done
gh api repos/wulfenbach-jpg/the-aperture/commits/c0e0a69ad430e73d47b718ac8973ef464b32b632/check-runs --jq '.total_count'
# independent probe (path-dep crate, public API only):
cd "$LOCALAPPDATA/hermes/cache/scratch/r22probe" && cargo run --release
```

The probe crate is scratch-only and was deleted after this review; its source is
reproduced by the case list in §3 and §6.
