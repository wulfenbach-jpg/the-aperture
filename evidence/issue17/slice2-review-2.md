# Issue 17 slice 2 — repair-delta re-review

Verdict: **approve_increment** for this bounded partial increment.
`full_issue17_accepted: false`. This does not accept all eight target lists,
#16 integration, release, merge, UI behavior or arbitrary Commander.

Reviewed head: `e16185c81c66e11e5bef686f03c2a3977a791301`
(repair delta over prior reviewed `095362b7eebbc58f57116db3f7899bd58e3b08be`).

## Initial finding dispositions

1. **CLI help omitted Talrand — corrected.**
   `aperture-cli selfplay --help` now lists Isamaru, Jasmine Boreal, Krenko and
   Talrand, and states both explicit profiles are bounded practice, not the
   Commander format. The new
   `selfplay_help_lists_every_supported_commander_and_the_bounded_scope` test
   passes.

2. **Drake token name — withdrawn and corrected on primary rules evidence.**
   I independently read the pinned official text at
   `evidence/issue17/slice2-rules-source.txt` lines 652–657:
   > “A spell or ability that creates a token sets both its name and its
   > subtype(s). If the spell or ability doesn’t specify the name of the token,
   > its name is the same as its subtype(s) plus the word ‘Token.’”
   The same `MagicCompRules 20260925` source is pinned in
   `docs/rules-reference.md`. Neither Krenko’s nor Talrand’s effect specifies a
   name, so `Goblin Token` and `Drake Token` are the correct generated-token
   names; Scryfall’s `Goblin`/`Drake` labels are printing labels. My prior
   finding compared the engine name to the printing label and was wrong.
   `catalog.rs` now uses both CR 111.4 names, and the source fixtures retain
   `name` plus `rules_name`/`name_note`. Both tokens remain nonimportable and
   outside `catalog::all`/`by_name`.

3. **Stale docs — corrected.**
   `docs/issue17-starter-corpus.md` now marks Talrand’s controller-only
   cast trigger and Krenko’s `{T}` token production as bounded implemented
   packages, with landfall/death/tribal triggers and sacrifice outlets still
   gaps. `HANDOFF.md` now scopes missing tokens to types/interactions beyond the
   bounded Goblin and Drake packages.

No remaining blocker was found.

## Checks executed

| command | result |
| --- | --- |
| `git diff --name-status 095362b..HEAD` | help, token names/tests, docs/HANDOFF, source metadata and repair/review evidence only; no engine/state/rules-revision/manifest/list-content change |
| independent CR 111.4 read | pinned source confirms subtype(s) + “Token” for unnamed tokens |
| `aperture-cli selfplay --help` | all four supported commanders and bounded scope present |
| `cargo test --locked -p aperture-cli` | exit 0: lib 7/7 (new help test), commander_check 8/8, explicit_decks 1/1 |
| `cargo test --locked -p aperture-core --test issue17_tokens --test issue17_talrand --test catalog_pool --test card_printings` | exit 0: 10, 6, 5, 4 passed |
| `cargo fmt --all -- --check`; `git diff --check`; release build | all exit 0 |
| existing Krenko replay | exit 0; 1805 actions, 6039 events, digest `e2827d82da51…` matches |
| existing Talrand replay | exit 0; 1830 actions, 4740 events, digest `c9fb97c6fc08…` matches |
| desired-list diff | empty for `decks/starter-corpus/planned/`, Jasmine list and manifest |

The earlier independently run four-seat Krenko and Talrand package games,
save/replay roundtrips, old Jasmine replay compatibility check, and manifest
support-gap audit remain valid because this delta changes no engine/state/list
content. The seven incomplete starters remain planned/not certified, with the
previous 381-name corpus gap.

## Scope and limitations

* Bounded slice-2 Krenko/Talrand packages only; original eight-list issue #17,
  #16 integration, release/merge and arbitrary Commander remain open.
* No full suite, full historical matrix, network refetch, UI review or
  playing-strength claim.
* Native exact-head CI is checked separately by the coordinator.
