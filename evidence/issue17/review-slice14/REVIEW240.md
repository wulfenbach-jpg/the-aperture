# Issue 17 — independent review run 240 (slice 14 delta, round 4 / contract lens)

Reviewer: independent review lane, same card `t_67778feb`, same draft PR46.
Reviewed head: `c872a82d277353884840785be74417eb11f71d16` (local == remote == PR head).
Delta scope: `a4db05f..c872a82` (commits 963850a, c30a336, 472aeba, eb516e4, c872a82).
No implementation file was edited by this review.

## Verdict

Implementation and evidence: **approved**. Published records in the delta: **corrections required**
(R240-1 … R240-4). The original #17 acceptance is **not** met — 1 of 8 desired starters is
certified — so this returns to the same implementer for the remaining sourced work.

## What was independently executed (not read from the handoff)

| Check | Result |
|---|---|
| `gh api …/commits/c872a82/check-runs` | 10/10 `completed/success`, every run keyed to `c872a82` |
| `gh pr view 46` | OPEN, draft, `MERGEABLE`, head `c872a82` |
| focused tests (`issue17_slice14` 10, `issue17_slice13` 4, `issue17_tokens` 10, `catalog_pool` 5, `card_printings` 7, `issue13_commander_combat` 43) | 79 passed, 0 failed |
| `cargo test --workspace --all-targets --locked` | 80 suites, **613 passed, 0 failed, 1 ignored** |
| `cargo fmt --all --check`, `git diff --check` | exit 0 (only pre-existing evidence-jsonl LF/CRLF warnings) |
| release CLI `replay --file evidence/issue17/slice14/demo-replay.json` | 2371 actions, 6665 events, replayed digest `028cc2e33168` == recorded, 4 seats, outcome `Win{P0}` |
| snapshot hash of `decks/package-slice14.txt` (raw and LF-normalised) | `8a02ad41…c8287a7` == `demo-journey.json.snapshot_sha256` |
| independent corpus audit (own script, manifest vs `catalog.rs` vs desired deck files) | catalog 126; union 459 = 114 supported + 345 missing; per-list missing equals `names - catalog` for all 8; all eight libraries parse to 99 cards; only `decks/starter-corpus/manifest.json` changed since base `e16185c` |
| live Scryfall re-read of the four printing URLs in `fixtures/starter-corpus/slice14-sourced.json` | mana cost, type line, P/T, oracle text byte-equal to the catalog strings; printing/set/collector-id match the fixture; all Commander-legal |
| `recompute_layers` | pump reset before layering (lines 459-470), threshold added as an additive 7c static, documented commutation argument holds |
| diff surface | README, `crates/core` src/tests, `decks/package-slice14.txt`, manifest `missing_cards` removals only, docs, fixtures — no import/start UI, no AI/model file, no desired snapshot change |

The two first-cut defects the handoff claims to have repaired are covered by behaviour tests that
run against real `legal_actions` output rather than direct submission
(`ready_werebear_is_offered_but_sick_or_tapped_is_not`,
`each_offered_dynamic_mana_action_validates_and_tapped_sources_offer_none`).

## Findings (all documentation/record, no product behaviour defect)

* **R240-1 (in-delta, factual).** `docs/issue17-starter-corpus.md:59` still says Ayula has
  **42** missing names. This slice removed exactly three Ayula names (Arcane Signet, Fellwar
  Stone, Werebear); the manifest and `evidence/issue17/slice14/corpus-state.json` both say **39**,
  independently confirmed. The same document already reports 126 / 114 / 345 correctly, so it now
  contradicts itself about the list this slice changed. The card's acceptance criterion 4 is about
  documenting exclusions, so this must agree with the manifest.
* **R240-2 (in-delta, evidence reconciliation).** `demo-journey.json` reports 4 Krenko, Mob Boss
  casts because `evidence/issue17/slice14/journey-driver/src/main.rs:15` injects Krenko as the
  commander of all four seats. `SLICE14-HUMAN-CHECKPOINT.md` lists only the four new cards' casts
  and never states that a commander was injected (it is only discoverable from the replay's
  top-level `commanders`). Disclose the injected commander and reconcile the table with the JSON.
* **R240-3 (carry-over, half-applied).** Run 234's non-blocking corrections landed only partly:
  docs now say CR 702.14c for the landwalk blocking restriction while every code comment, test and
  `SLICE11-HUMAN-CHECKPOINT.md` still cite 702.14b for the same rule; and the slice-11 checkpoint
  still says "5 focused tests" where `crates/core/tests/issue17_slice11.rs` contains 6. The pinned
  rules text settles which citation is right: `evidence/issue17/slice2-rules-source.txt:4030-4034`
  gives 702.14b as "Landwalk is an evasion ability" and 702.14c as "A creature with landwalk can't
  be blocked as long as the defending player controls at least one land with the specified land
  type", so the enforcement sites (`card.rs:274,312`, `engine.rs:2255,2270,2317`, `tokens.rs:49,53,
  224`, `catalog.rs:182`, `issue17_slice11.rs`) are mis-citing 702.14b where 702.14c (or 702.14a
  for the generic-term definition) is correct.
* **R240-4 (fixture note).** `fixtures/starter-corpus/slice14-sourced.json` `note` says "three
  records only" while the file holds four.

Non-blocking observations: threshold is applied before the timestamp-ordered 7c one-shots — sound
today because every 7c effect in the pool is additive, but it will need timestamp ordering if a
7c *setting* effect is ever added; Fellwar Stone reads only the modelled typed basic mana of
opponent lands (documented); the product heuristic still casts no artifacts and auto-activates no
nonland mana (unchanged, no AI edit).

## Original #17 contract audit (round-4 contract lens)

| Criterion | State at `c872a82` |
|---|---|
| ≥8 explicitly supported 100-card lists | **unmet** — 1 certified (`jasmine-vanilla-stompy`), 7 `planned` |
| every included card and interaction implemented + conformance coverage | **unmet** — 345 of 459 union names unsupported |
| four-seat games on exact deck snapshots, never bundled replacements | met for the one certified list; the slice-14 journey is an explicitly-labelled 40-card package |
| per-card/interaction exclusions documented; Archidekt compatibility an explicit non-goal | documented, but R240-1 makes the published counts wrong |

## Actions not taken

No merge, no master push, no force push, no issue closure, no child-card (`t_3c2fe51a`)
activation, no provider/model or global configuration change, no build-cache cleanup.
