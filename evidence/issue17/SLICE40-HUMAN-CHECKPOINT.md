# Slice 40 — review 306's two record findings, closed in code rather than wording

Card / branch / PR / histories unchanged: kanban `t_67778feb`, branch
`aperture/issue-17`, draft PR46 (https://github.com/wulfenbach-jpg/the-aperture/pull/46).
Approved increments 1–38 are untouched; slice 39 was **approved** by review 306 at
`814b572` (code head `a2a3b08`), so this slice carries review 306's two
non-blocking record corrections and nothing else.

* **Reviewed base:** `814b572` (slice 39), verdict APPROVED_INCREMENT, report
  `evidence/issue17/review306/REVIEW306.md` (untracked reviewer artifact).
* **Status of this increment: UNVERIFIED pending independent review** — it was
  written by the same run that reports its gates, exactly like slices 39 and 38.
* PR stays OPEN, **draft**, `BEHIND` master. No merge, master push, issue closure,
  downstream unlock, new team or global config change.
* **Original issue-17 contract remains unfinished**: 1 of 8 desired 100-card lists
  certified, Ayula **13** names short, **319** of 459 union names unsupported.

## 1. R306-1 — the token registry is now the predicate, not documentation

Review 306 found that `card::TOKEN_CARDS` had **no non-test consumer** in
`crates/` while `is_token()`'s doc claimed a variant missing from it "would be
treated as an ordinary card once it left the battlefield". Two ways to fix a false
statement: reword it, or make it true. This slice makes it true:

```rust
pub fn is_token(self) -> bool {
    TOKEN_CARDS.contains(&self)
}
```

`is_token()` was a hand-written `matches!` list that duplicated the registry. It
is now derived from `TOKEN_CARDS`, so:

* registering a variant in the registry is what makes every CR 111.7 path — the
  cease-to-exist SBA (`engine.rs:6000`), the graveyard record hook (`:1971`), the
  state validator (`state.rs:1963`), the nontoken filters (`:2001/3443/6815/7897`),
  `library.rs:43` and `view.rs:211` — treat it as a token;
* a creatable variant left out of the registry is now a real defect with a single
  obvious fix site, instead of a silent one with two places to update;
* the pool-wide `every_token_card_is_registered_as_a_token` test's registry half
  became load-bearing rather than tautological, which is the property review 306
  asked for. Its comment now says so.

The member set is unchanged: `TOKEN_CARDS` holds Soldier, Spirit, Goblin, Drake,
Beast, Pest, Bear and Food, which is exactly the set the removed `matches!` listed.
No behaviour changes — the four-seat digest below is the check for that.

## 2. R306-2 — "the two lists are the same list" corrected

`SLICE39-HUMAN-CHECKPOINT.md` claimed the registry change made the two token lists
"the same list". They are not: `tokens::TOKEN_CARDS` is `[CardDef; 2]` (Soldier,
Spirit) and `card::TOKEN_CARDS` has eight entries. The checkpoint now says what is
true — the *registry* enumerates every variant declared by either characteristics
table — and records that slice 40 derived the predicate from it. The same
overstatement in `docs/issue17-starter-corpus.md` ("what the `is_token()`/
`TOKEN_CARDS` entry protects") is rewritten to describe the derivation.

## Verification

* `cargo fmt --all --check` — exit 0 (`slice40-fmt.log`, `slice40-gates.log`).
* `issue17_slice38` **8/8**, `card_printings` **15/15** (`slice40-focused.log`).
* Whole workspace **122 suites / 869 passed / 0 failed / 1 ignored**, exit 0
  (`slice40-workspace.log`, `slice40-gates.log`).
* `cargo clippy --workspace --all-targets --locked -- -D warnings` — exit 0
  (`slice40-clippy.log`).
* **The digest does not move**, which is the point of a predicate refactor with no
  membership change: the slice-38 four-seat journey replay re-verifies as
  `digest 726fe20e72e1 | matches recorded | events 3861` on a release CLI built at
  this head, and the slice-35 (`1c13d8fdcdce`, 5347) and slice-37
  (`c4c5985cd048`, 4530) replays are unchanged.
* Exact-head CI and the receipt for the code head are recorded in §4 once the
  checks complete; no CI claim is made for any commit before its runs exist.

## 3. Ayula feedstock for the next writer (no card implemented here)

The 13 remaining Ayula names were fetched from Scryfall in one `/cards/collection`
call this slice and stored verbatim, with the retrieval stamp, at
`evidence/issue17/slice40/sources/ayula-remaining-collection.json` and
`ayula-remaining-retrieval.json` (retrieved `2026-10-28T06:22:46Z`). This is
provenance feedstock, **not** an implementation and **not** a support claim: none
of these names is in the catalog.

Ordered by the shared primitive each needs, from the printed text in that capture:

| Card | Printed shape | New primitive it would need |
|---|---|---|
| Lightning Greaves / Swiftfoot Boots | Equipment, "Equipped creature has haste and shroud/hexproof", equip {0}/{1} | the pool has **no Equipment at all**: attach action with its timing restriction, equip-granted statics, and a shroud/hexproof targeting filter |
| Thijarian Witness | Flash; "Whenever another creature dies, if it was attacking or blocking alone, exile it and investigate" | Clue token + investigate (the Food pattern), and an alone-attacker/blocker death condition |
| Lumra, Bellow of the Woods | Reach, vigilance; P/T = lands you control; ETB mill four, return land cards from graveyard tapped | layer 7a P/T already exists (Beorn); mill + return-from-graveyard do not |
| Ruxa, Patient Professor | ETB/attack return a no-abilities creature card from graveyard; no-abilities anthem; assign damage as though unblocked | a "no abilities" predicate and unblocked-damage assignment |
| Ursine Monstrosity | Trample; combat-begin mill, random opponent, must-attack, indestructible + pump per card type | random choice, must-attack, card-type count |
| The Earth King | ETB create a 4/4 green Bear; attack trigger searches basic lands for each attacker with power ≥ 4 | a 4/4 Bear characteristic (Bear tokens are 2/2) and a basic-land search |
| Rampaging Yao Guai | X counters; ETB destroy any number of artifacts/enchantments with total mana value X | X spells and multi-target total-MV constraint |
| Spirit of the Aldergard | ETB search a snow land; +1/+0 per other snow permanent | snow supertype and a land search |
| Dúnedain Rangers | Landfall: "if you don't control a Ring-bearer, the Ring tempts you" | the Ring mechanic |
| Owlbear Cub | attack trigger: look at top eight, put a creature onto the battlefield tapped and attacking | top-N selection and put-into-play-attacking |
| Evercoat Ursine | Trample; hideaway 3 twice; play the exiled card on combat damage | hideaway / face-down exile / play-without-paying |
| Titania's Command | modal "choose two" with four modes | modal spells with two choices |

## 4. Exact-head CI

Recorded in `evidence/issue17/slice40/exact-head-ci-<sha>.json` with the capture
stdout beside it. Nothing is claimed about a commit whose check runs are not all
completed SUCCESS and keyed to that commit.

## Disclosed

* This slice is a predicate refactor plus record corrections. It implements **no
  card** and does not move any certification number.
* The feedstock capture is provenance only; the printed text still has to be
  re-verified against the printing at implementation time, and each card needs its
  own fixture record enforced by `card_printings`.
* Review 290's two non-blocking notes stay open (7b-before-7a ordering inversion,
  unreachable today; `card_printings` "colour identity" wording).
* `evidence/issue17/review299/`, `review301/` and `review306/` are reviewers'
  untracked artifacts and stay untracked.
