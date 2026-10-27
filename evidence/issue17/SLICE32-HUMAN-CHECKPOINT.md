# Slice 32 — Kamahl's Summons (ONS 269), Surrak and Goreclaw's printed Trample, and the mandatory-target regression

Date: 2026-10-27. Branch `aperture/issue-17`, draft PR46, one writer.

This slice carries **all four** corrections review 288 required before the next increment:

1. the next sourced card, **Kamahl's Summons (ONS 269)**, whose per-seat arbitrary hand reveal is resolved APNAP from the active player;
2. a real conformance fix for **Surrak and Goreclaw's own printed Trample** (it was a plain `card!` row while the card prints the keyword), with the journey-digest refresh it forces;
3. a **committed regression test** proving `DeclineTriggerTarget` is neither offered nor legal for a mandatory target trigger;
4. the previously untracked **`evidence/issue17/slice31/exact-head-ci-e56b138.json`** receipt, committed with this slice's evidence.

## Sourcing

| Card | Printing | Immutable id | Oracle id | Retrieved (UTC) | Fixture |
|---|---|---|---|---|---|
| Kamahl's Summons | Scryfall **ONS 269** | `0edc37c6-b6a8-424f-95dd-928d03c28542` | `8b6ef0a2-cd90-44c8-8f11-e1983f2b1dfd` | see `fixtures/starter-corpus/slice32-sourced.json` `retrieved_at` | `fixtures/starter-corpus/slice32-sourced.json` |
| Surrak and Goreclaw | Scryfall **MOM 337** | `9c10934d-9016-43c4-a7ab-56cc7d8f671f` | (existing slice-24 record) | same fetch | same fixture |

Printed text, verbatim (Kamahl's Summons): "Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green Bear creature token for each card they revealed this way."

Printed text, verbatim (Surrak and Goreclaw): "Trample\nOther creatures you control have trample.\nWhenever another nontoken creature you control enters, put a +1/+1 counter on it. It gains haste until end of turn." — the printing's `keywords` list is `["Trample"]`, which is exactly what the repaired row now models.

## What is new and reusable

* `Pending::ApnapReveal { player, order, revealed }` — the pool's first *per-seat arbitrary hand subset*. `order` is every live seat in APNAP order from the **active player** (CR 101.4, 608.2f), `revealed` collects the public answers.
* `Action::RevealCreatureCards { cards }` (CR 701.9a) — names any subset of the choosing seat's own creature cards. Empty is a real answer ("may reveal **any number**"). Refused with `EngineError::Illegal`, leaving the pending choice untouched, when: a named card is not in that seat's hand, a named card is not a creature card, the same card is named twice, or no reveal is pending at all.
* `Effect::EachPlayerMayRevealCreatureCardsForBears` + `Event::CardsRevealed` — resolution suspends on the queue and **applies nothing per answer**; `apply_apnap_reveal_batch` creates each seat's Bears, in APNAP order, only after the last seat answers, so no seat's count depends on the answering order.
* Revealing never moves a card (CR 701.9a): the cards stay in hand, which the focused test asserts by zone.
* Privacy: the pending reveal is projected only to the seat that owes it (`view_for` filter, same treatment as the APNAP sacrifice queue); the answers themselves are public and logged.
* `Surrak and Goreclaw` now uses the catalog's `card_trample!` slot, so `Game::has_trample` answers it with no special case.

## Required-but-forced plumbing (disclosed, not new behaviour)

The new `PendingKind` is a public enum, so the exhaustive matches in two other crates had to gain one arm each:

* `crates/ai/src/heuristic.rs` — one arm that takes the **last** offered reveal (the dominant "reveal every creature card" answer, since revealing costs nothing); `crates/ai/src/search.rs` — one pending-kind label.
* `crates/ai/tests/common.rs` and `crates/core/tests/common/mod.rs` — test-helper arms doing the same.
* `crates/ui/src/app.rs` — one arm rendering the offered reveal buttons (without it a human could not finish resolving the card).

No search policy, model, import or start-up behaviour changed. This is disclosed because earlier slices kept `crates/ai`/`crates/ui` untouched; collapsing the new pending onto the sacrifice kind was rejected because the human UI would then show a sacrifice prompt with no legal button.

## Verification actually run (logs in `evidence/issue17/slice32/`)

* `issue17_slice32` — **4/4** focused tests: catalog/printing agreement; a real three-seat cast where P0 reveals one card, P1 two and P2 none, tokens = 1/2/0, revealed cards still in hand, `CardsRevealed` events in APNAP order with counts `[1, 2, 0]`, no token before the last answer, and every refusal listed above; a mandatory targeted trigger (Little Bear) that never offers and always refuses `DeclineTriggerTarget`; and Surrak's printed Trample both as a flag and as real combat (6/5 blocked by a 2/1, 1 lethal assigned, 5 tramples to the player, life −5).
* `card_printings` — **2/2** new tests: Kamahl's Summons row equals the ONS 269 record (cost, type, oracle, printing id, retrieved stamp), and Surrak's row equals the MOM 337 record **including `keywords: ["Trample"]` and `CardDef::has_trample()`**.
* Four discriminating mutation REDs (`run-mutations.py`): revealed count ignored, tokens created before the last answer, Surrak's row losing its trample slot, mandatory target made declinable. Each fails exactly its named test, and every touched file is restored **byte-for-byte** (`pre-`/`post-mutation-sha256.txt`).
* Gates: `FMT_EXIT=0` after one `cargo fmt` pass, `CLIPPY_EXIT=0` (`--workspace --all-targets -D warnings`), `WORKSPACE_EXIT=0` with **118 suites / 833 passed / 0 failed**, `RELEASE_EXIT=0`.
* Four-seat / 40-life exact-snapshot journey from `evidence/issue17/slice32-journey-driver` (`decks/package-slice32.txt`, seed 3232, Jasmine Boreal injected for all four seats by the demonstrator): terminal **Win{P3}**, **1362 actions**, 32 turns, **3660 replay events**, digest `f6e65209dba9a24c84b05c3990e39691d5bd5cccc767efd41a5cb686d0fdbff4`, snapshot sha256 `5e2a53302c563390e8b03ee0b2f5193fd09f6ec1317d75bae2ea538d04540488`; 3 Kamahl's Summons casts × 4 seats = **12 reveal answers in APNAP order** (the order rotates with the active player each cast), **23 revealed cards → 23 Bear tokens**, 4 Surrak casts and **10 Surrak combat-damage events**. Re-running with `--locked` reproduces `demo-journey.json` `7142783d…`, `demo-save.json` `7c9b6d37…`, `demo-replay.json` `a95721c9…` **byte-identically**, and a separate release `aperture-cli replay` of the recorded replay matches the digest (`cli-replay.log`).
* **Journey-digest refresh the Surrak repair forces** (`replay-drift.sh` + `replay-drift.log`): every committed journey driver was rebuilt and re-run. Exactly one moved — slice 24: `a41bbfd2…` / 4127 events → **`697a846d6cd74fc0c13bb8f2c09d5da6d5fb0561ee6ba2a01e990c87a7e6ee91`** / 4130 events, same 1602 actions, 37 turns and the same decisions; its three regenerated artifacts are committed. Slices 18–23, 25, 26, 28, 29, 30 and 31 are **UNCHANGED**, which is the evidence that only Surrak's own damage assignment moved.
* Independent corpus audit (`corpus-audit.py`, recomputed from `catalog.rs` + the eight desired decks + commanders and asserted equal to every manifest `missing_cards` entry): **148 catalog / 459 union = 136 supported + 323 missing**, per list `55/0/51/48/17/52/52/53`, Ayula **17**, **1 of 8 certified**.

## Disclosures

* The journey is a **40-card package**, not a 100-card starter certificate, and the driver is an evidence-only masked demonstrator, not product AI or strength evidence.
* The reveal offer surface enumerates two deterministic extremes ("reveal no creature cards", "reveal every creature card in hand"); the action validates and accepts any subset. This is a bounded *offer* limitation, not a rules limitation, and is recorded in the docs.
* `crates/ai`/`crates/ui` plumbing as listed above.
* Slice 24's journey artifacts were regenerated. That is a record update, not a re-review of slice 24: the same driver, deck, seed and decisions, with only the trample assignment differing.
* Pre-existing, untouched: three tracked slice-23 evidence files carry uncommitted edits from earlier runs; they are not part of this slice's record.

## State after this slice

Original issue #17 remains **unfinished**: 1 of 8 desired 100-card lists certified, Ayula 17 names short, 323 of 459 union names unsupported. No merge, no master push, no issue closure, no downstream unlock (`t_26a7a4ff`, `t_3c2fe51a` stay parked). Next sourced card remains **Circle of the Moon Druid's neighbours in `evidence/issue17/slice27/AYULA-NEXT-PACKAGE.md`**, plus the previously queued Beorn's Hospitality, which still needs layer 7a.
