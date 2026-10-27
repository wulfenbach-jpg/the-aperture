# Slice 34 — review-290 repair: the landfall target is "target creature you control", not "another"

Date: 2026-10-27. Branch `aperture/issue-17`, draft PR46, one writer.

Continues from the review-290 head `cff319d` (code head `01fdc3b`). Review 290 returned **CHANGES_REQUESTED** with one behavioural defect in the slice-33 card and two false record statements. Approved increments 1–32 and the slice-33 engine work are preserved without re-audit; this slice repairs the three findings and re-verifies the delivery bar.

## What was wrong, and what changed

**Blocking (R290-1).** `TargetSpec::CreatureYouControl` filtered `o.id != exclude`, because that spec had been built to model Little Bear's printed "untap **another** target creature you control". Beorn's Hospitality prints no "another", and its own animation makes it a creature, so its landfall trigger could never target the card itself — with the card as its controller's only creature the mandatory trigger was removed as "no legal target (CR 603.3d)" and no counter was placed.

The spec is split rather than patched per-ability:

* `TargetSpec::CreatureYouControl` — the printed "target creature you control": every creature the ability's controller controls, **the ability's own source included** when it is a creature at the moment targets are chosen (CR 115.1).
* `TargetSpec::AnotherCreatureYouControl` (new) — the printed "another target creature you control": the same set minus the ability's own source. Only a printing that says "another" may use it.
* `LITTLE_BEAR` now uses the "another" spec, so its behaviour is unchanged (`ability.rs`). Dancing from Dark to Dawn's cast trigger prints "target creature you control" as well, so it now reads the correct set too (unreachable difference today: the source is an enchantment at that moment).
* `target_spec_rule` gained the matching label; `coach.rs` keeps its catch-all, so no privacy or wording surface moved.

**Record (R290-2).** `SLICE33-HUMAN-CHECKPOINT.md` said `card_printings` was 13/13; the target has 12 tests and reports 12 passed at this head. Corrected.

**Record (R290-3).** The same checkpoint said the receipt-line commit `cff319d` "has no runs of its own". It has 10 check runs of its own, all completed SUCCESS keyed to `cff319d`. Corrected.

Also corrected in the same files: the "with none it does not put the trigger on the stack" sentence in the checkpoint and in `docs/issue17-starter-corpus.md` now states the rule actually implemented — the printed set includes the animated source, and the trigger is removed only when that seat controls no creature at all (CR 603.3d).

## Regression coverage added (five tests in `crates/core/tests/issue17_slice33.rs`)

1. `an_animated_hospitality_is_its_own_landfall_target` — animated with no other creature, a land is played: the source is the only legal target, the engine auto-chooses it (no pending choice), the counter lands on the card, and it is **5/5** (four lands from layer 7a plus the counter).
2. `landfall_offers_the_animated_source_alongside_another_creature` — with a Grizzly Bears out, the pending offer holds **exactly** the two creatures and includes the source; choosing the source counters it (5/5) and leaves the Bears at 2/2.
3. `an_unanimated_hospitality_is_not_its_own_landfall_target` — the printed Enchantment is not a creature target; with one other creature the counter is auto-aimed at it and the enchantment keeps zero counters.
4. `landfall_without_a_legal_target_is_removed_not_staged` — a lone unanimated watcher: `TriggerRemoved` with the CR 603.3d reason, nothing staged, empty stack, no pending choice.
5. `little_bear_keeps_the_printed_another_exclusion` — the tag is `AnotherCreatureYouControl` **and** behaviourally, as the only creature, Little Bear's own trigger is removed with no counter on itself.

Two existing slice-33 tests were updated because they played a land with the animated source as the only creature: their expected P/T is now the land count **plus the landfall counter this trigger legitimately places on the source**, with the characteristic-defining value still read alone after stripping the counter. That is a consequence of the card's correct text, not a weakened assertion.

## Verification actually run at this head

* `cargo test -p aperture-core --test issue17_slice33 --locked` — **12 passed / 0 failed** (7 slice-33 tests + 5 repair tests).
* `cargo test -p aperture-core --test card_printings --locked` — **12 passed / 0 failed**.
* `cargo test --workspace --locked` — **119 suites / 847 declared / 846 passed / 0 failed / 1 ignored** (the pre-existing network Archidekt test), exit 0, log `evidence/issue17/slice34/gates-final.log`.
* `cargo fmt --all --check` — exit 0.
* `cargo clippy --workspace --all-targets --locked -- -D warnings` — exit 0.
* **Two discriminating mutations**, each re-run from the **committed** bytes after formatting and restored byte-for-byte (`sha256sum -c` clean on both `pre-` and `post-mutation-*-sha256.txt`, logs in `evidence/issue17/slice34/`):
  * `red-mutation-source-excluded.log` — reinstating `&& o.id != exclude` in `CreatureYouControl` fails 4 tests, including both self-target tests and the two P/T tests. `pre-`/`post-mutation-sha256.txt` both name `08988a74…d6338`, the committed `engine.rs`, and `sha256sum -c` exits 0.
  * `red-mutation-another-spec-dropped.log` — pointing Little Bear back at the plain spec fails exactly `little_bear_keeps_the_printed_another_exclusion`. `pre-`/`post-mutation-ability-sha256.txt` both name `dff6b60d…a839`, the committed `ability.rs`, and `sha256sum -c` exits 0.
* **Four-seat / 40-life exact-snapshot journey** rebuilt from `evidence/issue17/slice33-journey-driver` at this head (`decks/package-slice33.txt`, seed 3333): terminal **Win{P3}**, **1117 actions**, 24 turns, **2967 replay events**, digest `3dfb14d4ce762cdcf52bd9c8d5f1832cebe3dbd4584b21c4cd1dff57dfd14c22`, snapshot `a7a45e4b…7c581d`, 6 casts, 2 animation activations, 4 continuous effects, 13 landfall triggers, 4 counters in play, **29 on-turn / 28 off-turn** invariant sightings. The generated `demo-journey.json`, `demo-save.json` and `demo-replay.json` were rewritten by the run and are **byte-identical** to the committed evidence (no diff), and a separate release `aperture-cli replay` of the recorded replay matches (`replayed digest 3dfb14d4ce76 | matches recorded | events 2967`, `CLI_EXIT=0`).
  The repair does not change this journey's action sequence: no resolution in this package had the source as the sole legal landfall target, and where several creatures were legal the driver's deterministic choice is unchanged. That is recorded rather than presented as evidence that the defect was unreachable — the five focused tests are what discriminate it.
* Corpus audit recomputed (`evidence/issue17/slice33/corpus-audit.py`, unchanged tool): **149 catalog / 459 union = 137 supported + 322 missing**, per list `55/0/51/48/16/52/52/53`, Ayula **16**, **1 of 8 certified**; log `evidence/issue17/slice34/corpus-audit.log`.

## Independent review 294 and the exact-head CI

* **Review 294** (same original reviewer history, `evidence/issue17/review-sessions/2026-10-25T08-39-41-067Z_01a17237-2d0b-7000-9ed9-5b89f398f17d.jsonl`, whose pre-resume state is recoverable from commit `5b60c2c`) returned **CHANGES_REQUESTED** at `b74815d` with **no product change indicated**: it independently reproduced the spec split, ran 12/12 slice-33 and 12/12 `card_printings`, checked the delta touches no other rules path, matched the CLI replay (1117 actions / 2967 events / `3dfb14d4ce76`) and the audit, and confirmed `crates/ai`, `crates/ui` and the deck snapshots are untouched. Its one blocking finding was record integrity: `pre-mutation-sha256.txt` had been written **before** `cargo fmt --all` reformatted `engine.rs`, so `sha256sum -c` failed against the committed file while the checkpoint still claimed a clean restore. Both mutation receipts were therefore regenerated from the committed bytes and re-verified: the source-exclusion mutation still fails 4 tests and the Little Bear mutation exactly 1, with `pre-` and `post-mutation-*-sha256.txt` now matching `engine.rs` `08988a74…` and `ability.rs` `dff6b60d…` and `sha256sum -c` exiting 0. Report: `evidence/issue17/review294/REVIEW294.md`.
* **Review 295** (same history, re-adjudication of the review-294 correction) returned **APPROVED_INCREMENT at `bfbbceb6e023644d080d980e91d922babaaca60c`**: the delta since `b74815d` is evidence/docs only, both `sha256sum -c` receipts exit 0 against the committed `engine.rs` (`08988a74…`) and `ability.rs` (`dff6b60d…`) with pre/post pairs agreeing, the regenerated RED logs still show 4 and 1 discriminating failures, `issue17_slice33` is still 12/12, and no product behaviour moved. Report: `evidence/issue17/review295/REVIEW295.md`.
* **Exact-head CI**: `b74815d6690d76a7d838bc491fd37b1d791328d0` — **10/10 check runs completed SUCCESS, every `head_sha` = `b74815d`** (receipt `evidence/issue17/slice34/exact-head-ci-b74815d.json`, log `slice34/ci-capture.log`), and `bfbbceb6e023644d080d980e91d922babaaca60c` — **10/10 completed SUCCESS, every `head_sha` = `bfbbceb`** (receipt `slice34/exact-head-ci-bfbbceb.json`, log `slice34/ci-capture-tip.log`, capture exit 0). Both reviews were written while runs were still in flight and say so; these are the completed captures.

## Disclosures

* The journey is a **40-card package**, not a 100-card starter certificate, and its driver is an evidence-only masked demonstrator, not product AI or strength evidence.
* `crates/ai` and `crates/ui` are untouched by this repair (`git diff --stat` empty for those paths); the desired deck snapshots are untouched.
* Review 290's non-blocking notes are recorded, not fixed here: the printed-7b-before-7a ordering inversion is unreachable in today's pool, and `card_printings.rs` still says "colour identity" where it asserts `colors`.

## State after this slice

Original issue #17 remains **unfinished**: 1 of 8 desired 100-card lists certified, Ayula 16 names short, 322 of 459 union names unsupported. No merge, no master push, no issue closure, no downstream unlock (`t_26a7a4ff`, `t_3c2fe51a` stay parked). No new worker, team or PR.
