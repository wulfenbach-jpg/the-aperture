# Slice 61 — review-360's four corrections, verified by execution

- Branch `aperture/issue-17`, draft **PR46**. Base `ef03f11`; **code head `4b18630`**;
  evidence tip is the commit that adds this file (it claims no CI of its own).
- `origin/master` is untouched by this lane.
- **Original #17 remains unfinished: 2 of 8 lists certified.** This slice enables no
  new card and claims no new list. It closes the four corrections review 360 required
  in the *opening* commit of the next product slice, two of which are code/coverage
  defects rather than records.

## 1. What the review required, and what this commit does

| Item | Where it was wrong at `ef03f11` | What `4b18630` does |
|---|---|---|
| **R360-1** | `certify_cmd.rs` `interaction_exclusions()` and the Ayula manifest block said "a general legend-rule SBA with its controller choice is not modelled for this certificate" — false since slice 59 | both say the CR 704.5j SBA **and** its controller's choice are implemented and covered by conformance suites; these games never *reach* a choice (one commander per seat, no copy effects) |
| **R360-2** | Jasmine's certificate exclusions said the legend rule is "unmodelled"; docs §Exclusion policy said legends are commander-only and "no other legend may enter a starter library"; §Phased gaps item 7 and the Jasmine section repeated it | one consistent present-tense statement per site: only `SUPPORTED_COMMANDERS` entries are commander-only (`DeckList::parse_practice` refuses them); other legendary cards are ordinary pool cards (the certified Ayula list contains five); the legend rule is implemented. The slice-41/43 sections keep their own-time text and are marked *superseded*; the slice-59 section records the supersession. `isamaru` and `krenko` `missing_interactions` no longer list the legend rule |
| **R360-3** | `entering_resolves_two_hideaways_that_exile_face_down_and_bottom_the_rest` asserted only counts and links; the reviewer's mutation (bottomed cards left on **top**) kept all 9 slice-60 tests green | the test now asserts the observable order: the untouched tail is on top in its original order, and the looked-at non-exiled cards are at the bottom (multiset). Fixture tail changed to two Plains so the two regions are distinguishable |
| **R360-4** | `selfplay --commander` help omitted **Ayula, Queen Among Bears** and still said "no legend rule, no full 100-card construction" | the help names every `CardId::SUPPORTED_COMMANDERS` entry in registry order, drops the retired claim, and a CLI test binds the help text to the registry so drift fails CI |

## 2. Evidence that the two new assertions discriminate (RED first)

- **R360-3, by the reviewer's own mutation.** `apply_hideaway_exile` set to
  `library.splice(0..0, rest.iter().copied())` (bottomed cards left on top):
  `issue17_slice60` → **8 passed / 1 failed, exit 101**, failing on
  `left: [RuneclawBear, Island] / right: [Plains, Plains]`
  (`slice61/mutation-bottoming-red.log`). `crates/core/src/engine.rs` was restored and
  is byte-identical to the base (`git checkout`, verified clean).
- **R360-4, by removing the missing commander from the help.** Ayula deleted from the
  `--commander` doc list → `selfplay_commander_help_lists_every_supported_commander`
  **fails, exit 101** (`slice61/red-help-missing-ayula.log`); the file was restored
  from a byte backup and re-diffed.

## 3. Gates executed at `4b18630`

- `cargo fmt --all --check` → 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → 0.
- Focused, 10 binaries / **79 passed / 0 failed**: `certify` 4, `commander_check` 9
  (one new), `starter_corpus` 8, `catalog_pool` 5, `card_printings` 20,
  `issue17_slice60` 9 (strengthened), `issue17_slice59_repairs` 7,
  `issue17_slice59_ring_repairs` 10, `issue17_slice55` 5, `issue17_slice15_corpus` 2.
  No fresh whole-workspace total is claimed here; exact-head CI is the broad gate.
- Corpus audit re-run through `evidence/issue17/slice60/corpus-audit.py`: **AUDIT OK**,
  166 catalog / 459 union = **153 supported + 306 missing**, per-list
  `[55, 0, 51, 48, 0, 52, 52, 53]`, **2 certified**, Ayula's two gap fields empty, no
  supported-card interaction citation. The manifest delta is text-only; no deck file,
  printing fixture or catalog row changed in this slice.

## 4. Four-seat journey, on the exact snapshot, re-executed

The shipped `certify` path was re-run at the code head on
`decks/starter-corpus/ayula-bear-counters.txt` (exact snapshot, sha256
`7a1b3f9c745967bd…19c4`), four seats, 40 life, seeds 1–3
(`slice61/ayula-certificate.json`):

| seed | outcome | steps | turns | winner | digest |
|---|---|---|---|---|---|
| 1 | terminal | 1499 | 31 | P1 | `78b301cbef68…` |
| 2 | terminal | 1373 | 30 | P3 | `8ccf4bc4a753…` |
| 3 | terminal | 1233 | 26 | P2 | `0adc5f3097ea…` |

Every game is terminal, every embedded replay verifies, and all three digests are
**identical to the accepted slice-60 certificate**: the exclusion wording changed the
certificate text, not a single decision. Separately, `selfplay --seed 1` with the same
snapshot in all four seats and Ayula in the command zone produced the same terminal
`Win{P1}` / 1499 steps / `78b301cbef68…`, and `replay --file` on that separate replay
printed `matches recorded`, 3793 events, exit 0 (`slice61/selfplay-1.log`,
`slice61/replay-1.json`, `slice61/replay-check-1.log`).

The run used the debug CLI binary (`cargo run -p aperture-cli --bin aperture-cli`), not
`--release`, which changes nothing about determinism; the manifest's `reproduce` line
still names the release build.

## 5. CI

`gh api commits/4b18630/check-runs` → **10 runs, every `head_sha` = `4b18630`,
all 10 completed `success`, 0 cancelled, 0 failed**, from push run
`40841872045` (attempt 1) and `pull_request` merge-ref run `40841884279`
(attempt 1) — receipt `slice61/exact-head-ci-4b18630.json`. Both workflows
finished before the evidence commit was pushed; nothing was pushed while the gate
ran, and this tip claims no CI of its own.

## 6. Honest state and next slice

- **Original #17: 2 of 8 certified** (`jasmine-vanilla-stompy`, `ayula-bear-counters`);
  six lists planned with real gaps `[55, 51, 48, 52, 52, 53]`. No completion, merge,
  master push, closure, dependent unlock, new team or global-config change.
- **This slice shipped no card and no engine behaviour change.** It is not a
  record-only slice in the sense review 356/360 feared: R360-3 and R360-4 were real
  coverage/user-facing defects, and both now have discriminating regressions. It is
  still true that the *product* moved sideways, and the reason is time-window, not a
  blocker: the review required these corrections in the **opening commit** of the next
  product slice, and this run's window closed after the corrections were published and
  gated.
- **Next slice, in order:** `krenko-goblin-tokens` (smallest real gap, 48 names) with
  **Goblin Chieftain** as the first ordered package — printed Haste (the engine reaches
  it today only through granted haste) plus "Other Goblin creatures you control get
  +1/+1 and have haste" as one subtype-filtered static that composes the existing
  anthem and haste-grant layers — then `Searslicer Goblin` (Raid: end-step trigger with
  "if you attacked this turn" and a real 1/1 red Goblin token). Each with a whole-field
  `missing_cards` **and** `missing_interactions` rewrite, focused changed-behaviour
  tests, one real four-seat journey with separate replay, and exact-head CI on the code
  head.

## 7. Disclosures

- Native author of this slice; not its independent reviewer.
- The mutation RED for R360-3 is the reviewer's mutation, executed by me at the
  strengthened test; the pre-fix RED for R360-4 is my own help-text deletion.
- `crates/core/src/engine.rs` was mutated only for the RED and restored; the tracked
  tree was verified clean afterwards.
- The certificate, the manifest text and the CLI help were corrected; no behaviour,
  digest, deck, printing fixture or catalog row moved.
