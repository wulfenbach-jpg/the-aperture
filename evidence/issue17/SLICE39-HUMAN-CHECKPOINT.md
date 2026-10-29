# Slice 39 — review 301 repair: the Food token really ceases to exist (CR 111.7)

Card / branch / PR / histories unchanged: kanban `t_67778feb`, branch
`aperture/issue-17`, draft PR46
(https://github.com/wulfenbach-jpg/the-aperture/pull/46). Approved increments 1–37
were **not** re-audited; slice 38 is the increment this repair completes.

Review 301 (`evidence/issue17/review301/REVIEW301.md`, independent run 303)
returned **CHANGES_REQUESTED** for one functional conformance defect in the new
Food token plus two false record statements. This slice is the repair, on the same
card.

* **Code head of record:** `a2a3b08` (registration fix, tests, ability comment,
  driver assertions, docs and both checkpoints). **Exact-head CI: 10/10 completed
  SUCCESS, every `head_sha` keyed to `a2a3b08`**
  (`evidence/issue17/slice38-repair/exact-head-ci-a2a3b08.json` + `.log`, captured
  with `timeout=120` on every `gh` call).
* **Status of this increment: UNVERIFIED pending a fresh independent
  re-adjudication.** The reviewer asked for exactly that after the repair, and a
  repair authored in the run that also claims to verify it is not independent.
  Nothing here is described as reviewed.
* PR stays OPEN, **draft**, `BEHIND` master. No merge, master push, issue closure,
  downstream unlock, new team or global config change.
* **Original issue-17 contract remains unfinished**: 1 of 8 desired 100-card
  lists certified, Ayula **13** names short, **319** of 459 union names
  unsupported (audit re-run this slice, `corpus-audit.log`).

## 1. Blocking defect — fixed at the registration level

`CardId::FoodToken` was created by `Effect::CreateFoodToken` but was missing from
`CardId::is_token()` (`crates/core/src/card.rs`) and from the `TOKEN_CARDS`
registry, so every CR 111.7 path treated the sacrificed Food as an ordinary card:
the cease-to-exist SBA skipped it, `move_to_graveyard_inner` recorded no
"will cease to exist" record, the state validator accepted the invalid state and
the public graveyard view showed it as a card. The reviewer's own probe showed the
consequence: three real cards plus four sacrificed Foods made seven graveyard
objects, so Werebear was 4/4 with the printed condition false.

The repair:

* registers `CardId::FoodToken` in `CardId::is_token()` **and** `TOKEN_CARDS`
  (the registry now also lists `SoldierToken`/`SpiritToken`, which had lived only
  in `tokens::TOKEN_CARDS`, so the registry enumerates every declared variant.
  The two constants are *not* identical and were not made identical:
  `tokens::TOKEN_CARDS` still holds the two `CardDef` rows it always did. Review
  306 corrected an earlier wording that said "the two lists are the same list");
* documents on `is_token()` that every CR 111.7 path reads it, so a new token
  variant must register in the registry — slice 40 went one step further and
  derived the predicate from `TOKEN_CARDS` (`is_token()` is now
  `TOKEN_CARDS.contains(&self)`), because review 306 found that the constant had
  no non-test consumer at this head;
* replaces the permissive lane assertion (`matches!(zone, Graveyard | Removed)`),
  which accepted exactly the state CR 111.7 forbids, with the printed outcome:
  the object is gone from state objects, absent from the player's graveyard list,
  absent from `view_for(P0).own_graveyard`, and both CR 111.7 records are present
  in the activation events ("is put into the graveyard and will cease to exist"
  and "token ceases to exist"), which also shows the cost was paid during
  activation rather than on resolution;
* corrects the `ability.rs` comment that stated the behaviour that did not happen;
* adds two regressions: **three real cards + four sacrificed Foods leave three
  objects and a 1/1 Werebear**, and a pool-wide token-registration test that walks
  every variant in both characteristics tables (`tokens::TOKEN_CARDS` and the
  catalog's `TOKENS`) and asserts `is_token()`, `TOKEN_CARDS` membership,
  non-importability and that the registry has no variant without characteristics.
  That second test is what keeps the next token from repeating the gap silently.

## 2. Record item — "first ability with all three at once" corrected

`ability.rs`, the slice-38 checkpoint and `docs/issue17-starter-corpus.md` all
claimed the Food ability was the pool's first with mana + `{T}` + sacrifice-self.
False: `Mind Stone` ("{1}, {T}, Sacrifice this artifact: Draw a card.",
`ability.rs:540`) and `Hedron Archive` ("{2}, {T}, Sacrifice this artifact: Draw
two cards.", `ability.rs:553`) already combine all three. What is new is that the
source is a **token**. All three places now say exactly that and name the two
earlier cards.

## 3. Record item — `issue17_slice15_corpus` count corrected

`SLICE38-HUMAN-CHECKPOINT.md` said **1/1**; the binary has two tests and both pass
(`2/2`, as slice 37 recorded). Corrected in place with a note.

## Verification

* `crates/core/tests/issue17_slice38.rs` — **8/8** (`focused.log`).
* `card_printings` **15/15**, `catalog_pool` **5/5**, `starter_corpus` **8/8**,
  `issue17_slice15_corpus` **2/2**, `issue13_commander_combat` **43/43**
  (`focused.log`).
* Whole workspace: **122 suites / 869 passed / 0 failed / 1 ignored**, exit 0
  (`workspace-tests.log`: +2 tests over slice 38's 867, the two new regressions;
  `gates.log` records `workspace_exit=0`).
* `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --locked
  -- -D warnings`: exit 0 (`gates.log`, `fmt.log`, `clippy.log`).
* **Mutation RED, from the committed bytes** (`mutate.sh`): removing the
  `CardId::FoodToken` arm from `is_token()` — the one line the repair adds — fails
  exactly the three discriminating tests (`every_token_card_is_registered_as_a_token`,
  `sacrificed_food_tokens_do_not_count_as_graveyard_cards_for_threshold`,
  `activating_the_food_gains_three_life_and_sacrifices_the_token`) and no others;
  the file is restored and `sha256sum -c` on the checked-out file is clean
  (`mutation-is-token-red.log`, `mutation-restored.log`). The committed runner was
  itself run this slice, closing review 296's non-blocking "no committed mutation
  runner" note for this card.

### Four-seat journey, regenerated

`evidence/issue17/slice38-journey-driver` still plays the exact
`decks/package-slice38.txt` snapshot (4 seats, 40 life, seed 3838) choosing only
from masked legal actions. Same seed, deck, seats, **1450** steps, **34** turns and
terminal **`Win{P2}`** as before, and a changed state digest — which is the point:
the final state no longer carries phantom Food objects.

* digest **`726fe20e72e17b353e8d7fc9f4f8317bc06ab6b2bfd9ab1fbc0f2fb1a871cb31`**
  (was `93329b63…`);
* **3861** replay events (was 3845): +8 "will cease to exist" records and +8
  removal state-based actions, one pair per activation;
* **8** casts, **8** Food tokens created, **8** activations, **8** tokens ceased
  to exist, **0** Food objects left at game end (was 4 in the winner's graveyard),
  **24** life gained reconciled as 3 × 8;
* the driver now *asserts* cessation from the engine's events
  (`foods_ceased == food_activations`, no Food object in a non-`Removed` zone, and
  any remaining Food object belongs to an eliminated seat) and reports both counts;
* the fresh `SessionSave` reload goes through `SessionSave::from_json`, which runs
  `GameState::validate()`, so the reload is now a real check that the corrected
  validator accepts the final state — the pre-repair save would have been rejected;
* `driver-journey.json` / `driver-replay.json` / `driver-save.json` regenerate
  byte-identically on a second run (`ARTIFACTS_IDENTICAL=yes`) and a separate
  release-CLI replay of the driver's own replay file reproduces the digest and all
  3861 events (`driver-cli-replay.log`).

## Disclosed

* The journey remains a **40-card evidence package**, not a 100-card certificate
  and not product AI or strength evidence.
* Nothing else in the pool changed behaviour: no other card creates or interacts
  with a Food token, and no other slice's replay is affected by an `is_token()`
  entry that only a Food object can match. The slice-35 and slice-37 journeys were
  replayed at this head and keep their digests (see `digest-preservation.log`).
* The two non-blocking notes review 290 left open (the printed-7b-before-7a
  ordering inversion, unreachable today; the `card_printings` "colour identity"
  wording) are still open and still disclosed.
* `evidence/issue17/review299/` and `review301/` are the reviewers' untracked
  artifacts and stay untracked.
