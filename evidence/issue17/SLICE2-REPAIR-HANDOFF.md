# Issue 17 slice 2 — repair handoff (review findings 1, 2 adjudication, 3)

Findings 1 and 3 are fixed. Finding 2 is **rejected and corrected on the pinned
official rules text**: the reviewer compared the engine name to the Scryfall
printing label, not to the CR 111.4 generated-token name. Both created-token
names now follow CR 111.4. No commit/push. Full #17 remains incomplete.

## CR 111.4 adjudication (primary evidence)

From `evidence/issue17/slice2-rules-source.txt` lines 652–657, the pinned
official text says:

> “A spell or ability that creates a token sets both its name and its
> subtype(s). If the spell or ability doesn’t specify the name of the token, its
> name is the same as its subtype(s) plus the word ‘Token’.”

Neither Krenko’s nor Talrand’s effect specifies a token name, so the correct
in-game names are **“Goblin Token”** and **“Drake Token”**, independent of the
Scryfall token printing labels (“Goblin”, “Drake”). The source fixtures now keep
the printing label in `name` and the rules-derived name in `rules_name`, with a
note explaining the distinction. Reviewer finding 2 is therefore recorded as
incorrect (source/identity mismatch only, not a rules failure); the engine name
is **not** renamed to “Drake”.

### Genuine RED for the Goblin name

```
a_token_ceases_to_exist_instead_of_going_to_the_graveyard
  assertion failed: CR 111.4: an unnamed token is its subtype(s) plus Token
    left: "Goblin"
   right: "Goblin Token"
```

Fix: `CardId::GoblinToken` catalog name is now `"Goblin Token"`. The Drake test
asserting `"Drake Token"` passed and was retained. Type/color/P-T and
non-importability are unchanged; both remain outside `catalog::all`/`by_name`.

## Finding 1 — CLI help omitted Talrand

**Genuine RED (test first):**

```
cargo test -q -p aperture-cli selfplay_help
  selfplay_help_lists_every_supported_commander_and_the_bounded_scope FAILED
  help omits Talrand, Sky Summoner
```

Fix: `--commander` help now lists Isamaru, Jasmine Boreal, Krenko and Talrand
and states both explicit profiles are **bounded practice**, not the Commander
format (no legend rule, no full 100-card construction).

**GREEN:** `cargo test -q -p aperture-cli` -> 8 + 1 passed.

## Finding 3 — stale docs

* `docs/issue17-starter-corpus.md` roadmap item 2 now marks Talrand’s
  controller-only spell-cast trigger as a **bounded implemented package**
  (landfall/death/tribal remain gaps); item 3 marks Krenko’s activated token
  production as a **bounded implemented package** (sacrifice outlets and broader
  activated abilities remain gaps).
* `HANDOFF.md` now scopes “tokens” to token types/interactions beyond the
  bounded Goblin and Drake packages rather than claiming all tokens are absent.

Historical slice handoffs were not rewritten.

## Verification

```
cargo test -q -p aperture-core --test issue17_tokens --test issue17_talrand \
  --test card_printings --test catalog_pool
  -> 10, 6, 4, 5 passed
cargo test -q -p aperture-cli
  -> 8 + 1 passed
cargo fmt --all --check
cargo clippy -q --all-targets -- -D warnings
cargo build --release -p aperture-cli
  -> all exit 0 (slice2-repair-gates.log, slice2-repair-focused.log)
```

Replay parity after the name change (names are not serialized; digests
unchanged):

* Talrand: `c9fb97c6fc08…` matches.
* Krenko: `e2827d82da51…` matches.
* Jasmine: `d7f8c2adaa17…` matches.

## Changed files

`crates/cli/src/main.rs`, `crates/core/src/catalog.rs`,
`crates/core/tests/issue17_tokens.rs`, `crates/core/tests/issue17_talrand.rs`,
`fixtures/starter-corpus/talrand-sourced.json`,
`fixtures/starter-corpus/krenko-tokens-sourced.json`,
`docs/issue17-starter-corpus.md`, `HANDOFF.md`, plus this handoff and
`evidence/issue17/slice2-repair-{gates,focused}.log`.

No new packages, no gap-count rewriting, no #16 integration claim. The reviewer
should independently check CR 111.4 in the same preserved rules source.
