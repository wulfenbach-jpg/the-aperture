# REVIEW 288 — slice 31 (Beorn the Fierce), contract lens

Reviewed head: `e56b138131bec70458caaadaaaae0240a1b82e43` (code head `e53b987e50a46d0cddb44665eb6d94f9cfe7fefc`).
Range: `073fd19..e56b138`. Draft PR46, live head read back as `e56b138`, OPEN / draft / mergeable.
Verdict: **APPROVED_INCREMENT** for the slice-31 delta. The ORIGINAL #17 contract is still
UNFINISHED, so the card returns to sourced implementation, not completion.

Mode: read-only. Every build and test below ran in `git archive` copies of the recorded head
(`C:/AI/r288a` = e56b138, `C:/AI/r288base` = 073fd19). I edited no deliverable file; my only
additions are this untracked report, out-of-tree probe sources and scratch outputs.

## Acceptance criteria of issue #17 mapped to evidence at this head

| # | Criterion | State at e56b138 |
|---|---|---|
| 1 | Eight explicitly supported 100-card Commander lists | **1 of 8** (`jasmine-vanilla-stompy`, 99 library + 1 commander); re-certified this run, seeds 1/2/3 digests identical to the recorded certificate |
| 2 | Every included card and required interaction implemented with conformance coverage | **135 of 459 union names**; 324 missing (Ayula 18 short). One included card, `Surrak and Goreclaw`, still lacks its own printed trample (disclosed by the writer, not fixed) |
| 3 | Four-seat games on exact deck snapshots | Exercised on a **40-card evidence package** (`decks/package-slice31.txt`), not on a desired 100-card starter; snapshot exact, `snapshots_exact: true`, digest reproduced |
| 4 | Document per-card/interaction exclusions | Present and updated (`docs/issue17-starter-corpus.md` slice-31 section, known limits, Surrak gap) |

## What I ran, and what it returned

1. Out-of-tree `git archive e56b138` → `cargo run --release --locked` in
   `evidence/issue17/slice31-journey-driver`: **exit 0**, terminal `Win{P2}`, seed 3131,
   **1917 actions / 46 turns / 4930 replay events**, digest
   `36e181859f398146b2cac38278a712db4ed0cddc4372f9e23642ce57a51da79f`, snapshot
   `370cfe3b…`, 7 Beorn casts, 13 beginning-of-combat triggers, 13 trample counters, 13 Bear
   additions, 8 Bear-count gates declined.
2. The three generated artifacts are **byte-identical** to the committed evidence:
   `demo-journey.json 0808bd32…`, `demo-save.json aa071b30…`, `demo-replay.json a6d172e5…`.
3. `aperture-cli replay` built by me from the same archive: 1917 actions, digest matches,
   4930 events.
4. `cargo test -p aperture-core --test issue17_slice31 --locked`: **8 passed / 0 failed**.
5. `cargo test --workspace --locked`: **117 suites / 827 passed / 0 failed**, exit 0.
   `fmt --all --check` exit 0. `clippy --workspace --all-targets -D warnings` exit 0.
6. My own mutation (delete the `CounterKind::Trample` read from `Game::has_trample`) fails
   exactly `trample_counter_lets_the_creature_trample_over_a_blocker` (life 20, expected 17);
   `engine.rs` restored byte-for-byte (`8fe51710…30d2` before and after, all 8 tests green again).
7. My own probe crate (scratch copy only, 3 tests, all passed):
   * a **mandatory** target trigger (Ayula) never offers `DeclineTriggerTarget` and the action is
     refused with `EngineError::Illegal`, leaving the pending choice intact;
   * the anthem reads another card's effective subtype: Circle of the Moon Druid is 6/4 on its
     controller's turn (base 4/2 Bear + +2/+2) and 2/4 on the opponent's turn; Beorn stays 6/6;
   * a trample counter alone, after the Bear addition expired, still tramples exactly 1 over a
     1/1 blocker on the controller's next turn (life 19).
8. My own corpus audit (catalog.rs macro scan + eight deck files + commanders, compared to every
   manifest `missing_cards`): **147 catalog / 459 union = 135 supported + 324 missing**, per list
   `55/0/51/48/18/52/52/53`, **1 of 8 certified**, manifest equality on all eight lists.
9. Live Scryfall `hob/119`: id `367d5f8b…`, oracle `30ca8e92…`, `{3}{G}{G}`, Legendary Creature —
   Bear Shapeshifter Warrior, 6/6, text and `keywords:["Trample"]` **identical** to fixture,
   catalog row and ability definition.
10. Live CI: `commits/e53b987/check-runs` = 10 completed, 10 success, all keyed to e53b987;
    `commits/e56b138/check-runs` = 10 completed, 10 success, all keyed to e56b138;
    `gh pr checks 46 --required` shows every required context passing.
11. `aperture-cli certify --starter jasmine-vanilla-stompy --seats 4 --seeds 1,2,3` on my build:
    certified, snapshot `4312ab2a…`, seeds 1/2/3 → 2364/2802/1685 steps, digests
    `8e4feea3…`, `4677eb42…`, `520e8a0d…` — identical to the recorded certificate.
12. Regression sweep: 16 of the 18 committed `evidence/issue17/slice*/demo-replay.json` still
    replay. slice15 and slice16 fail with **the same errors on a base build of 073fd19**
    (Bearscape graveyard activation rejected; slice16 digest mismatch), and those replay files
    are unchanged between the heads — pre-existing stale fixtures, not a slice-31 regression.
13. Scope: `crates/ai` and `crates/ui` untouched; no desired deck snapshot (`planned/*.txt`,
    `jasmine-vanilla-stompy.txt`) changed; the manifest delta is only the removal of
    "Beorn the Fierce" from the Ayula missing list. Test edits are exact pool/Ayula count updates
    (146→147, 19→18), not weakened assertions.

## Non-blocking findings to carry into the next increments

1. **Untested legality guard.** `DeclineTriggerTarget` is offered and validated only for a printed
   "up to one" target, but no committed test exercises the refusal path for a mandatory target;
   the slice tests cover only Beorn's optional case. Behaviour is correct (my probe), coverage is
   not. Add a regression test with a mandatory targeted trigger when a card is next touched.
2. **Surrak and Goreclaw printed trample.** An included, supported card whose own printed
   keyword is not modelled (its static grants trample only to other creatures). This is a direct
   criterion-2 gap in an already-approved card; it needs its own reviewed increment and the
   resulting journey-digest refresh, and it should not stay only in prose.
3. **Tip CI receipt is untracked.** `evidence/issue17/slice31/exact-head-ci-e56b138.json` exists
   only in the worktree; the committed receipt covers e53b987. Commit it with the next evidence
   commit or stop citing a tip-level 10/10 that has no committed artifact.
4. Stale slice-15/16 replay fixtures (pre-existing, already noted in
   `evidence/issue17/legacy-replay-note.md`) — evidence debt, unchanged by this slice.
5. `Effect::AddTargetCreatureType`'s display string says "becomes a creature with type Bear";
   accurate for its only target spec (a creature you control), misleading if the effect is ever
   reused on a non-creature. Wording only.

## Verdict

The slice-31 delta is correct, reproducible from its own recorded bytes, and does exactly what it
claims: a beginning-of-combat trigger, an honest "up to one" target with decline, a keyword
counter read by `has_trample`, a layer 4 additive subtype effect, and a layer 7c subtype anthem
that reads effective subtypes. No functional defect was found and I made no implementation edits.

The original #17 contract remains open: one certified list of eight, 324 unsupported union names,
and a 40-card demonstrator rather than a desired 100-card starter. Continue on the same card,
PR46, branch `aperture/issue-17` and the saved writer/reviewer histories.
