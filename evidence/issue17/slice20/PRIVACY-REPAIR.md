# Slice20 privacy repair

- Base observed head: `78dea97` (candidate approval superseded pending this repair).
- Scope: opponent masking only; no engine/card/UI/AI/search/config changes.
- Original RED retained at
  `evidence/issue17/slice20/native-eligibility-privacy-red.log`: two unequal hidden
  libraries reached `TopCardChoice`; the full opposing `PlayerView` differed because the
  pending description leaked `can_take`. No card identity was needed to leak eligibility.

## Fix
`crates/core/src/view.rs` filters `Pending::TopCardChoice` to its owner exactly like
`ApnapSacrifice`. The owner still sees `pending_viewed`, the correct identity and the
take/bottom offers; every opponent now receives the same neutral masked pending shape and
digest for an eligible or ineligible hidden top card.

The native independent probe was retained permanently as
`crates/core/tests/issue17_slice20_privacy.rs` (renamed only) and passes.

## Command-owned results
- `cargo test -q -p aperture-core --test issue17_slice20_privacy` -> 1 passed.
- `cargo test -q -p aperture-core --test issue17_slice20` -> 9 passed.
- `cargo test -q -p aperture-core --test projection` -> 4 passed.
- `cargo test -q -p aperture-core --test replay_digest` -> 6 passed.
- `cargo fmt --all --check` -> exit 0.
- `cargo clippy -q --workspace --all-targets -- -D warnings` -> exit 0.
- Current package journey rerun: digest
  `72cd69fb1143763fcd8fe1387b9a625c896538bdd389f33958d8705d8c7cdd48` unchanged;
  replay matched, 2064 decisions, 46 turns, take 5 / voluntary bottom 5 / mandatory
  bottom 16. The driver uses the entitled own view, so masking opponents does not alter
  play.

Product diff is confined to the owner-only pending filter plus the permanent regression.
No histories, backups, operational prompts or unrelated evidence were staged.
