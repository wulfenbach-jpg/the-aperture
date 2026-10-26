# Slice 4 review round 2 — adjudicate R211-1 only

Same read-only history. New head **eea8f9dea34ae9b5f65c4bda3a1f50797449a118** on
draft PR46 (previous reviewed head `86aebf9`).

Your finding R211-1 is accepted. Adjudicate exactly three things and nothing
else:

1. **R211-1 repaired?** `crates/core/src/engine.rs` now has one shared
   `queue_another_creature_enters(entering)` used by both `enter_battlefield`
   (creatures) and `create_tokens` (creature tokens), excluding the entering
   object. Re-run your own probe deck
   `evidence/issue17/slice4/slice4-probe-warden.txt` and confirm the Goblin token
   now gains life for each Soul Warden; confirm a *non-creature* permanent still
   does not, and that Soul Warden still never triggers for itself.
2. **New regressions genuine?**
   `soul_warden_triggers_for_creature_tokens_too` (RED evidence recorded with the
   token hook disabled: `slice4/slice4-red-R211-1-token.log`),
   `trample_carries_surplus_across_two_blockers`,
   `trample_counts_damage_already_marked_on_the_blocker`. Check the assertions
   would fail if the behavior regressed — not just that they pass.
3. **No collateral change?** Re-run `cargo test --workspace --locked`,
   `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
   the first journey (`package-ayula-slice4`, seed 17 — digest must still be
   `5e905fdc0399…`) and the new second journey
   (`decks/package-soul-warden-tokens.txt`, seed 23). Confirm the four sourced
   card strings/flags and the corpus numbers (97 catalog / 459 union / 85
   supported / 374 missing, desired lists unedited) are unchanged.

Write `evidence/issue17/slice4/slice4-review-2.json` and `slice4-review-2.md`
with `review_outcome` = `approve_increment` only if R211-1 is genuinely fixed,
otherwise `request_changes` with the concrete remaining defect. Keep it under 40
lines. Read-only: no edits, no commit, no merge, no closure. A bounded increment
approval still does not certify the eight desired lists or accept #17.
