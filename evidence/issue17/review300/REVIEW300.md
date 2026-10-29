# Self-check 300 — slice 38 (Spider-Ham, Peter Porker, SPM 114)

> **NOT an independent review.** This file was written inside the run that wrote
> the slice-38 commits and checkpoint (run 302, session
> `local-d727e0de…`), so it is an implementer's verification note, not a
> reviewer's verdict. It claims **no** approval, and slice 38 must not be
> described as independently reviewed until a separate review run adjudicates
> the exact head from `git archive` and posts its own verdict. The record-integrity
> note that established this is on kanban `t_67778feb` (supervisor, 2026-10-28).

Author: the slice-38 implementing worker. Lens: execution — the record was read
first, then every material claim was re-run from a `git archive` of the reviewed
head, out of tree. No deliverable file was edited by me after the commits;
everything produced here is untracked evidence.

Reviewed head: **`1cd966d`** (code head `6bb0740`).

## Verdict

**No verdict — self-check only.** What follows is what the implementing run
executed. Review 299's two record-integrity items are closed, and the new card is
sourced, tested and exercised in a real four-seat game. **The ORIGINAL #17
contract is still UNFINISHED** (criterion 1: 1 of 8 certified lists; criterion 2:
319 of 459 union names unsupported, Ayula 13 short), so this returns to sourced
implementation rather than completing.

## Review 299's required items

1. **Driver record-integrity — closed.** `evidence/issue17/slice37-journey-driver/src/main.rs`
   no longer claims a pairing ends in the game; both the module doc and the
   report's `purpose` string state what the run demonstrates and name the focused
   test as the ending evidence. I re-ran the slice-37 driver from an archive of
   the reviewed head: exit 0, digest **`c4c5985cd048…`** unchanged, 1758 steps /
   42 turns / 4530 events, `pairs_ended_in_game = 0` now consistent with the prose,
   and `driver-replay.json` / `driver-save.json` identical to their previous bytes
   (`git diff 07ad00e..1cd966d -- evidence/issue17/slice37` touches only the
   journey JSON, the run log and the receipt).
2. **Tip receipt — closed.** `evidence/issue17/slice37/exact-head-ci-07ad00e.json`
   is committed (10 runs, all `completed/success`, every `head_sha` = `07ad00e`),
   and `git status` shows no stray untracked receipt in `slice35/` or `slice37/`.
3. **Ayula to zero — one more name.** Spider-Ham, Peter Porker is implemented and
   the audit moves 139 → **140** supported, Ayula 14 → **13**.

## What I verified by running it

* **Printed text, live.** Scryfall `spm/114` re-fetched this review: `{1}{G}`,
  `Legendary Creature — Spider Boar Hero`, 2/2, oracle text byte-identical to the
  catalog row, the fixture and the `card_printings` record. The printings query
  returns five printings (`spm 114`, `spm 201`, `pspm 114s`, `pw25 10`, `om1 118`)
  with **one** distinct text, type line, cost and P/T.
* **Focused suites at the reviewed head.** `issue17_slice38` **6/6**,
  `card_printings` **15/15**, `catalog_pool` **5/5**, `issue17_slice15_corpus`
  **1/1**, `issue13_commander_combat` **43/43**; whole workspace **122 suites /
  867 passed / 0 failed**; `cargo fmt --all --check` exit 0; `cargo clippy
  --workspace --all-targets --locked -- -D warnings` exit 0.
* **Evidence reproducible from the record alone.** `git archive 1cd966d` →
  `/c/AI/r300/<stamp>`, `cargo run --release --locked` in
  `evidence/issue17/slice38-journey-driver`: exit 0, terminal **`Win{P2}`**, seed
  3838, **1450 steps / 34 turns / 3845 replay events**, digest
  **`93329b6394838334e60bd2b9ecbfa646c3044c4a91ba347d1c59cb387a5bbc34`**, 8
  Spider-Ham casts, 8 Food tokens created, 8 activations, 7110 anthem assertions.
  The regenerated `driver-journey.json` / `driver-replay.json` / `driver-save.json`
  are **byte-identical to the committed blobs**
  (`2a096e68…`, `1e590f10…`, `436c3bf0…`, `sha256sum` both ways).
* **The anthem is load-bearing.** In my export only, I made the new layer 7c
  list-filtered arm match no recipient (`&& false` added to the filter, which
  still compiles): exactly the two behavioural anthem tests go RED (4 passed, 2
  failed). Restoring the committed bytes (`sha256 f2ed375a…` both before and
  after) returns 6/6. Logs: `evidence/issue17/slice38/mutation-anthem-matches-nothing.log`,
  `mutation-restored.log`.
* **Corpus audit.** My own run of the audit script over `catalog.rs` + the eight
  desired deck files + `manifest.json` (manifest set-equality asserted): catalog
  **152**, union **459** = **140 supported + 319 missing**, per list
  `[55, 0, 51, 48, 13, 52, 52, 53]`, Ayula **13**, **1 of 8 certified**,
  `AUDIT OK`.

## Rules reading I checked against the printed text

* "Sacrifice this token" as a cost (CR 602.2b): the engine pays `{2}`, taps the
  token, moves it to the graveyard **before** the ability is on the stack, and
  resolves "You gain 3 life" afterwards — asserted by the focused test (token's
  zone is the graveyard at activation time) and by the journey's life accounting
  (3 × activations = every positive life change in a package with no other life
  gain).
* "Other ... you control get +1/+1": the source is excluded from its own set,
  a second copy of the anthem does pump it (it is a Spider), the recipient must
  be a creature, and the read is the *effective* subtype (layer 4 Bear Form
  counts). All four are covered.
* The token is an artifact and not a creature (`is_creature` false, no P/T), and
  its ability is only offered while it is untapped and `{2}` is payable.

## Non-blocking observations (disclosed, not a hold)

* The Food token's ability is modelled as the printed cost/effect only; it is not
  a generic sacrifice outlet and nothing in the pool can copy, blink or recur it.
  That is the honest scope of a token with one printed ability.
* `OtherControlledCreaturesWithAnySubtypeGet` is read with `find_map`, so a card
  printing *two* such anthems would only have the first applied. No pool card
  does; worth a guard or a loop when a second one lands.
* The journey's anthem assertion skips creatures whose base is replaced by a
  layer 7b static (Circle of the Moon Druid), because "printed + 1/+1" is not the
  right baseline for them; the focused test pins that card's exact 5/3 and 8/6
  instead. Disclosed in the checkpoint.
* Three tracked slice-23 evidence files still carry pre-existing uncommitted
  appends from earlier runs; untouched here.

## Next required work (same card / PR46 / branch / histories)

Continue the supervisor's Ayula-to-zero package. **13 names remain**: Dúnedain
Rangers, Evercoat Ursine, Lightning Greaves, Lumra, Bellow of the Woods, Owlbear
Cub, Rampaging Yao Guai, Ruxa, Patient Professor, Spirit of the Aldergard,
Swiftfoot Boots, The Earth King, Thijarian Witness, Titania's Command, Ursine
Monstrosity — each sourced from authoritative printed text with printing id and
retrieval stamp, ordered by shared primitive (the two Equipment cards share the
equipment/attached-static primitive; the tutor names share library search).
When the last lands, run the certification itself (exact 100-card snapshot, four
seats, real games plus separate replays, per-list audit showing 0 missing) so
1 of 8 becomes 2 of 8.

## Exact-head CI

`gh api repos/wulfenbach-jpg/the-aperture/commits/1cd966d/check-runs` → **10/10
completed success, every `head_sha` = `1cd966d`** (`board and roadmap` ×2,
`rust (ubuntu|windows|macos-latest)` ×2, `windows package` ×2), capture exit 0.
Receipt: `evidence/issue17/slice38/exact-head-ci-1cd966d.json`, capture log and
wait trace beside it. The capture resolves the abbreviated sha to the full commit
id first and bounds every `gh` call at 120 s (exit 124 on timeout).

This self-check did not merge, push master, close the issue, unlock `t_26a7a4ff` /
`t_3c2fe51a`, or edit any deliverable file after the commits; the mutation was made
in the out-of-tree export only. **It is not acceptance: a separate review run must
adjudicate `1cd966d` on its own.**
