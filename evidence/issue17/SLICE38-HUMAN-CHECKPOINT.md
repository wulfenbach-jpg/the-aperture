# Slice 38 — Spider-Ham, Peter Porker (SPM 114), the pool's first Food token

Card / branch / PR are unchanged: kanban `t_67778feb`, branch `aperture/issue-17`,
draft PR46 (https://github.com/wulfenbach-jpg/the-aperture/pull/46). Approved
increments 1–37 were **not** re-audited. This slice closes review 299's two
record-integrity items and continues the supervisor's "Ayula to zero" package
with the next sourced name.

* **Code head of record:** 6bb0740 (implementation + tests + fixture + docs +
  the two pool-size guards + the manifest drop).
* **Evidence tip:** 1cd966d (journey driver and its artifacts, CI receipt,
  this checkpoint).
* **Status of this increment: UNVERIFIED.** No independent review exists for this
  head yet. `evidence/issue17/review300/REVIEW300.md` was authored inside this
  same run and is therefore a **self-check, not a review**; it claims no verdict,
  and slice 38 must not be described as independently reviewed until a separate
  review run adjudicates `1cd966d` from `git archive` and posts its own verdict.
  The corpus counts here are *implementation* counts and stand on their own.
* PR stays OPEN, **draft**, `BEHIND` master. No merge, master push, issue
  closure, downstream unlock, new team or global config change.
* **Original issue-17 contract remains unfinished**: 1 of 8 desired 100-card
  lists certified, Ayula **13** names short, **319** of 459 union names
  unsupported.

## Review 299's required items — closed

1. **Record-integrity (driver scope).** The slice-37 driver's module doc and its
   report `purpose` string no longer claim that a pairing *ends* in the game.
   They now state what the run demonstrates — cast, pairing created, pairing
   observed, every surviving pair still granting its bonus — and name the focused
   `issue17_slice37` test (partner killed, survivor back to printed
   characteristics) as the ending evidence. `driver-journey.json` was regenerated
   from the corrected source: digest unchanged at `c4c5985cd048…`, and
   `driver-replay.json` / `driver-save.json` are byte-identical to the previous
   ones.
2. **Tip receipt.** `evidence/issue17/slice37/exact-head-ci-07ad00e.json` is now
   **committed** (10 check runs, all `completed/success`, every `head_sha` =
   `07ad00e`), and the two stray untracked copies in `slice35/` plus the stray
   empty `.err.log` are removed.
3. **Ayula to zero — one more name closed.** Spider-Ham, Peter Porker is
   implemented, sourced, tested and exercised in a real four-seat game (below);
   the audit moves to 140 of 459 supported and Ayula 13 short.

## What was added

`Spider-Ham, Peter Porker` (`{1}{G}`, 2/2 Legendary Creature — Spider Boar Hero)
— the pool's first **artifact token** and its first **list-filtered subtype
anthem**. Sourced from Scryfall **SPM 114**: immutable printing id
`41f18f42-b86b-4a12-9f0d-76b761571195`, oracle id
`00b50215-e832-417f-9c80-2bc3050b6ebb`, retrieved **2026-10-28T00:54:14Z**. All
five printings (`spm 114`, `spm 201`, `pspm 114s`, `pw25 10`, `om1 118`) carry
one identical text, type line, cost and P/T. Raw response, the printings list and
the retrieval stamp are in `evidence/issue17/slice38/sources/`; the enforced
fixture is `fixtures/starter-corpus/slice38-sourced.json` and `card_printings`
compares the catalog row against it.

Printed text:

> When Spider-Ham enters, create a Food token. (It's an artifact with "{2}, {T},
> Sacrifice this token: You gain 3 life.")
> Animal May-Ham — Other Spiders, Boars, Bats, Bears, Birds, Cats, Dogs, Frogs,
> Jackals, Lizards, Mice, Otters, Rabbits, Raccoons, Rats, Squirrels, Turtles,
> and Wolves you control get +1/+1.

### Reusable surfaces, not a one-card patch

* **The Food token (CR 111.10, 205.3).** `CardId::FoodToken` is a card definition
  in the catalog's token table — `Token Artifact — Food`, no power/toughness,
  never importable, never part of `catalog::all()` — created by
  `Effect::CreateFoodToken`. Its printed ability is the pool's first activated
  ability whose cost combines a mana cost, `{T}` **and** "Sacrifice this token":
  the engine pays `{2}`, taps the token, moves it to the graveyard as part of the
  cost (CR 602.2b) and only then puts the ability on the stack, so the token has
  already left the battlefield when "You gain 3 life" resolves.
* **The anthem (CR 613.4c + 205.3).**
  `StaticAbility::OtherControlledCreaturesWithAnySubtypeGet` is a layer 7c
  additive static over a printed *list* of subtypes. It asks
  `Game::has_subtype`, so a creature made a Bear by a layer 4 effect counts and a
  creature with none of the listed subtypes never does; the printed "Other"
  keeps the source out of its own set, while a second copy of Spider-Ham (itself
  a Spider) is pumped by the first copy's anthem, exactly as printed.
* `Game::has_subtype` is now `pub`: it is the engine's own effective-subtype
  query, and the journey driver uses it (against the printed list) rather than
  re-deriving subtypes, so the driver cannot disagree with the rules layer.

## Verification

* `crates/core/tests/issue17_slice38.rs` — **6/6**: sourced text and both printed
  abilities; entering creates exactly one Food token that is an artifact, not a
  creature, and whose ability is offered once `{2}` is payable; activating gains
  exactly 3 life, spends the mana, sacrifices the token as a cost and removes it
  from the battlefield; the activation is neither offered nor legal without its
  cost (no mana / tapped token); the anthem pumps a Bear, leaves an Elf alone and
  never pumps the source; and the anthem reads the *effective* subtype (Circle of
  the Moon Druid 5/3 on its turn, 8/6 after Giant Growth).
* `card_printings` — **15/15** (the new record plus the exhaustive pool-coverage
  test).
* `catalog_pool` **5/5**, `issue17_slice15_corpus` **1/1**, `issue13_commander_combat`
  **43/43** after the two pool-size guards moved 151 → 152 and Ayula 14 → 13.
* Whole workspace: **122 suites / 867 passed / 0 failed**, exit 0 (evidence/issue17/slice38-workspace-tests.log).
* `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --locked
  -- -D warnings`: both exit 0 with no warnings (`evidence/issue17/slice38/gates.log`).

### Four-seat journey (delivery bar)

`evidence/issue17/slice38-journey-driver` plays the exact `decks/package-slice38.txt`
snapshot (4 seats, 40 life, seed 3838) choosing only from the masked legal actions
of the seat with priority:

* terminal **`Win{P2}`**, **1450** steps, **34** turns, **3845** replay events,
  digest **`93329b6394838334e60bd2b9ecbfa646c3044c4a91ba347d1c59cb387a5bbc34`**;
* **8** Spider-Ham casts, **8** Food tokens created, **8** Food activations,
  **24** life gained — reconciled exactly (3 life × 8 activations) against every
  positive life change in the game, and the package has no other life gain;
* **7 110** anthem assertions (once per listed-subtype creature per action
  boundary), each requiring at least printed + 1/+1 beside its controller's
  Spider-Ham, with the source asserted at printed + 1/+1 per *other* copy of the
  anthem it controls (never its own);
* the replay reproduces the digest and every seat's life total, and a fresh
  `SessionSave` reloads to the identical state; a separate release-CLI replay of
  the driver's own replay file matches.

## Corpus audit (recomputed in this slice)

```
catalog_cards      = 152
union_names        = 459
supported_union    = 140
missing_union      = 319
per_list_missing   = [55, 0, 51, 48, 13, 52, 52, 53]
certified_lists    = 1  (jasmine-vanilla-stompy)
```

Script and log: `evidence/issue17/slice38/corpus-audit.py`,
`evidence/issue17/slice38/corpus-audit.log`. The manifest's Ayula
`missing_cards` entry was regenerated from the audit, not hand-edited
(`evidence/issue17/slice38/update-manifest.py`).

Ayula's remaining 13 names, in the order the package leaves them:

Dúnedain Rangers, Evercoat Ursine, Lightning Greaves, Lumra, Bellow of the Woods,
Owlbear Cub, Rampaging Yao Guai, Ruxa, Patient Professor, Spirit of the Aldergard,
Swiftfoot Boots, The Earth King, Thijarian Witness, Titania's Command, Ursine
Monstrosity.

## Disclosed limits

* The journey is a **40-card evidence package**, not a 100-card certificate and
  not product AI or strength evidence; the driver is a masked demonstrator.
* The Food token's ability is modelled with its printed cost and effect only: it
  is not a generic "sacrifice an artifact" outlet for other cards, and nothing in
  the pool can copy, blink or recur it.
* The anthem's recipient list is the printed list in singular form (CR 205.3);
  no pool card changes a creature's subtypes into a listed one except the layer 4
  forms already supported (Circle of the Moon Druid), which the tests cover.
* The journey's anthem assertion skips creatures whose printed base is replaced
  by a layer 7b static (Circle of the Moon Druid's 4/2 Bear Form), because
  "printed + 1/+1" is not the right baseline for them; the focused test asserts
  that card's exact 5/3 and 8/6 values instead.
* Nothing in the pool prints hexproof, shroud or protection, so no targeting
  interaction with the token is claimed.
* This worktree had **no** pre-existing tracked edits when this slice started
  (`git diff` empty after the review-299 record commit); the only tracked change
  outside the slice's own files is the manifest row regeneration.
