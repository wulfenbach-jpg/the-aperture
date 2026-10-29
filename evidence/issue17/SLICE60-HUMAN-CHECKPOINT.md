# Slice 60 human checkpoint — Evercoat Ursine implemented, the exact Ayula list certified (2 of 8)

- Branch `aperture/issue-17`, draft PR46, same writer/session/card. No merge,
  master push, closure, dependent unlock, new team or provider/model-config
  change. `origin/master` untouched at `73820d7`.
- Commits: `99af329` (R359-1 record fix, opening), `75ecf85` (Evercoat Ursine +
  the Ursine Monstrosity trample defect), `4e66f25` (exact-snapshot
  certification + truthful certificate exclusions), `15c0e60` (import-side
  planned-list audit accepts the second certified list).

## 1. What was asked and what was delivered

Review 359 returned the unfinished original #17 with one record item (R359-1)
and the directive: put the record fix in the opening commit, then implement
**Evercoat Ursine** — sourced printed text, linked hideaway exile and free play —
with whole-field `missing_cards`/`missing_interactions` discipline, then, the
moment both Ayula gap fields are zero, run the **exact 100-card four-seat
certification** with separate replays and claim 2 of 8 only if it passes.

All of that is done:

* **R359-1 closed** in `99af329`: the slice-59 checkpoint had the CI run ids
  swapped; push is `40825526651`, the `pull_request` merge-ref run
  `40825533912`.
* **Evercoat Ursine enabled** in `75ecf85` with all three printed clauses.
* **Ayula gaps are zero** in both fields and the manifest now says so.
* **The exact Ayula 100-card list is certified** with three real four-seat
  40-life games, separate replays, and a manifest certificate. **2 of 8**.

## 2. The card: what is implemented and how it is proved

Printing: **BLC 30** (`b299d768-b171-47e5-b69d-82ce13917c43`, oracle
`65c32889-3d70-449f-af76-05385c78c5a9`), `{4}{G}`, Creature — Elemental Bear,
6/5, Trample. Scryfall was fetched live during this run (2026-10-29T06:58:12Z);
the fixture records the full record, the 2-printing check (one distinct rules
text) and the card's six official rulings.

1. **Trample** is a real keyword flag, not catalog text.
2. **"Hideaway 3, hideaway 3"** is two instances of the keyword, so the card
   carries two separate enters-the-battlefield triggered abilities and entering
   resolves two independent looks. Each look is private (CR 701.20a), exiles
   exactly one looked-at card **face down** linked to that exact permanent
   (CR 702.75a; `GameState::hideaway` + `face_down_exile`), and puts the rest of
   the *looked-at* cards on the bottom in a random order (CR 701.20e) without
   touching the rest of the library.
3. **"Whenever this creature deals combat damage to a player, if there are cards
   exiled with it, you may play one of them without paying its mana cost"** is a
   real triggered ability on a new `TriggerEvent::ThisDealsCombatDamageToPlayer`.
   The intervening-if is evaluated when it triggers and again at resolution
   (CR 603.4); the "you may" is the ordinary CR 603.5 yes/no; the play itself is
   a resumable `Pending::PlayExiled` offer with **one named action per playable
   linked card** plus an explicit decline (not one hard-coded sample — the same
   defect class review 351 caught in the Ring discard).

**The printed ruling is enforced, not paraphrased.** The card is played while the
ability resolves, so no normal timing restriction applies; a land may be played
only while the seat still has its land play for the turn (CR 305.2); a printed
`{X}` is 0; no alternative cost is chosen (no pool card prints one). A free cast
goes through the ordinary stack and cast-trigger path with no mana paid, and the
card stops being exiled with anything the moment it changes zone (CR 400.7).

**Hidden information.** A face-down exiled card is identified only to the player
who controlled the exiling permanent (CR 406.3). The public projection of every
other seat renders it as "a face-down exiled card" with no oracle text and no
cost, and the public event log never names it — not even the card the owner
chose.

**Forged-state refusal.** Both new pending kinds are validated by the same
structural validator a save load uses: the look must be exactly the current top
of that seat's library and bound to the resolving hideaway trigger; every play
option must be a face-down exiled card whose link names that source and that
controller; a land option is refused when the turn's land play is spent. The
slice-60 test file proves a real `SessionSave::from_json` refuses an injected
option and that `GameState::validate` refuses it too.

**Deliberate limits (documented in `docs/issue17-starter-corpus.md`).** The look
permission is recorded for the controller who exiled the cards; control changes
of that permanent are not tracked. Additional costs on a free-cast card are not
modelled because no pool card prints one. A card whose exiling permanent has left
the battlefield stays exiled face down and is unreachable (the correct CR 400.7
outcome, covered by a regression).

## 3. A real defect in an accepted increment, fixed here

**Ursine Monstrosity printed "Trample" but was registered through the plain
catalog macro**, so its own printed keyword was never implemented — it was a
3/3 without trample in every game since slice 54. It is now a `card_trample!`
row, and `ursine_monstrosity_now_really_has_its_printed_trample` in
`issue17_slice60.rs` keeps it fixed. This is a behaviour change in an approved
increment; it is stated here rather than folded silently into the new card.

## 4. The certification (2 of 8)

`aperture-cli certify --starter ayula-bear-counters --seats 4 --seeds 1,2,3`
reads the declared snapshot `decks/starter-corpus/ayula-bear-counters.txt`
(99 library cards, sha256 `7a1b3f9c745967bdaae018c20d9ba479dd1b4f14e8a24081c3efe10fd14c19c4`
over the LF-normalised file), verifies every card is supported, puts that exact
list in **every** seat with Ayula as the commander, and plays three four-seat
40-life Commander-combat games:

| seed | outcome | actions | turns | events | winner | digest |
| ---- | ------- | ------- | ----- | ------ | ------ | ------ |
| 1 | terminal | 1499 | 31 | 3793 | P1 | `78b301cbef68f364e0047157b991c2f7016007b38613a12c4caf109750008703` |
| 2 | terminal | 1373 | 30 | 3508 | P3 | `8ccf4bc4a753fe7f7bba174b48917002da1af16faa18f18e6b0dc2e367ca72aa` |
| 3 | terminal | 1233 | 26 | 3127 | P2 | `0adc5f3097ea3f9c5cf39505c88a9fa5e69e8e3555267b96e7ffdefd410f315d` |

Every embedded replay reproduces its digest. The same three games were then
replayed **separately** through `selfplay --replay-out` and `replay --file`: both
paths reproduce the identical digests, `replay` prints `matches recorded` and
exits 0 (`evidence/issue17/slice60/replays/`). 31 of the 59 distinct names were
cast or played in those games; the other 28 are covered by engine conformance
suites and the certificate says so instead of inferring behaviour from the
catalog.

**Correction to the certificate generator.** `certify` carried a fixed exclusion
sentence saying "equipment, Auras, planeswalkers and nonbasic lands: the list
contains none". That is true for Jasmine and false for Ayula, which really does
carry Sol Ring, Arcane Signet, Fellwar Stone, Swiftfoot Boots, Lightning Greaves,
Stuffed Bear, Friendly Teddy and five enchantments. The exclusions are now
computed from the list being hashed, and the Ayula certificate names every such
card and states that only the ones actually cast are proved by the trace. The
manifest's certificate block repeats that wording; the test asserts it.

## 5. Corpus state

Fresh whole-field audit (`evidence/issue17/slice60/corpus-audit.py`, `AUDIT OK`):

* **166 catalog / 459 union = 153 supported + 306 missing**;
* per-list `[55, 0, 51, 48, 0, 52, 52, 53]`;
* **2 of 8 certified** (`jasmine-vanilla-stompy`, `ayula-bear-counters`);
* Ayula's `missing_cards` **and** `missing_interactions` are both empty and its
  `deck_path` points at the supported exact snapshot, not a planned list;
* no manifest interaction entry cites a supported card.

## 6. Gates

Local, at `15c0e60` (logs under `evidence/issue17/slice60/`):

* `cargo fmt --all --check` — clean;
* `cargo clippy --workspace --all-targets --locked -- -D warnings` — exit 0;
* focused suites: `issue17_slice60` 9, `card_printings` 20, `catalog_pool` 5,
  `starter_corpus` 8, `issue17_slice15_corpus` 2, `issue13_commander_combat` 43,
  `starter_corpus_planned` 1, `certify` (CLI) — see the exact-head CI for the CLI
  suite, which cannot share `target/` while the workspace run holds it;
* the exact-head CI on the code+evidence head is recorded in the `ci` object of
  the handoff metadata and in `evidence/issue17/slice60/exact-head-ci-<sha>.json`.

## 7. What is NOT done

* Original #17 remains **unfinished**: 6 of 8 lists are still planned with real
  card gaps (55, 51, 48, 52, 52, 53 names).
* The certificate is the bounded Commander-combat practice profile (40 life, one
  supported commander per seat), not the full Commander format, and it proves
  the list reaches a terminal replay-verified game, not playing strength.
* No merge, master push, issue closure or dependent unlock.
