# REVIEW 290 — slice 33 (Beorn's Hospitality, first layer 7a characteristic-defining P/T)

Reviewed head: **`cff319d7174f86fbe8189e32a5c578a08279956b`** (code head `01fdc3b`; `e9ec8f5` adds the
slice evidence and `cff319d` the CI receipt line). Range `e165873..cff319d`. Branch `aperture/issue-17`,
draft PR46.

**Verdict: CHANGES_REQUESTED — one behavioural conformance defect in the new card, two false statements in
the slice record.** The increment's engine work (layer 7a, the non-expiring duration, the additive
animation, the targeted landfall watcher) is sound and reproduces: I rebuilt and re-ran it from my own
`git archive` copy, matched every digest, and re-derived the corpus numbers. The defect is in the printed
target surface of the new card, and it is reachable in ordinary play.

Lens this round: artifact first (cold read of `e165873..cff319d` before the handoff narrative), then
execution (out-of-tree rebuild + my own probes), then the contract re-check. Previous rounds led with
contract and execution, so the cold read is the new angle this round.

Mode: read-only. No deliverable file was edited by me. My artifacts are this directory:
`evidence/issue17/review290/` (`REVIEW290.md`, `r293-audit.py`, `r293-sum.py`→`r290-ws.py`,
`probe/` — a self-contained crate that compiles against this worktree's `crates/core`).

---

## 1. Blocking — the landfall trigger cannot target its own animated source (CR 115.1 / 603.3d)

**Where.** `crates/core/src/ability.rs:851-857` gives the landfall trigger
`TargetSpec::CreatureYouControl`; `crates/core/src/engine.rs:2862-2873` implements that spec as
"creature you control **and** `o.id != exclude`", and `stage_triggers`
(`engine.rs:4136`) passes `trigger.source` as `exclude`. That exclusion exists because it backs Little
Bear's printed "untap **another** target creature you control" (`ability.rs:688-698` says so explicitly).
Beorn's Hospitality prints no "another":

> Landfall — Whenever a land you control enters, put a +1/+1 counter on **target creature you control**.

`Effect::AddSourceCreatureTypeWhileOnBattlefield` makes the enchantment a creature, so from that moment it
is a legal target of its own trigger — and it is exactly the permanent that the new layer-4 half creates.
Reusing the "another" spec makes the card's own text wrong in a state this slice introduces.

**Reproduced (my probe, run against this worktree's core):**

* `probe/tests/slice33_review_probe.rs::lone_animated_hospitality_loses_its_own_landfall_trigger` —
  Beorn's Hospitality animated with no other creatures, then a land played:
  `Event::TriggerRemoved { card: BeornsHospitality, reason: "it requires a target and no legal target
  exists (CR 603.3d, …)" }`, no counter, P/T stays `3/3` (three lands).
* `probe/tests/slice33_review_probe2.rs::landfall_wrongly_excludes_its_own_animated_source` — with one
  other creature on the battlefield, `legal_targets(P0, TargetSpec::CreatureYouControl, source)` returns
  `[Permanent(ObjectId(2))]` (the Grizzly Bears only). The animated source is filtered out, so the counter
  goes to the Bears and can never go to the card itself.

The slice's own prose claims the opposite guarantee: `SLICE33-HUMAN-CHECKPOINT.md:25` and
`docs/issue17-starter-corpus.md` ("the trigger asks only for the watcher's controller's own creatures and is
not put on the stack when none exist (CR 603.3d)"). The "none exists" case is computed with a filter the
card does not print — that is why the claim looked verified by the slice's tests: the tests only ever place
a *different* creature as the sole legal target.

**Why this violates the task.** Criterion 2 ("every included card and required interaction has an
implementation and conformance coverage") is not met for this card while its own printed target ("target
creature you control", no "another") is narrowed to "another creature you control".

**Minimum outcome.**

1. A mandatory "target creature you control" spec that does **not** exclude the ability's source, used by
   this trigger; Little Bear's printed "another" must keep excluding it. (A new `TargetSpec` variant, or a
   per-ability flag threaded into `stage_triggers`, both work — the choice is the implementer's.)
2. A regression test that proves the animated Hospitality is offered/auto-chosen as its own landfall target
   and receives the counter, and that the **unanimated** enchantment is not a legal target (it is not a
   creature). Cover the zero-legal-target path explicitly so the CR 603.3d branch stays honest.
3. Correct the "with none it does not put the trigger on the stack" sentence in the checkpoint and the docs
   so it states the rule the engine now implements.

---

## 2. Required record corrections (same files this slice already edits)

**R290-2 — the `card_printings` count is wrong.** `SLICE33-HUMAN-CHECKPOINT.md:32` says
"`card_printings` — **13/13**". The target has **12** `#[test]` functions at this head
(`grep -c '#[test]' crates/core/tests/card_printings.rs` → 12) and
`cargo test -p aperture-core --test card_printings --locked` reports **12 passed / 0 failed** here
(review 289 recorded 11 before this slice added its one test). Restate as 12/12, or name the extra test if
one exists in another target.

**R290-3 — the tip's CI statement is wrong, though the outcome is fine.**
`SLICE33-HUMAN-CHECKPOINT.md:50` says the receipt-line commit `cff319d` "inherits the `e9ec8f5` runs and has
no runs of its own". `cff319d` has **10 check runs of its own** (two each of `board and roadmap`,
`rust (ubuntu|windows|macos-latest)`, `windows package`). They have now finished: **10/10 completed
SUCCESS, every `head_sha` = `cff319d`** (live `gh api …/commits/cff319d/check-runs`). Record that instead of
the inheritance claim; a verification record must not describe runs that do not exist the way it says they
do.

Both are the "published count next to an updated count" class of defect the project has been treating as
real, not cosmetic, and both live in files this increment already touches.

---

## 3. Non-blocking observations (record, no hold)

* **N1 — layer order between the *printed* 7b static and the new 7a pass.** `recompute_layers`
  (`engine.rs:512-537`) applies `StaticAbility::SelfBasePowerToughnessDuringYourTurn` (Circle of the Moon
  Druid) **before** the new 7a block (`engine.rs:543-565`), and the 7a block overwrites `pump_power`. An
  object carrying both would therefore have its characteristic-defining value beat its base-P/T setter,
  which is the reverse of CR 613.4a → 613.4b. Unreachable today (no pool card prints both, and the printed
  static only applies to its own card), so this is a disclosure or a later reorder — not a hold.
* **N2 — duplicate animation.** Disclosed at `SLICE33-HUMAN-CHECKPOINT.md:43`; my probe confirms it is
  genuinely idempotent (two activations → four effects, still `4/4` with four lands, no stacking).
* **N3 — mid-animation persistence.** Not claimed by the slice; I checked it anyway: a mid-animation
  `GameState` JSON round-trip keeps both effects with layer/kind/`WhileSourceOnBattlefield`, stays a
  creature, and still reads the land count (`probe/tests/slice33_review_probe2.rs`).
* **N4 — colour wording.** `card_printings.rs` asserts `def.colors == [Green]` but the assertion message
  says "the printing's colour identity is green"; `colors` and `color_identity` are different Scryfall
  fields (they coincide here). Wording only.

---

## 4. Gates I ran on this machine, this head

| Command | Result |
|---|---|
| `cargo fmt --all --check` | exit 0, no output |
| `cargo test -p aperture-core --test issue17_slice33 --locked` | **7 passed**, exit 0 |
| `cargo test -p aperture-core --test card_printings --locked` | **12 passed**, exit 0 (checkpoint says 13) |
| `cargo test --workspace --locked` | exit 0 — **119 suites, 841 passed, 0 failed, 1 ignored** (the pre-existing network Archidekt test) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| my probes (`evidence/issue17/review290/probe`) | **7 passed**, exit 0 |

Mutation receipts check out mechanically: `pre-mutation-sha256.txt` = `post-mutation-sha256.txt` =
`sha256sum` of the current `engine.rs` (`14554dec…`) and `ability.rs` (`c17b9d78…`), and each of the four
RED logs ends at exit 101 on the named tests.

## 5. Execution — reproduced from my own copy, not from the handoff

`git archive cff319d` → `C:/AI/r293-review`, driver bytes untouched, own target dir:

* `cargo run --release --locked` in `evidence/issue17/slice33-journey-driver`: exit 0, terminal
  **`Win{P3}`**, **1117 actions**, **24 turns**, **2967 replay events**, digest
  `3dfb14d4ce762cdcf52bd9c8d5f1832cebe3dbd4584b21c4cd1dff57dfd14c22`, snapshot
  `a7a45e4b…7c581d`, 6 casts of the card, 2 animation activations, 4 `ContinuousEffectStarted`, 13 landfall
  triggers, **29 on-turn / 28 off-turn** invariant sightings.
* `demo-journey.json` / `demo-save.json` / `demo-replay.json` regenerate **byte-identically**
  (`1ee05e63…`, `c040689b…`, `e47bb078…`).
* A separate `aperture-cli` I built myself: `replay --file demo-replay.json` → `replayed digest
  3dfb14d4ce76 | matches recorded | events 2967`, exit 0.

## 6. Corpus and contract audit (my own numbers)

`r293-audit.py`: pool names taken from a probe over `catalog::all()` (149 entries, unique), union taken
from the eight manifest starters (deck file lines **plus** each list's commander), and every manifest
`missing_cards` entry asserted equal to the computed set:

```
pool 149  union 459  supported 137  missing 322
per_list_missing [55, 0, 51, 48, 16, 52, 52, 53]   certified 1 of 8
```

That matches the checkpoint, the docs and the metadata exactly. `crates/ai`, `crates/ui` and the desired
deck snapshots are untouched by the delta (`git diff --stat` empty for those paths); the only deck change is
one manifest line removing `Beorn's Hospitality` from Ayula's missing list. Live Scryfall
`/cards/named?exact=Beorn's Hospitality` (fetched this session) returns `hob 120`, `{1}{G}`,
`Enchantment`, colours `["G"]`, id `153ca57e-30f0-4ad7-ae9d-c55cbf0fd4c9`, and oracle text identical to the
fixture and the catalog row.

CI: `e9ec8f5` — **10/10 check runs completed SUCCESS, every `head_sha` = `e9ec8f5`** (verified live, twice
each context). `cff319d` — **10/10 SUCCESS keyed to `cff319d`** (see R290-3).

## 7. Contract status of the original task

Unchanged by this slice: **criterion 1 is still 1 of 8 certified 100-card lists** (Ayula 16 names short, the
other six 48–55 short) and **criterion 2 still has 322 of 459 union names unsupported**. The four-seat
journey is a 40-card evidence package (`decks/package-slice33.txt`), not a desired starter certificate, and
the driver is an evidence-only demonstrator, not product AI. No merge, master push, closure or downstream
unlock was performed by me, and `t_26a7a4ff` / `t_3c2fe51a` stay parked.
