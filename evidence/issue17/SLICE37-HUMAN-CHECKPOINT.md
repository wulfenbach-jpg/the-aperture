# Slice 37 — Druid's Familiar (AVR 175), the pool's first soulbond pair

Card / branch / PR are unchanged: kanban `t_67778feb`, branch `aperture/issue-17`,
draft PR46 (https://github.com/wulfenbach-jpg/the-aperture/pull/46). Approved
increments 1–36 were **not** re-audited; this slice answers review 298's
"continue Ayula to zero" direction with the next sourced name.

* **Code head of record:** `d5ccc15` (implementation + tests + fixture + docs +
  the two pool-size guards).
* **Evidence tip:** `108df74` (the journey driver and its artifacts).
* **Record-only tip:** `07ad00e` commits the 10/10 exact-head CI receipt
  (`evidence/issue17/slice37/exact-head-ci-07ad00e.json`, all ten check runs
  `completed/success`, every `head_sha` = `07ad00e`); it carries no code, test or
  fixture byte.
* PR stays OPEN, **draft**, `BEHIND` master. No merge, master push, issue
  closure, downstream unlock, new team or global config change.
* **Original issue-17 contract remains unfinished**: 1 of 8 desired 100-card
  lists certified, Ayula **14** names short, 320 of 459 union names unsupported.

## What was added

`Druid's Familiar` (`{3}{G}`, 2/2 Creature — Bear) — the fourteenth of Ayula's
remaining fifteen names, and the pool's first **Soulbond** card (CR 702.94a).
Sourced from Scryfall **AVR 175**: immutable printing id
`1d8c794a-7964-41f0-bc9c-27af7cf87aaa`, oracle id
`6a5b1cd4-e597-4122-8fc7-4dce241d7c01`, retrieved **2026-10-27T23:16:27Z**. Both
printings (`avr 175`, `plst AVR-175`) carry one identical text. Raw responses,
the printings list and the retrieval stamp are in
`evidence/issue17/slice37/sources/`; the enforced fixture is
`fixtures/starter-corpus/slice37-sourced.json` and `card_printings` compares the
catalog row against it.

Printed text:

> Soulbond (You may pair this creature with another unpaired creature when either
> enters. They remain paired for as long as you control both of them.)
> As long as this creature is paired with another creature, each of those
> creatures gets +2/+2.

### Reusable surfaces, not a one-card patch

* **Pairing is state, not a continuous effect.** `Object::paired_with`
  (`#[serde(default, skip_serializing_if)]`, so every earlier save, replay and
  digest is unchanged) stores the partner, and every reader goes through
  `Game::paired_with`, which re-validates zone, shared controller and the mutual
  back-reference. No effect can be orphaned by a bounce, a reload or a stale id.
* **Both printed trigger events.** Two triggered abilities implement "when either
  enters": this creature's own entry (CR 603.6a) and another creature entering.
  Both carry the printed clause verbatim, and `catalog_pool` enforces that.
* **A dedicated choice spec.** `TargetSpec::UnpairedAnotherCreatureYouControl`
  is the printed set — another, unpaired, controlled by the same player, source
  excluded. It is the second spec the engine treats as declinable, so declining
  the pairing choice *is* the printed "you may" and the surface always offers it.
* **The bonus is layer 7c and applies to both members**, re-read on every layer
  pass from the validated pair; two paired copies each grant their own +2/+2.
* **The pair ends at one point.** `Game::detach_object` — the single chokepoint
  every battlefield exit goes through — clears the partner's back-reference
  (CR 400.7, "for as long as you control both of them").

## Verification (all run in this workspace)

* `crates/core/tests/issue17_slice37.rs` — **6/6**: sourced text and both
  triggers; entering alone pairs with nothing and stays 2/2; entering beside one
  unpaired creature offers the pairing, the source is not offered, declining is
  offered, and both members become 4/4; declining leaves both unpaired; a later
  creature entering pairs with the already-present familiar; **killing the
  partner ends the pair and the familiar drops back to 2/2**.
* `card_printings` — **14/14** (new provenance record + the exhaustive
  "every post-issue-15 pool card has a recorded printing" guard).
* Workspace `cargo test --workspace --locked` — **121 suites / 860 passed / 0
  failed**, exit 0 (`evidence/issue17/slice37-workspace-tests.log`).
* `cargo fmt --all --check` exit 0; `cargo clippy --workspace --all-targets
  --locked -- -D warnings` exit 0 (`evidence/issue17/slice37/gates.log`).
* **Four-seat journey** `evidence/issue17/slice37-journey-driver` over the exact
  `decks/package-slice37.txt` snapshot (4 seats, 40 life, seed 3737, only masked
  legal actions from `view_for`): terminal **Win{P3}**, **1758 steps / 42 turns
  / 4530 replay events**, digest **`c4c5985cd048…`**, Druid's Familiar cast
  **twice**, **two soulbond pairings created**, and **574 pair-invariant
  observations** — every live pair mutual through `Game::paired_with` and both
  members carrying at least the printed +2/+2 over their own printed
  characteristics. Two runs regenerate all three artifacts byte-identically
  (`driver-sha256-first/second.txt`). A fresh `SessionSave` reloads to the exact
  state. A **separate release CLI** replay of the driver's own replay reports
  `digest c4c5985cd048 | matches recorded | events 4530`.
* Corpus audit (my own script): catalog **151**, union **459** = **139 supported
  + 320 missing**, per list `[55, 0, 51, 48, 14, 52, 52, 53]`, Ayula **14**,
  **1 of 8 certified**, manifest `missing_cards` equal on all eight.

## Review 298's record items

* Stale head line in `SLICE36-HUMAN-CHECKPOINT.md` — fixed: code head `7bb8197`
  and evidence tip `be56111` are now named separately.
* `evidence/issue17/slice35/exact-head-ci-be56111.json` — now committed rather
  than implied.
* `slice35/driver-reproducibility.log` — labelled: the recorded hashes are this
  checkout's `sha256sum` values, the git-blob (LF) hashes are named beside them,
  and a fresh export is expected to report false FAILEDs on line endings.

## Disclosed limits

* Soulbond's pairing choice happens as the trigger resolves and is not a target;
  it is offered through a dedicated choice spec rather than the targeting
  system. Nothing in the pool has shroud or hexproof, so no interaction between
  the two is observable today.
* No control-changing effect exists in the pool, so the "as long as you control
  both" half is enforced at the exit point plus the validating query rather than
  by a control-change test.
* The journey is a 40-card evidence package and a masked demonstrator, not a
  100-card certificate and not product AI.
* In this journey both pairs survived to the end of the game, so the *ending*
  half is demonstrated by the focused test (partner killed) rather than by the
  game; the driver reports the count instead of demanding an occurrence.
* The evidence commit that recorded the workspace run also swept in
  pre-existing untracked files under `evidence/issue17/` (session logs, review
  snapshots). They are unchanged historical artifacts, not new work.
