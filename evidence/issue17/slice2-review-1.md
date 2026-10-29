# Issue 17 slice 2 — independent delta review

Verdict: **request_changes** for this bounded increment. `full_issue17_accepted:
false`. This is not review or approval of all eight target lists, #16
integration, arbitrary Commander, UI behavior or playing strength.

Reviewed head: `095362b7eebbc58f57116db3f7899bd58e3b08be`
(one commit over approved `86db7d316edd6200f7f598849018e428d11b69eb`).

## Findings

1. **Medium — CLI supported-commander help omits Talrand.**
   `crates/cli/src/main.rs:70` still advertises only Isamaru, Jasmine Boreal and
   Krenko. `SUPPORTED_COMMANDERS` contains Talrand and the Talrand package runs.
   Repro: `aperture-cli selfplay --help`; expected help lists Talrand, Sky
   Summoner. Runtime behavior is correct, but the user-facing support list is
   wrong.

2. **Low — Drake token name does not match its source record.**
   `crates/core/src/catalog.rs:555` names the engine token `Drake Token`, while
   `fixtures/starter-corpus/talrand-sourced.json:29` records the printed token
   name `Drake`. Event output says “Drake Token enters the battlefield”. The
   type line, 2/2 body, blue color and flying are correct, and the token remains
   excluded from `catalog::all`/`by_name` and deck import. This is a
   source/identity mismatch, not a rules failure.

3. **Low — remaining docs still describe implemented mechanics as absent.**
   `docs/issue17-starter-corpus.md:57-59` leaves Talrand spell-cast triggers and
   Krenko non-mana activated token generation in the pending roadmap without
   marking them partially implemented. `HANDOFF.md:306` still says tokens are
   not implemented. The manifest and README do describe the bounded packages
   correctly.

No behavioral defect was found in the reviewed focus areas: Krenko’s >255 X,
activation restrictions, token identity/controller/count/nonimportability,
real battlefield→graveyard/hand/exile records before CR 111.7 cessation,
Swords last-known-power gain, Talrand’s controller-only cast trigger above the
spell and after a counter, flying block legality, or the absence of priority
while resolving.

## Independently executed checks

| check | result |
| --- | --- |
| `git diff --name-status 86db7d3..HEAD` | expected code/docs/tests/package/manifest/source delta; no `decks/starter-corpus/planned/*.txt` content change |
| focused core tests (`issue17_tokens`, `issue17_talrand`, `catalog_pool`, `card_printings`, `issue13_commander_combat`, `starter_corpus`) with `--locked` | exit 0: 10, 6, 5, 4, 43, 8 passed |
| focused import tests with `--locked` | exit 0: planned 1/1, analyzer 3/3 |
| `cargo fmt --all -- --check`; `git diff --check`; release CLI build | all exit 0 |
| four-seat exact `decks/package-krenko-tokens.txt` game, seed 5, Krenko commander each seat | exit 0; terminal P0 win, 1528 steps/34 turns, 40 life, digest `69487135f53f…`, not capped |
| replay seats vs exact package text | all four are exactly the 40-card package multiset; all four commanders Krenko; no unknown ids |
| replay, `--save-out`, replay saved game | all exit 0; digest `69487135f53f…` matches, 4910 events, pending none |
| real Krenko log | token ETB creation under correct controllers, Krenko activations, token cessation; a 14-creature death block is recorded before cessation |
| four-seat exact `decks/package-talrand-drakes.txt` game, seed 5, Talrand commander each seat | exit 0; terminal P3 win, 2861 steps/66 turns, 40 life, digest `cd6e07d238f7…`, 50 casts, 9 counters, 2 blocks |
| real Talrand log | Talrand triggers are logged above their original spells and create Drakes under the controller’s control; Drakes attack; counters resolve normally |
| old Jasmine replay | exit 0; digest `d7f8c2adaa17…` still matches, 6061 events |
| manifest/pool audit | pool 86; 447 union names = 66 supported + 381 missing; Krenko/Talrand remain planned with 52/54 missing; package decks are 40 cards, not the desired starters |
| `selfplay --help` | confirms Talrand omission finding |

## Scope and limitations

* Original eight desired lists were not changed; seven remain planned and
  unsupported. The Krenko and Talrand package fixtures are explicitly 40-card
  package exercises, not substitutes for the 100-card desired starters.
* No full suite, full historical matrix, network refetch, UI review, or #16
  integration test was performed. Exact-head CI is checked separately.
* The review does not retroactively judge assignment-1 TDD history; the retained
  RED/GREEN disclosures and repair evidence were read for context.
* No playing-strength or full Commander claim.
