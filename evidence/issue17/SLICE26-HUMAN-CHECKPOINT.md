# Issue17 — slice 26 reviewed increment (original eight-list contract still unfinished)

## Current candidate

- Same native card `t_67778feb`, checkout `C:/AI/aperture-worktrees/issue-17`, branch `aperture/issue-17`, draft PR46.
- Code head `9cd0b68` (the only behavior change); record/docs head `80fff73` (comment-only source edits with no behavior change in `crates/core/src/{ability,catalog,engine}.rs` and `tests/issue17_slice26.rs`, plus docs and regenerated mutation evidence).
- Both heads are pushed; local head == remote head (`git ls-remote` readback). No merge, master push, issue closure, downstream activation, new team or global config change.
- Final exact-head CI for `80fff73`: **10/10 completed SUCCESS, every `head_sha` = `80fff7357c31ebaf54d4c0f3ca8907e91dbccce7`**, all four strict required contexts (`rust (ubuntu-latest)`, `rust (windows-latest)`, `rust (macos-latest)`, `board and roadmap`) PASS with the configured Actions app `15368`, captured 2026-10-27T10:42:10Z in `slice26/exact-head-ci-80fff73.json` / `exact-head-ci-summary-80fff73.json` by `slice24/capture-ci.py` (which exits non-zero until all ten are green and head-bound; exit 0 recorded in `slice26/ci-capture-80fff73.log`). No green is inherited from `9cd0b68` or any earlier head.

## What became usable

**Ursine Champion** (M19 304, Scryfall `e60b6c02-c1f1-4ab9-bd62-ae3b447a898f`, oracle `4f142d99-8146-4b16-a2cc-ac03da19b960`, retrieved 2026-10-27T09:59:58Z; raw response, retrieval stamp and normalized fixture `fixtures/starter-corpus/slice26-sourced.json` retained): `{5}{G}: This creature gets +3/+3 and becomes a Bear Berserker until end of turn. Activate only once each turn.`

- The subtype change is the pool's first **layer 4** (CR 613.1d) continuous effect: `Layer::Type` plus a typed `layer::SubtypeLine` (a closed enum, so it stays replay-serializable) and `Effect::SetSourceCreatureSubtypes`.
- It **replaces** the printed subtype line for every rules question, so `Game::has_subtype` now answers the Bear/Human subtype questions previously read straight off `CardDef`: the `TargetSpec::Bear` / `Bear`+control and `HumanYouControl` filters, Ayula's fight revalidation, `Little Bear`'s counter branch, and the Surrak "entered Bear" filter.
- `Effect::PumpSource(3,3)` and the type change resolve in printed order (CR 608.2c) but apply in **layer** order (CR 613.1), so the pump lands after the new type line. Cleanup removes both (CR 514.2).
- The printed once-each-turn limit reuses the restriction `Rootwalla` already prints; this card adds no new activation-limit machinery.

## Verification actually run (native, this slice)

- `crates/core/tests/issue17_slice26.rs`: **6/6 pass** — catalog/printing and typed-ability equality; +3/+3 and the effective Bear subtype proven through Little Bear's real enters trigger (counter granted to the Champion, still withheld from a non-Bear); both effects gone after cleanup; the once-each-turn refusal (with an unchanged state and a second activation still refused); no offer without {5}{G}; `legal_targets(BearYouControl)` flipping only after the activation.
- Focused suites re-run green: abilities 13, card_printings 8, catalog_pool 5, command_zone_setup 44, issue13_commander_combat 43, issue17_slice15 6, issue17_slice15_corpus 2, issue17_slice17 6, issue17_slice20 9, issue17_slice22 9, issue17_slice23 11, issue17_slice24 10, issue17_slice25 7, rules_conformance 5, timing_mana 12 — **196 passed, 0 failed**. The independent reviewer additionally ran `cargo test -p aperture-core --locked` → **539 passed / 0 failed**.
- `cargo fmt --all --check` PASS; `cargo clippy --workspace --all-targets --locked -- -D warnings` PASS; `cargo build --workspace --release --locked` PASS.
- Two discriminating mutation REDs, re-derived from the current bytes in byte mode by `slice26/regenerate-mutation-evidence.py` (it fails unless exactly the claimed tests fail and the engine is restored byte-for-byte): layer 4 ignored → exactly the two subtype tests fail; printed once-per-turn dropped → exactly the record/restriction tests fail. Recorded hashes in `slice26/pre-mutation-sha256.txt` match the reviewed blobs.
- Real four-seat/40-life game on the exact `decks/package-slice26.txt` snapshot (seed 2626, Jasmine Boreal in all four command zones): terminal `Win{P3}`, **1366 actions / 34 turns / 3613 replay events**, digest `86bbd60bcec7b2162507628ee0e7377acb3ef2b1237ffaf85b66e59d6995de19`, 9 Ursine Champion casts, 14 layer-4 type-change events and 14 matching pumps. A **separate** release-CLI replay reproduces the same digest and 3613 events; the fresh save round-trips. Slice-25's replay still verifies (`4d6ea58e`) — the digest basis did not move. **This is a 40-card evidence package driven by a masked demonstrator, not product AI, not a strength claim and not a desired 100-card starter certificate.**

## Independent review of this increment

A fresh independent reviewer (not the saved original OMP histories, no implementation edits) audited delta `ea10693..9cd0b68` and returned **CHANGES_REQUESTED with no functional defect found**. It re-ran 105 focused tests plus the whole `aperture-core` suite, rebuilt and re-ran the journey from its own copy (byte-identical save/replay/report), replayed the CLI, verified the live Scryfall printing, audited the corpus independently (143 / 459 = 131 supported + 328 missing, per list 55/0/51/48/22/52/52/53) and re-derived both mutations. Its four blocking items were all record/claim problems, and all four are fixed in `80fff73`:

1. "first once-each-turn restriction" was false — Rootwalla already prints it (catalog and test comments corrected);
2. `docs/issue17-starter-corpus.md` still said Ayula had 23 missing names (now 22);
3. the same doc claimed Little Bear reads the *printed* subtype (it now reads the effective one);
4. the mutation record did not describe the reviewed blob (hashes and panic line numbers predated `cargo fmt`); it is now regenerated from the current bytes with a byte-mode restore proof.

Non-blocking findings retained as disclosed debt: `has_subtype` claims timestamp ordering it does not implement (unobservable while `SubtypeLine` has one variant); `SubtypeLine::has` splits on whitespace, which would mishandle a two-word creature subtype (CR 205.3b); `once_per_turn` is keyed on the object, not the player (no control-changing effect exists in the pool); the old `Rootwalla`-era CR 602.5b citations elsewhere in the tree were not rewritten.

## Remaining acceptance and limits

Independent each-list audit after this slice: **143 catalog / 459 unique desired names = 131 supported + 328 missing**, per list `55/0/51/48/22/52/52/53`. **1 of 8** desired 100-card lists is certified (Jasmine); Ayula is 22 names short. Desired deck snapshots and certification statuses unchanged. Full original #17 acceptance remains false; arbitrary Archidekt compatibility remains a non-goal. No UI/import/AI-policy change.

Slice-26 limitations: only one `SubtypeLine` variant exists, so layer-4 stacking is untested beyond "the answer is still Bear"; no control-change, colour-change (layer 5) or animation (non-creature → creature) was added, so `Circle of the Moon Druid`, `Stuffed Bear`, `Beorn the Fierce`, `Beorn's Hospitality` and the rest of the Ayula gap remain unsupported; the journey is a 40-card package, not a certificate.

## Histories and next action

The saved writer/reviewer histories in `evidence/issue17/sessions/` and `evidence/issue17/review-sessions/` were not rewritten by this slice; this run's reviewer was a separate delegated session, which is disclosed above and in the handoff metadata.

Continue sourced missing mechanics on the same card / PR46 / branch, preserving approved increments 1–25 without re-audit, until one whole desired 100-card starter can be certified end to end. Do not merge, push master, close the issue or unlock certified downstream work from this lane.
