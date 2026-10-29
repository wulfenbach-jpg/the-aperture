# REVIEW 289 — slice 32 (Kamahl's Summons, Surrak's printed Trample, mandatory-target regression)

Reviewed head: **`770cb640896aa9efc07caaf376f6175699a82baa`** (code head `40a87aa`; `a987dde`..`770cb64` are evidence/docs only).
Range: `e56b138..770cb64`. Branch `aperture/issue-17`, draft PR46.

**Verdict: APPROVED_INCREMENT.**

The four corrections the previous round asked for are present and independently reproduced. No functional
defect was found in the new `Pending::ApnapReveal` / `Action::RevealCreatureCards` path, in the Surrak
`card_trample!` repair, or in the mandatory-target refusal. The original issue #17 contract is **not**
finished (see the closing section), so the card returns to sourced implementation, not completion.

Mode: read-only. Every build/test below ran either in the worktree (gate runs only) or in out-of-tree
copies: `C:/AI/r289tree` (`git archive 770cb64`), `C:/AI/r289probe` (probe crate depending on
`C:/AI/aperture-worktrees/issue-17/crates/core` by path), `C:/AI/r289build` (shared CARGO_TARGET_DIR).
I edited no deliverable file. My additions are this directory only.

---

## 1. What the delta actually contains

`git diff --stat e56b138..770cb64` = 57 files, +26617/−25. Under `crates/` (19 files, +860/−12):

| Area | Change |
|---|---|
| `crates/core` | `Effect::EachPlayerMayRevealCreatureCardsForBears`, `CardId::KamahlsSummons`, `Pending::ApnapReveal`, `Action::RevealCreatureCards`, `Event::CardsRevealed`, `PendingKind::ApnapReveal`, engine queue/batch + offers + validation, catalog row (and `SurrakAndGoreclaw` `card!`→`card_trample!`), coach rules/label arms |
| `crates/ai` | `heuristic.rs` +8 (one `PendingKind::ApnapReveal` arm taking the **last** offer), `search.rs` +1 (label), `tests/common.rs` +6 (test helper takes the last reveal offer) |
| `crates/ui` | `app.rs` +18 (one `PendingKind::ApnapReveal` arm rendering the offered reveal buttons) |
| tests | new `issue17_slice32.rs` (4 tests), 2 new `card_printings.rs` tests, `catalog_pool.rs`/`rules_conformance.rs`/test-helper arms, pool-count updates 147→148 and Ayula 18→17 |
| non-code | `decks/package-slice32.txt`, `fixtures/starter-corpus/slice32-sourced.json`, manifest (one line: `Kamahl's Summons` removed from the Ayula `missing_cards`), docs, `evidence/issue17/**` |

`git diff 40a87aa 770cb64 -- crates fixtures decks` is **empty**, so the last two commits really are
docs/evidence only, as claimed.

**Scope judgement (`crates/ai`, `crates/ui`).** The arms are exactly what the handoff describes: one
heuristic arm, one search label, one UI arm, two test-helper arms. All are exhaustive-match plumbing forced
by the new public `PendingKind`; none touches import/start-up, the model, or the search tree. The heuristic
arm does encode a policy choice ("reveal everything"), but it is the dominant answer for a free reveal and
is disclosed with a comment. Disclosure is honest and the arms are behaviourally minimal — with one
omission: `crates/core/src/coach.rs` (+15) is **not** named in the checkpoint's "required-but-forced
plumbing" list, and it also rewrites the pre-existing `ApnapSacrifice` privacy sentence ("P{n} has a
simultaneous sacrifice choice pending; details are private to that seat" → "P{n} has a pending choice; …").
Mechanical and harmless (no test asserts that string), but it is an undisclosed edit to an existing surface
(see N2).

---

## 2. Gates I ran (real exit codes, this machine, this head)

| Command | Result |
|---|---|
| `cargo test -p aperture-core --test issue17_slice32 --locked` | **4 passed / 0 failed**, exit 0 |
| `cargo test -p aperture-core --test card_printings --locked` | **11 passed / 0 failed**, exit 0 (includes the 2 new slice-32 tests) |
| `cargo test --workspace --locked` | **exit 0 — 118 suites, 833 passed, 0 failed, 1 ignored** |
| `cargo fmt --all --check` | **exit 0**, zero output |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | **exit 0** |

The single ignored test is pre-existing and unrelated: `start_flow_network.rs::
a_real_archidekt_commander_deck_is_certified_with_exact_unsupported_cards` ("network: fetches one real
public Archidekt deck"). The handoff's "833 passed / 0 failed" is accurate but does not mention this skip.

---

## 3. The four required corrections

### (a) Kamahl's Summons — PASS

**Printing vs live Scryfall.** `curl -H "User-Agent: …" https://api.scryfall.com/cards/ons/269` (also
`/cards/mom/337`) compared field-by-field with `fixtures/starter-corpus/slice32-sourced.json`:
`oracle_id 8b6ef0a2-cd90-44c8-8f11-e1983f2b1dfd`, `scryfall_id 0edc37c6-b6a8-424f-95dd-928d03c28542`,
`set ons`, `269`, layout `normal`, `{3}{G}`, `Sorcery`, power/toughness `null`, colour identity `[G]`,
`keywords []`, commander legality `legal`, and the oracle text
*"Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green
Bear creature token for each card they revealed this way."* — **every field identical**. Same exercise for
MOM 337 (`9c10934d-9016-43c4-a7ab-56cc7d8f671f`, `Trample\nOther creatures you control have trample…`,
`keywords ["Trample"]`, 6/5) — identical.

**Catalog row + effect.** `catalog::by_name("Kamahl's Summons")` → cost `{3}{G}`, type line `Sorcery`,
`spell_spec().effects == [EachPlayerMayRevealCreatureCardsForBears]`, `TargetSpec::None`; pool size 148.

**My own probe** (`C:/AI/r289probe`, source copied to `probe/`, full output in `probe-output.txt`), four
seats, **active player P2** (so APNAP cannot be confused with P0-first):

```
[1] Kamahl's Summons: four seats, active = P2 (not P0)
  REFUSED as expected: reveal with no pending choice
  first seat asked: P2; order: [2, 3, 0, 1]
  offers while P2 owes an answer: [RevealCreatureCards { cards: [] }, RevealCreatureCards { cards: [ObjectId(27)] }]
  REFUSED as expected: the same card named twice
  REFUSED as expected: a non-creature card from the chooser's own hand
  REFUSED as expected: a creature card from another seat's hand
  REFUSED as expected (NotYourDecision { expected: 2, got: 0 }): another seat answering a reveal it does not owe
  P0's empty reveal accepted (declining is legal)
  Bear tokens by seat P0..P3: [0, 1, 1, 2]
  hands after: P2 2 P3 2 P0 1 P1 2 (spell + no card moved)
  CardsRevealed events: [(2, 1), (3, 2), (0, 0), (1, 1)]
  REFUSED as expected: reveal after the queue resolved
```

* (i) **every live seat asked in APNAP order from the active player** — order `[2,3,0,1]` with anchor P2;
  a seat holding no creature card (P0) is still asked and simply answers empty.
* (ii) **tokens equal each seat's own reveal** — P2 revealed 1 → 1 Bear; P3 revealed 2 → 2 Bears; P1
  revealed **1 of its 2** creature cards → exactly 1 Bear (arbitrary subset really is accepted); P0
  declined → 0. Nothing is created before the last answer.
* (iii) **cards stay in hand** — all four revealed cards remain `Zone::Hand`; P2's hand only lost the cast
  spell.
* (iv) **refusals** — duplicate card, non-creature card in the chooser's own hand, another seat's creature,
  a reveal with no pending choice, a reveal after the queue resolved: all rejected (`EngineError::Illegal`),
  and the non-entitled seat is rejected with `NotYourDecision`. The pending choice is unchanged after each
  refusal.
* (v) **declining is legal** — empty reveal accepted for P0 in test 1 and for every seat in the 3-seat
  all-decline probe (`[2] tokens after everyone declined: 0`).

Bears verified as 2/2 green Bear tokens on the battlefield of the revealing seat.

### (b) Surrak and Goreclaw's printed Trample — PASS

Probe `[3]`: `has_trample=true`, oracle first line `Trample`; real combat (6/5 blocked by a 2/1 Elite
Vanguard) leaves the blocker in the graveyard with 2 damage marked and deals **exactly 5** to the defending
player (1 lethal assigned, 5 trample over, CR 702.19b). The MOM 337 record in
`fixtures/starter-corpus/slice32-sourced.json` carries `keywords: ["Trample"]` (live-checked above) and
`card_printings::surrak_and_goreclaw_models_the_trample_its_printing_records` asserts it against the row.

### (c) Committed mandatory-target regression — PASS (with a coverage caveat)

`crates/core/tests/issue17_slice32.rs::a_mandatory_target_trigger_never_offers_or_accepts_declining` asserts
**both** halves: the offer list does not contain `DeclineTriggerTarget` while offering the legal
`ChooseTriggerTarget`, and `submit_err(DeclineTriggerTarget)` is `EngineError::Illegal` with the pending
choice intact. I reproduced it in my probe (`[4]`): offers are only the two `ChooseTriggerTarget` targets;
declining is refused.

Caveat (N3): the *behaviour* is not new — `trigger_target_is_optional` was introduced in `e53b987`
(slice 31), so this delta is test-only coverage of an existing guard, and the retained mutation proves only
the **refusal** half (it removes the legality guard; the "not offered" assertion is gated by a separate
`if` at `engine.rs` ~8438 that no mutation exercises).

### (d) slice-31 tip CI receipt committed — PASS

`git cat-file -e 770cb64:evidence/issue17/slice31/exact-head-ci-e56b138.json` succeeds; the path **doesn't
exist at `e56b138`** (added by `a987dde`). Contents: `head e56b138131be…`, `total 10`, 10 checks, all
`conclusion: success`, all `head_sha` = `e56b138131be…` — matching live `gh api
commits/e56b138/check-runs` (10 runs, 10 success, one head_sha).

---

## 4. Mutations (independent re-runs)

I re-ran three of the four retained mutations myself in `C:/AI/r289tree` and restored every file from the
worktree copy, verifying the recorded sha256 afterwards:

| Mutation | Observed | Restored |
|---|---|---|
| `let count = cards.len()` → `0` (engine) | `every_seat_reveals_in_apnap_order_and_creates_that_many_bears` **FAILED** at `issue17_slice32.rs:197`, 3 passed/1 failed, exit 101 | `engine.rs 5cf6df44…27c5`, 4/4 green again |
| `card_trample!` → `card!` for Surrak (catalog) | `surrak_and_goreclaw_prints_its_own_trample` **FAILED** at `issue17_slice32.rs:318` | `catalog.rs 2c4919db…8434`, 4/4 green again |
| legality guard `&& trigger_target_is_optional(...)` removed | `a_mandatory_target_trigger_never_offers_or_accepts_declining` **FAILED** at `tests/common/mod.rs:216` ("expected rejection") | `engine.rs 5cf6df44…27c5`, 4/4 green again |

The recorded pre/post-mutation hashes in `evidence/issue17/slice32/{pre,post}-mutation-sha256.txt` equal the
hashes of the reviewed bytes, so the "restored byte-for-byte" claim is consistent. I did not re-run the
fourth mutation (`tokens created before the last answer`); its log is internally consistent and its target
assertion is the same one my first mutation discriminated.

---

## 5. Journey reproducibility (out of tree, `git archive 770cb64` → `C:/AI/r289tree`)

**slice 32** (`evidence/issue17/slice32-journey-driver`, `decks/package-slice32.txt`, seed 3232, 4 seats,
40 life, `cargo run --release --locked`):

| Recorded claim | My run |
|---|---|
| digest `f6e65209dba9a24c84b05c3990e39691d5bd5cccc767efd41a5cb686d0fdbff4` | **match** |
| 1362 actions / 32 turns / 3660 replay events | **match** |
| terminal `Win{P3}` | **match** |
| snapshot `5e2a53302c563390e8b03ee0b2f5193fd09f6ec1317d75bae2ea538d04540488` | **match** |
| 3 Kamahl casts, 12 reveal answers, 23 revealed cards, 23 Bears, 4 Surrak casts, 10 Surrak combat-damage events | **match** |
| `demo-journey.json` / `demo-save.json` / `demo-replay.json` byte-identical to the committed files | **identical** (`7142783d…`, `7c9b6d37…`, `a95721c9…`) |
| release `aperture-cli replay` matches | **match** — `replay seed 3232 — 4 seat(s), 1362 action(s) … replayed digest f6e65209dba9 \| matches recorded \| events 3660`, exit 0 |

**Drift claim.** I ran every driver that exists in the archive:

* slice 24 → `697a846d6cd74fc0c13bb8f2c09d5da6d5fb0561ee6ba2a01e990c87a7e6ee91`, 4130 events, **1602
  steps**, 37 turns — exactly the refreshed record; its three regenerated artifacts are byte-identical to
  the committed ones. The base (`e56b138`) record was `a41bbfd2…` with **4127** events, same 1602 steps, so
  the Surrak repair moved exactly that journey.
* slice 31 → `36e181859f398146b2cac38278a712db4ed0cddc4372f9e23642ce57a51da79f` (1917 steps, 46 turns,
  4930 events) — unchanged, as claimed.
* slices 18, 19, 20, 21, 23, 25, 28, 29, 30 → all reproduce their recorded digests (**unchanged**).
* slice 22 → my `--locked` run **fails** ("cannot create the lock file … because --locked was passed"):
  that driver has no committed `Cargo.lock`. Re-run without `--locked` it produces
  `17f03348d81c1a92a6bd578db9ce2be95737985a6e255ff2fa769389e128a04d` = the committed digest, i.e. the
  substance holds but the scripted invocation cannot succeed from a clean export.
* slice 26 → **not reproducible from the reviewed tree**: `evidence/issue17/slice26-journey-driver` exists
  only as an untracked working-tree directory (its `demo-journey.json` is committed with digest
  `86bbd60b…`, but no driver source is in git at `770cb64`).

So "exactly one previously recorded digest moved" is **confirmed for the 10 drivers that are actually
committed and runnable**, and the two remaining lines of `replay-drift.log` are not reproducible as claimed
(N4).

---

## 6. Corpus audit recomputed from source (not from the writer's script)

Supported names were dumped from the candidate crate itself (`catalog::all()`, 148 rows), then compared
against the eight manifest deck files + commanders (`corpus-audit-review289.py`):

```
catalog_rows = 148
union_names  = 459
supported    = 136
missing      = 323
  isamaru-white-aggro    status=planned   distinct=59 missing=55
  jasmine-vanilla-stompy status=certified distinct=61 missing=0
  talrand-draw-go        status=planned   distinct=59 missing=51
  krenko-goblin-tokens   status=planned   distinct=59 missing=48
  ayula-bear-counters    status=planned   distinct=59 missing=17
  elas-sacrifice         status=planned   distinct=60 missing=52
  tatyova-landfall       status=planned   distinct=60 missing=52
  zada-spells            status=planned   distinct=59 missing=53
per_list_missing = [55, 0, 51, 48, 17, 52, 52, 53]
certified        = 1 ['jasmine-vanilla-stompy']
manifest_disagreements = []        # all eight lists, computed == declared
ayula_missing = 17
```

Every number in the handoff's audit claim is reproduced, and every manifest `missing_cards` list equals my
independent computation (no disagreements, either direction).

---

## 7. Live CI (`gh`, this session)

| Commit | Check-runs | Conclusions | head_sha |
|---|---|---|---|
| `770cb64` (reviewed head) | 10 | **10 success**, 0 other | all `770cb64…` ✔ |
| `a987dde` (docs/evidence commit carrying the code tree) | 10 | **10 success** | all `a987dde…` ✔ |
| `e56b138` (slice-31 tip, receipt check) | 10 | **10 success** | all `e56b138…` ✔ |
| `40a87aa` (code head itself) | **0** | — | — |

Detail worth stating precisely: the slice-32 code commit `40a87aa` has **no** check-runs of its own; the
10/10 that covers it is keyed to `a987dde`, whose tree — verified by `git diff 40a87aa 770cb64 -- crates
fixtures decks` being empty — is code-identical. That is honest as written ("keyed to a987dde") but it is CI
evidence for the tree, not for the code commit.

---

## 8. Non-blocking findings (carry into the next increments)

**N1. The offer surface is bounded, not the printed choice.** `legal_actions()` for `ApnapReveal` enumerates
exactly two options: "reveal no creature cards" and "reveal every creature card in hand". The action itself
accepts any subset (my probe had P1 reveal 1 of 2), so the rules model is right, but no interactive or AI
surface can express an intermediate subset. Disclosed in `docs/issue17-starter-corpus.md` and the
checkpoint. Harmless in practice for this card (revealing is free and strictly dominant), but a
human-facing gap the lane should close when the offer builder is generalised.

**N2. Undisclosed `coach.rs` edit.** `crates/core/src/coach.rs` (+15) is missing from the checkpoint's list
of forced plumbing, and it rewrites the existing APNAP-sacrifice privacy wording. Mechanical and
dimension-forced, inside core scope, no test asserts the string — but the disclosure is one file short.

**N3. The mandatory-target correction is test-only, and half of it is not mutation-proven.** The guard dates
from `e53b987`; this slice adds the regression test (which is what review 288 asked for). The retained
mutation removes the legality guard and fails the test at the refusal assertion only; nothing mutates the
offer gate at `engine.rs` ~8438, so the "not offered" assertion is asserted but not demonstrated to be
discriminating.

**N4. The drift log is not reproducible as scripted** (details in §5): `slice22` has no committed
`Cargo.lock` and cannot run under `--locked` from a clean export; `slice26-journey-driver` is not committed
at all although its digest appears in `replay-drift.log`. Either commit those two drivers (locks included)
or drop the lines and narrow the prose "every committed journey driver was rebuilt and re-run", which is
currently an overstatement.

**N5. One checkpoint sentence overstates the journey data.** "12 reveal answers in APNAP order (the order
rotates with the active player each cast)" — the recorded `reveal_seats` is `[1,2,3,0]` three times, i.e. a
single anchor seat P1. Rotation is *not* evidenced by this journey (my probe demonstrates the anchoring
from a non-zero seat, not the journey). The APNAP-order part of the claim is true and is what matters;
delete the parenthesis.

**N6. "All four corrections review 288 required".** Review 288 listed the Surrak repair, the mandatory-target
test and the CI receipt as **non-blocking** findings to carry forward, plus the standing corpus work. The
substance of all four is delivered; the word "required" is the writer's framing, not the reviewer's.

**N7. Minor evidence cosmetics.** `evidence/issue17/slice32/corpus-audit.py` asserts 148 while its failure
message says "expected 147". The workspace gate has 1 ignored (pre-existing network) test that the summary
line omits; "833 passed / 0 failed" is nonetheless correct.

---

## 9. Scope and state of the issue

The delta stayed inside core/corpus/deck fixtures as intended, apart from the disclosed mechanical
`crates/ai`/`crates/ui` arms (plus the `coach.rs` arm noted in N2). No desired starter list
(`decks/starter-corpus/*.txt`, `planned/*.txt`) changed; the manifest delta is the single removal of
"Kamahl's Summons" from the Ayula `missing_cards`; the only new deck is the evidence-only 40-card
`decks/package-slice32.txt`.

**The original issue #17 contract remains UNFINISHED.** At the reviewed head there are **1 of 8** desired
100-card starter lists certified, **136 of 459** union names supported (**323 missing**, Ayula still 17
names short), and the four-seat evidence run is a 40-card package, not a certified starter. Nothing here
justifies closing the issue, merging, or unlocking the parked downstream tickets; the card returns to
sourced implementation (next per `evidence/issue17/slice27/AYULA-NEXT-PACKAGE.md`, plus Beorn's Hospitality
which still needs layer 7a).

---

## 10. Where the handoff's evidence claims are accurate, and where they are not

Accurate, reproduced by me: catalogue/printing agreement against live Scryfall; focused and workspace gate
numbers; fmt/clippy; three of four mutation REDs with byte-for-byte restore; the slice-32 journey digest,
step/turn/event counts, snapshot, artifact byte-identity and the independent CLI replay; the slice-24
`697a846d`/4130 refresh and the slice-31 `36e18185` non-move; the 148/459/136/323 audit with per-list
`55/0/51/48/17/52/52/53`; the 10/10 CI at `a987dde` (and at `770cb64`); the committed slice-31 CI receipt.

Overstated or imprecise: the drift sweep ("every committed journey driver was rebuilt and re-run" — slice
26's driver is untracked, slice 22's cannot run under `--locked`); "the order rotates with the active player
each cast" (the journey shows one anchor seat); "all four corrections review 288 *required*" (review 288
listed three of them as non-blocking); and the forced-plumbing list omits `coach.rs`.

## Artifacts in this review directory

* `probe-output.txt` — full output of my probe (all probes passed).
* `probe/probe-main.rs`, `probe/Cargo.toml` — the probe crate (built at `C:/AI/r289probe`).
* `corpus-audit-review289.py`, `catalog-names-from-crate.txt` — audit script and the crate-dumped name list.
