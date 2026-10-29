# Slice 62 checkpoint

**Product batch (product commit `a77182a`), self-check only — not independent review.** Same writer, branch
`aperture/issue-17`, draft PR46, base `f5e36dc3…`. No push/merge/closure/unlock.

## Delivered
- **Goblin Chieftain** (JMP 324, fetched 2026-10-29T08:48:15Z): complete printed
  Haste plus "Other Goblin creatures you control get +1/+1 and have haste".
  Reuses the list-filtered subtype anthem and adds a typed subtype-haste static;
  tests cover self/other/opponent/non-Goblin controls and sick attacks.
- **Searslicer Goblin** (FDN 93, fetched 2026-10-29T08:48:16Z): complete Raid
  end-step trigger with intervening-if and the real typed 1/1 red Goblin token.
  `attacked_this_turn` is declaration history, set from an actual nonempty attack,
  survives attackers leaving, resets only at the turn transition, and is rechecked
  on resolution. Empty state is omitted: old states that never gain history
  serialize unchanged. This is not broad digest continuity; a historical
  mid/post-attack save or replay run now materializes the field and redigests.
- Whole Krenko manifest gaps moved together: 48→46 `missing_cards`; interaction
  prose refreshed for the two implemented payoffs.
- Sources, printings sweeps, fixture chain, README/docs, audit and focused tests.
- AI compatibility is exactly two mechanical lines: `crates/ai/src/search.rs`
  copies the public declaration history into its reconstructed phase, and
  `crates/ai/tests/common.rs` initializes the new GameState field. No search,
  policy, ranking, model or budget behavior was redesigned.
- The Ruxa no-abilities predicate now shares effective subtype/creature queries
  and treats a Chieftain-granted vanilla Goblin token as having an ability.

## Verification
- `issue17_slice62`: 7/7; card_printings 21; catalog_pool 5; commander 43;
  slice15 2; replay_digest 6; import planned 3; analyzer 1.
- fmt, workspace all-target clippy `-D warnings`, locked workspace release: 0
  (`evidence/issue17/slice62/final-native/gates.log`, aggregate exit 0).
- Fresh slice-local audit: **168 catalog / 459 union = 155 supported + 304 missing**,
  per-list `[55, 0, 51, 46, 0, 52, 52, 53]`, Krenko **46**, **2/8 certified**.
- Four-seat/40-life package journey: 1030 steps, 25 turns, terminal P3; Chieftain
  cast 5, Searslicer cast 6, 15 Raid Goblin tokens, 15 attack declarations, 74
  combat-damage events, digest `8753aeb5…35ae`, snapshot `e7f1a198…b735`.
  Separate shipped CLI replay: **matches recorded**, exit 0.

## Non-claims
Krenko remains `planned`; this is not 3/8. Full independent review, publication,
CI and native acceptance remain outside this writer's scope.
