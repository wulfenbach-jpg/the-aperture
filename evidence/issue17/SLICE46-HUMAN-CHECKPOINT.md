# Slice46 checkpoint — review-321 record correction published; Thijarian Witness not published

**Original #17 UNFINISHED: 1/8 certified, Ayula 7 missing.** Same card, branch `aperture/issue-17`, same draft PR46. No merge, master push, issue closure, downstream unlock or new team.

## What this slice publishes (verified)

1. **R321-1 closed.** `decks/starter-corpus/manifest.json` `starters[ayula-bear-counters].missing_interactions` is rewritten as a whole: the stale `"modal spells that choose two of four modes (Titania's Command)"` entry is gone (slice 45 implements all six pairs and the card already left `missing_cards`), and the seven remaining entries each still name a missing interaction of a card that is still in `missing_cards`.
2. **Unrelated stale count fixed** in a file this slice already touches: `docs/issue17-starter-corpus.md` "Phased gaps" said the implemented pool is **136** cards; the same page's corpus section and the catalog both say **159**. Corrected to 159.

Verified after the change: `cargo test -p aperture-core --test starter_corpus --test catalog_pool --test issue17_slice15_corpus --locked` → 8 + 5 + 2 passed, 0 failed; `cargo fmt --all --check` exit 0. No product file changed, so no engine behavior is claimed.

## What this slice does NOT publish, and why

The next Ayula card (**Thijarian Witness**, WHO 111) was implemented locally and then **reverted before commit**: the two combat-death behavior tests I wrote against the real combat path did not pass inside this slice's window (they fail in the *test harness* plumbing — after submitting `DeclareAttackers` the pending state is not yet `DeclareBlockers` without an intervening pass, and my out-of-combat burn control asked for the offer while P0, not P1, held priority). The catalog/registration test and the Clue-token activation test did pass, which is exactly the misleading half-evidence that must not be published as a card.

The complete work-in-progress is preserved verbatim so the next slice can resume it instead of re-deriving it:

* `evidence/issue17/slice46/WIP-thijarian-witness.patch` — product diff (card.rs `Effect::CreateClueToken` / `Effect::ExileDeadCreatureAndInvestigate`, `CardId::ThijarianWitness` / `ClueToken` + token registry, catalog creature + Clue token definitions, `TriggerEvent::AnotherCreatureDies`, `CLUE_TOKEN` / `THIJARIAN_WITNESS` abilities, `TriggerRef/StackItem::dead_object` serde-skipped fields, death dispatch `queue_another_creature_dies` + `died_attacking_or_blocking_alone`, effect resolution).
* `evidence/issue17/slice46/WIP-issue17_slice46.rs.txt` — the five tests, with the two failing harness paths named above.
* `evidence/issue17/slice46/sources/` — the seven remaining Ayula cards fetched live from Scryfall this run with printing/oracle ids, retrieval UTC stamp, byte count and sha256 (`*.stamp.json`), so a later slice has authoritative printed text without a refetch: Dúnedain Rangers (LTR 159), Evercoat Ursine (BLC 30), Owlbear Cub (CLB 246), Rampaging Yao Guai (PIP 82), Ruxa, Patient Professor (C21 66), Thijarian Witness (WHO 111), Ursine Monstrosity (DSC 36).

Current audit at this head is unchanged from slice 45: catalog **159**, eight-list union **459 = 146 supported + 313 missing**, per-list `[55,0,51,48,7,52,52,53]`, **1 certified**, Ayula **7** missing names / **7** missing interactions.

## Honest authorship

This run wrote only the record correction above and the WIP card code; it wrote no passing behavior test for the new card, so no increment is claimed for it, and this run is not an independent reviewer of anything.

## Next gate

Independent review of this record-only delta (R321-1 + the 159 count) against review 321's requirement, then the same writer/history/card/PR46 resuming the preserved Thijarian Witness WIP with the two harness fixes and the remaining six Ayula names (Dúnedain Rangers, Evercoat Ursine, Owlbear Cub, Rampaging Yao Guai, Ruxa Patient Professor, Ursine Monstrosity), then exact 100-card four-seat certification with separate replays.
