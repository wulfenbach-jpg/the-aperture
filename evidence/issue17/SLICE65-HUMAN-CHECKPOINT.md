# Slice 65 — Siege-Gang Commander (first "Sacrifice a Goblin" card) + R364-1 last-known-power repair

Branch `aperture/issue-17`, draft PR 46. Code head **`a9844dc`** (product `3ef267a`,
journey/evidence commit `a9844dc`); local == remote == PR head at the time of this
checkpoint. `origin/master` untouched at `73820d7`.

## What changed

**R364-1 (functional, from review 364).** `Effect::CreateGoblinTokensEqualToSourcePower`
read the live source only and fell back to **0**, so Krenko, Tin Street Kingpin created no
tokens when it died while its own attack trigger was on the stack. CR 608.2h requires the
source's **last known** power. The effect now falls back to the existing
`StackItem::source_pt_lki` capture. The reason no card could use that capture before is a
real second defect found while fixing the first: the capture inside `detach_object` was
gated on `object.card == CardId::ExuberantWolfbear`. The gate is removed, so every
battlefield departure records last-known P/T for live stack items, trigger queues and
pending triggers. `deal_effect_damage` now passes the ability's controller into the damage
pipeline (`deal_damage_from`), so a source that has already left the battlefield cannot
silently disable a damage multiplier.

**Siege-Gang Commander (DMR 136, `{3}{R}{R}`, 2/2 Goblin).** Printed text fetched live
from Scryfall on 2026-10-29 (`fixtures/starter-corpus/slice65-sourced.json`, 18 printings,
**one** distinct oracle text): "When this creature enters, create three 1/1 red Goblin
creature tokens. / `{1}{R}`, Sacrifice a Goblin: This creature deals 2 damage to any
target." Note the current cost is **`{1}{R}`**, not the original Scourge printing's
`{2}{R}` — the card is implemented from the current wording, not from memory.

New shared primitive: `AbilityCost::sacrifice_creature_subtype` — a *chosen* creature cost
(CR 118.3, 701.21a). Validation requires a creature the activator controls with that
subtype; payment sacrifices it as the ability is activated, so it is a real zone change and
its dies triggers are recorded before the ability is put on the stack; `legal_cost_card_sets`
offers **one action per legal Goblin** (never a single hard-coded sample), so a policy that
answers with a different legal Goblin still holds an offer. The card may sacrifice itself
(it prints the subtype), and the damage still resolves afterwards.

**R364-2 (record).** `docs/issue17-starter-corpus.md` (lines 49, 54, 347), `README.md`
(30, 358) corrected to 177 catalog / 163 supported / 296 missing, and the missing slice-64
per-slice corpus line added beside the new slice-65 line.

**Corpus.** 177 catalog / 459 union = **163 supported + 296 missing**, per-list
`[55, 0, 51, 38, 0, 52, 52, 53]`, Krenko 39→38, still **2 of 8 certified**
(`evidence/issue17/slice65/corpus-audit.py` → `AUDIT OK`). Krenko's `missing_cards` **and**
`missing_interactions` were rewritten whole (`evidence/issue17/slice65/update-manifest.py`).

## Gates run in this slice

| gate | result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| focused binaries | slice65 5, slice65 journey 1, card_printings 24, catalog_pool 5, issue13_commander_combat 43, slice15_corpus 2, slice64 11, starter_corpus 8 — all 0 failed |
| R364-1 discriminating regression | mutating the LKI fallback back to `live.unwrap_or(0)` fails `krenko_tokens_use_last_known_power_when_it_dies_before_the_trigger_resolves` (0 passed / 1 failed, exit 101); restored, 5/5 green |
| four-seat journey (in-engine, offers only) | `issue17_slice65_journey`: 4 seats, 40 life, cast → 3 tokens → sacrifice a token → 2 damage to P2 → three Bears attack P3; digest `b2c8474c77b9…`, seeded 6565 |
| four-seat CLI journey (separate process) | `selfplay --seats 4 --seed 2525 --commander-combat-practice`, report `evidence/issue17/slice65/journey.json`: terminal `Win{P2}`, 1648 steps / 38 turns / 5571 events, digest `3ae78bd3a0c1…`; separate `aperture-cli replay --file …/journey-replay.json` → `matches recorded`, exit 0 |

**Not claimed:** the CLI journey's 5 casts were commander casts — the default heuristic
policy did not cast Siege-Gang Commander in that game, which is why the in-engine
offers-only journey exists as the changed-behaviour journey. A 4-seat selfplay with many
Siege-Gang Commanders is also slow in a debug build (more than 12 minutes at 2500 steps
before it was stopped); that performance observation is disclosed, not measured as a
regression.

## CI, stated exactly

* Code head **`a9844dc`**'s own runs were **cancelled** when the evidence checkpoint
  (`a03d088`) was pushed into the same workflow concurrency group: `a9844dc` shows
  1 completed success and 4 cancelled, which is NOT a green gate.
* The evidence tip **`a03d088`** — whose tree is identical to `a9844dc` apart from
  `evidence/` paths — has **10/10 completed success**, every run keyed to
  `a03d0882f3ab90e6c88d3fdfc5cd1f1de5aa9d6e` (receipt
  `evidence/issue17/slice65/exact-head-ci-a03d088.json`).
* This is the R364-3 lesson repeating in a new shape: pushing evidence starts its own
  runs and cancels the code head's in-flight runs, so the code head must not be
  described as green. The tree that passed is the tree under review; the reviewer should
  bind the verdict to `a03d088`'s own keyed runs and re-run the code head if it wants a
  run bound to `a9844dc`.

## Honest scope

* Original issue #17 remains **unfinished**: 2 of 8 lists certified, six planned lists with
  real gaps, and this slice moves Krenko 39→38.
* This is a bounded practice pool, not arbitrary Archidekt compatibility, not a strength
  claim, not a certificate.
* No completion, merge, master push, closure, dependent unlock, new team or global config
  change. Children `t_26a7a4ff` / `t_3c2fe51a` were inspected previously and stay gated;
  they are not increment-review children.
* R364-3: the failed `08f2a7e` push job (UI frame-sampling flake) was re-run on that same
  SHA at the start of this slice; its result is recorded in the review handoff, and this
  slice's evidence is held until the code head's own CI finishes.

## Next action

One independent, concise, practical delta review of `08f2a7e..a9844dc` (code head
`a9844dc`): the LKI/departure path and the new sacrifice cost are the two places where a
plausible implementation passes its own test and is still wrong. Then the same writer,
history, card, branch and draft PR 46 continues the remaining sacrifice-a-Goblin family
(Skirk Prospector, Siege-Gang Lieutenant, Goblin Trashmaster, Pashalik Mons) and the rest of
Krenko's 38 names.
