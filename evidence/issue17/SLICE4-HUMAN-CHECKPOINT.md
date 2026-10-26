# Issue17 — slice 4 checkpoint (Flash / Trample / another-creature-enters package)

Full issue17 is **UNFINISHED**. This is a bounded implementation increment on the
existing draft PR46, not eight certified starter lists. No merge, master push,
issue closure or successor activation was performed.

## Published exact head

* draft PR46: https://github.com/wulfenbach-jpg/the-aperture/pull/46
* **code head of record (reviewed + CI): `eea8f9dea34ae9b5f65c4bda3a1f50797449a118`**
  (previous reviewed head `86aebf9`; previous approved increment `9cecd62`)
* branch tip `ca23edb` is the **evidence-only** follow-up commit for this
  checkpoint: `git diff --name-only eea8f9d ca23edb` touches 0 non-evidence paths
  (6 files, all under `evidence/issue17/`), so the reviewed code bytes are
  unchanged. A head-bound supervisor should bind approval to `eea8f9d` and treat
  `ca23edb` as its evidence delta.
* PR OPEN, MERGEABLE, still draft, `mergeStateStatus: BEHIND` (branch is behind
  master by other lanes' commits; no engine drift seen locally).
* exact-head CI: **10/10 check-runs SUCCESS** on `eea8f9d`
  (`evidence/issue17/slice4/slice4-final-ci-check-runs.json`): board and roadmap,
  rust ubuntu/windows/macos (two runs each), windows package.
* Independent practical review (same saved reviewer history):
  * round 1 `request_changes` on `86aebf9` — real defect **R211-1**: creature
    tokens skipped the "another creature enters" watcher scan.
  * round 2 `approve_increment` on `eea8f9d`, **0 findings**, R211-1 re-tested by
    the reviewer's own probe deck
    (`evidence/issue17/slice4/slice4-review-1.json`, `slice4-review-2.json`).

## What is usable now (real behavior, sourced)

| card | printing | new behavior |
|---|---|---|
| Ordinary Bear | hob 133 | vanilla 4/5 green creature (last printed vanilla in the union) |
| Ashcoat Bear | tsp 190 | printed **Flash**: the engine allows instant-timing casting for a card that prints Flash (CR 702.8a); a non-Flash creature is still refused there |
| Owlbear | afr 198 | printed **Trample** plus its enters trigger: lethal assigned to each blocker, surplus trampled to the defending player (CR 702.19b/510.1c); non-trample assignment unchanged |
| Soul Warden | mm3 24 | first **"another creature enters"** trigger in the pool (CR 603.6a): fires for creatures its controller does not control, never for itself |

Live Scryfall records (named+exact, plus `unique=prints` to prove the wording is
identical across every printing) are in
`fixtures/starter-corpus/slice4-sourced.json`; retrieval timestamp is recorded
per card. Ashcoat Bear has 1 printing, Owlbear 3 (1 distinct text), Soul Warden
16 (1 distinct text), Ordinary Bear 1.

## Corpus movement (native recomputation)

Script `evidence/issue17/slice4/update_manifest.py` recomputes every desired
list's `missing_cards` from the compiled catalog and asserts each desired deck
file is exactly 99 cards:

* catalog **93 → 97** real cards;
* eight-list union **459** exact names (unchanged — the desired lists were not
  edited);
* supported **81 → 85**, missing **374** (was 378);
* per list: isamaru 56 (was 57), ayula 47 (was 50), others unchanged;
* **1 implemented / 0 certified** starters; the seven planned lists stay planned.

Evidence: `evidence/issue17/slice4/slice4-corpus-state.json`.

## Verification actually run

* `crates/core/tests/issue17_slice4.rs` — **13** focused tests (10 + 3 added for
  R211-1: token entry, two-blocker trample, pre-damaged-blocker trample).
* **Genuine RED (round 1)**: with only the three engine hunks reverted
  (`git checkout -- crates/core/src/engine.rs`) exactly the four behaviour tests
  fail (`printed_flash_allows…`, `trample_assigns_only_lethal…`,
  `soul_warden_gains_life…`, `soul_warden_triggers_for_an_opponents…`) while the
  non-trample and no-self-trigger controls pass —
  `evidence/issue17/slice4/slice4-red-before-engine.log`; the engine diff is
  preserved as `engine-slice4.diff`. Restored afterwards: all green.
* **Genuine RED (R211-1)**: with only the token hook disabled,
  `soul_warden_triggers_for_creature_tokens_too` fails —
  `evidence/issue17/slice4/slice4-red-R211-1-token.log`. Restored afterwards.
* `cargo test --workspace --locked` → **515 passed, 0 failed** (log
  `slice4-workspace-tests.log`).
* `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo build --workspace --release --locked` → all exit 0
  (`slice4-fmt.log`, `slice4-clippy.log`, `slice4-release.log`).
* Real executable journeys (both **terminal**, both with separate replay parity):
  1. `decks/package-ayula-slice4.txt`, four seats, 40 life, seed 17 → winner P1,
     1567 steps / 43 turns, 4209 events, digest `5e905fdc0399…` (unchanged by the
     R211-1 repair — `slice4-game-after-fix.json`).
  2. `decks/package-soul-warden-tokens.txt`, four seats, 40 life, seed 23 →
     winner P3, 1680 steps / 43 turns, 4662 events, **29 triggered abilities**,
     digest `e98d6253de68…` (`slice4-warden-game.json`).
  Both are **40-card package fixtures, not desired 100-card starters**.
* Provenance and pool conformance guards extended:
  `every_slice4_card_matches_its_recorded_printing_and_keywords` (label,
  power/toughness, mana cost, oracle text, and the Flash/Trample flag against the
  printed keyword list) and a keyword-only branch in
  `every_printed_ability_maps_to_implemented_semantics` that only accepts
  keywords the engine actually models.

## Honest limitations

* Trample assignment is still automatic (no damage-assignment choice is
  exposed); a blocked trampler whose blockers have all left the battlefield
  still deals no player damage, matching the pre-existing documented
  approximation for blocked creatures.
* Flash models the printed keyword only; nothing yet grants Flash to another
  card, and no "cast as though it had flash" effect exists.
* "Another creature enters" is source-scoped and unconditional: no intervening
  "if", no "nontoken" wording, and no optional ("you may") form yet.
* Keywords still absent from combat: flying is partial (existing), and haste,
  vigilance, reach, first strike, lifelink, deathtouch, menace are NOT modelled.
  Cards that print them remain unsupported and refused at import.
* Counters as rules objects, legend rule, artifacts/enchantments, tutoring,
  milling, token types beyond Goblin/Drake and single-target spell copying are
  still unimplemented — the eight desired lists cannot be certified.
* Shared core hooks (`card.rs`, `catalog.rs`, `engine.rs`, `state.rs`) remain an
  integration hotspot with the separate #16 lane; this work does **not** approve
  #16 integration.
* Reporter pointer still targets completed issue20 and was not changed here.
* Tests for the new rules were written in the same run as the engine change; the
  RED run above was produced by reverting the engine hunks, and no earlier
  per-rule RED is claimed.

## Exact resume state (next slice)

* Saved OMP histories (complete), backed up before this run at
  `evidence/issue17/slice4-backup-run211/`:
  * writer `evidence/issue17/sessions/2026-10-25T07-56-17-971Z_01a1720f-74b3-7000-bb34-cf8988951469.jsonl`
  * reviewer `evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl`
* Run overlay `.starter-run.yml` pins every role to
  `opencode-go/deepseek-v4.1-flash` with empty fallback chains (no paid route).
* Reviewer assignment for this increment:
  `evidence/issue17/SLICE4-ASSIGN-REVIEW-1.md`; verdict files
  `evidence/issue17/slice4/slice4-review-1.{json,md}`.
* Next concrete package (from the recomputed manifest, most reuse per card):
  Ayula's remaining bears that need only one new primitive each —
  `Sazh's Chocobo`/`Floral Evoker` (landfall +1/+1 counter on itself) and
  `Werebear` (`{T}: Add {G}` creature mana ability) are the cheapest, but both
  need counters or non-land mana abilities, which is the #16 hotspot and must be
  integrated serially. Zada needs spell copying; Isamaru needs artifacts and
  equipment; Elas needs sacrifice/drain. Recommend the coordinator pick one
  primitive per slice and keep it inside one lane.
