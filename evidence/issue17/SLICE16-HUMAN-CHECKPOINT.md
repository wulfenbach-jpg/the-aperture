# Issue 17 — slice 16 checkpoint (mass destruction + targeted untap)

## Result

Two more sourced Ayula-list cards on the SAME draft PR46 / branch
`aperture/issue-17`, each implemented from its authoritative printed text
(Scryfall printing URL + retrieval stamp in
`fixtures/starter-corpus/slice16-sourced.json`):

* **Fade from History** `{2}{G}{G}` sorcery — "Each player who controls an
  artifact or enchantment creates a 2/2 green Bear creature token. Then destroy
  all artifacts and enchantments."
* **Little Bear** `{2}{G}` 3/2 Bear with flash — "When this creature enters,
  untap another target creature you control. If that creature is a Bear, put a
  +1/+1 counter on it."

New behaviour, all of it reachable through ordinary legal actions:

* `Effect::BearTokenForEachPlayerWithPermanent` — the token's controller is the
  player who controlled the artifact/enchantment, not the spell's controller
  (CR 111.2). Seats with neither permanent get nothing; eliminated seats are
  skipped; order is APNAP from the effect's controller.
* `Effect::DestroyAllArtifactsAndEnchantments` — one simultaneous destruction
  (CR 704.3): every zone change is recorded first, then the dies-watchers are
  dispatched once for the whole batch, mirroring the state-based-action path.
* `TargetSpec::CreatureYouControl` — "target creature you control", with the
  printed word "another" enforced by excluding the trigger's own source, so an
  opponent's creature is never offered and the Bear can never target itself.
* `Effect::UntapTargetAndCounterIfBear` — untap is unconditional (CR 701.26a),
  the +1/+1 counter reads the **target's** printed subtype.

Corpus: **131 catalog; 459 union names = 119 supported + 340 missing; 1 of 8
desired starters certified (jasmine-vanilla-stompy)**. Ayula 36 → 34 missing.
All eight desired 100-card snapshots are unchanged. Full #17 is unfinished.
Per-list `missing_cards` sums to 345 (names repeat across lists); the distinct
union missing count is 340.

## Honest RED evidence

* `red-cards-unsupported.log` is the first run of the new suite, before either
  card existed in the catalog: 7/7 tests failed with
  `<card> must be supported`.
* `red-mutation-always-counters.log` is a deliberate mutation (the Bear-subtype
  condition forced true in the engine): only
  `little_bear_untaps_a_non_bear_without_countering_it` failed, proving that
  test discriminates the conditional rather than merely observing an untap. The
  mutation was reverted and the suite is green (7/7).

## Evidence

* Focused: `issue17_slice16` 7/7, `card_printings` 8/8 (the catalog audit now
  includes the slice-16 fixture), `catalog_pool` 5/5, plus the two suites whose
  pinned counts this slice changes: `issue13_commander_combat` 43/43 and
  `issue17_slice15_corpus` 2/2 (`focused-refix.log`, 65 tests, 0 failures).
* `release-build.log` — locked release workspace build PASS.
* The first full workspace run (`workspace-tests-prefix-fix.log`) finished 41
  suites green and one failure: `the_three_slice15_cards_leave_the_ayula_gap`
  still pinned 36 missing names / 129 pool cards. That is the same kind of
  pinned-count assertion this slice legitimately moves; the test's purpose (the
  three slice-15 cards stay supported and the recorded gap matches the compiled
  pool) is unchanged, so the numbers were corrected to 34 / 131 and the focused
  re-run above is green. The exact-head CI run is the full-suite gate for this
  head.
* Evidence-only masked legal-action journey (NOT product AI, NOT strength
  evidence): exact four seats, 40 life, `decks/package-slice16.txt`, seed 1616,
  terminal P2, **1957 actions / 44 turns / 5060 events**, digest
  `ad5cd86d3432…`; separate release-CLI replay reproduces the digest
  (`native-demo-replay.log`). Casts: Fade from History 8, Little Bear 7,
  Bearscape 6, Sol Ring 6, Grizzly Bears 7, Werebear 13. The mass destruction
  really removed artifacts and enchantments (Sol Ring 6, Bearscape 6);
  8 Bear tokens were created; Little Bear's trigger resolved 6 times with a
  Bear target and granted a counter every time.
* The journey driver injects **Jasmine Boreal for all four seats** as commander
  (recorded in `demo-journey.json`); the package fixture carries no commander and
  nothing in this slice reads the commander's identity.

## Gaps

* No additional desired 100-card starter is certified. Nearest: Ayula 34,
  Krenko 48, Talrand 51, Elas 52, Tatyova 52, Zada 53, Isamaru 55.
* Equipment (Lightning Greaves, Swiftfoot Boots), soulbond, hideaway, saddled
  mounts, Ring temptation, regeneration, X-spells and multi-mode text remain
  unimplemented; the Ayula manifest's `missing_interactions` now names the real
  remaining ones instead of the token/counter work this and earlier slices
  delivered.
* `Ayula, Queen Among Bears` itself is still unsupported (its fight/counter
  trigger needs a fight primitive), so the list remains `planned`.
* No arbitrary Archidekt compatibility, full Commander, playing-strength or v1
  claim.

## Resume

Same card / PR46 / branch; saved writer and reviewer histories preserved under
`evidence/issue17/sessions/…` and `review-sessions/…`. Current-lane target only;
no cleanup, retired builds, global provider or model changes. No merge, master
push, issue closure or #26/#32 unlock.
