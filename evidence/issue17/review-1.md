# Issue 17 slice 1 — independent practical review

Verdict: **request_changes** (increment only). `full_issue17_accepted: false`.

Reviewed commit: `0a7d781c41d1799a79c8ef41e6888d81ab7f9e6d`, one commit over base
`8a72cc607ea25f203be672c0007ad5e798be7145`. Tracked tree was clean; untracked
scope was `.starter-run.yml`, writer evidence/run logs, and this review's
`review-1.*` / `review1-*` outputs. (Windows is case-insensitive, so writing
`review-1.md` replaced the uppercase assignment copy; it is preserved verbatim
at `evidence/issue17/review1-assignment-original.md`.)

The implemented Jasmine package is functionally sound. The exact 99-card library
plus commander loads unchanged into four seats, plays a real terminal 40-life
game, and replays/saves correctly. The analyzer remains legal/supported with
`gameplay.available=false`, and the seven other lists remain honestly planned.
Two concrete issues require a small correction before this increment is approved:

## Blocking findings

### 1. Default CLI deck identity regressed

`crates/cli/src/decks.rs:23-33` now derives the list name from `file_stem()` for
*every* existing path. That includes the non-explicit bundled defaults, so an
unchanged default invocation no longer produces the prior human names.

Reproduction:

```text
./target/release/aperture-cli.exe selfplay --seed 1 --max-steps 50 \
  --out evidence/issue17/review1-default-names.json
```

Observed seats: `temur-beats` / `izzet-control`.
Base source used `DeckList::parse_practice(fallback_name, &text)` on the same
branch, producing `Temur Beats` / `Izzet Control`. This changes the
machine-readable `aperture-selfplay/2` seat identity for the default command,
contrary to the preserve-defaults requirement. Deck content, rules and digest
behavior are unchanged.

Fix: use the stem only when the deck path was explicitly supplied (or only for
non-bundled paths), keeping `fallback_name` for default loads. Add a regression
covering both default and explicit starter names.

### 2. Tracked corpus docs contradict the manifest they describe

`docs/issue17-starter-corpus.md:10` says planned entries have `deck_path: null`;
`:116-118` says no exact future list is claimed and a `deck_path` exists only
when a supported 99-card list can be built. The actual manifest has non-null
`deck_path` for all seven planned entries under
`decks/starter-corpus/planned/`, and `crates/import/tests/starter_corpus_planned.rs`
requires and reads those exact lists. The same stale claim is repeated at
tracked `evidence/issue17/IMPLEMENTATION-HANDOFF.md:103`.

This is not a request to implement the other seven. It is a request to make the
tracked schema description match the honest planned-list state: planned means
exact desired list present but engine-unsupported/not certified, not absent.

## What was verified

| area | command / method | result |
| --- | --- | --- |
| focused core | `cargo test --locked -p aperture-core --test starter_corpus --test card_printings --test issue13_commander_combat` | exit 0; 8/8, 3/3, 43/43 |
| focused import | `cargo test --locked -p aperture-import --test starter_corpus_analyzer --test starter_corpus_planned` | exit 0; 3/3, 1/1 |
| focused CLI | `cargo test --locked -p aperture-cli` | exit 0; lib 5/5, commander_check 8/8, explicit_decks 1/1 |
| formatting | `cargo fmt --all -- --check`; `git diff --check` | both exit 0 |
| release build/help | `cargo build --release --locked -p aperture-cli`; `selfplay --help` | flags and both commanders documented |
| exact 4-seat game | four `--deck decks/starter-corpus/jasmine-vanilla-stompy.txt`, `--commander-combat-practice`, four `--commander N:Jasmine Boreal` | exit 0; terminal P1 win, 2364 actions/53 turns, starting life 40, digest `d7f8c2adaa17…`, not capped |
| embedded libraries | replay `seats[].cards` mapped through catalog variant names vs exact text list | all four are exactly the 99-card multiset; all four commanders Jasmine; no unknown variants |
| actual casts | parse verbose log; classify against sourced fixture | 55 casts; 39 new-vanilla casts of 30 distinct cards; 5 Jasmine casts; rest pre-existing supported cards |
| replay | `replay --file review1-selfplay.replay.json` | exit 0; digest matches recorded, 6061 events |
| save/replay | `replay --save-out review1-save.json` then replay the save | both exit 0; stored/recomputed digest match; `pending None` |
| analyzer, implemented | `commander-check` on exact fixture pair | exit 0; format legal, engine supported, 1/99/100, 61 canonical names, unsupported empty, `gameplay.available=false` |
| analyzer, planned | `commander-check` on planned zada pair | exit 0; format legal, engine unsupported (58), `gameplay.available=false`; not promoted |
| manifest honesty | parse all eight lists; compare declared missing vs catalog; compute union | declared == computed for all 8; union 447 = 61 supported + 386 missing; planned 99/57/42, Jasmine 99/58/41 |
| source binding | hashes, raw printing cross-check, live Scryfall spot checks | `cards_sha256` and `deck_text_sha256` match; 61 sourced records match saved raw printings; Jasmine `tsb/93`, Armored Warhorse `m12/7`, Norwood Ranger `9ed/260` match live API |
| ordinary-library refusal | explicit text file containing `1 Jasmine Boreal` | exit 1, commander-only/legend-rule refusal (also engine-tested) |
| no substitution | explicit missing `--deck` path | exit 1, refuses to substitute the bundle |

Exact Jasmine facts confirmed: `{3}{G}{W}` Legendary Creature — Human, 4/5,
empty Oracle text, Commander-legal, G/W identity. 47 sourced vanilla creatures
all print no rules text and cast/resolve through the existing permanent path.
41 basics (20 Plains, 21 Forest) + 58 singleton nonlands = 99 library, plus
commander = 100.

## Limitations

* Full issue 17 is **not** accepted: seven target styles remain planned; only
  Jasmine is implemented. This review does not approve eight-style completion,
  merge, or closure.
* Full suite and historical matrix were not rerun; writer logs were reused as
  instructed, with focused tests rerun above.
* The issue-14 legacy v6 replay mismatch was not independently re-executed;
  `evidence/issue17/legacy-replay-note.md` records the same mismatch at untouched
  base, so it is treated as inherited evidence staleness.
* Native UI is untouched and still hardcodes Isamaru for its bounded profile;
  Jasmine is CLI-exercised. No UI hidden-zone rendering audit was performed.
* `fixtures/starter-corpus/manifest.json.deck_sha256` binds `raw_deck` bytes
  (`853a49d4…`), not the deck JSON file bytes (`677f24ee…`). Tests and deck
  provenance define that binding, but the field lacks an explicit domain note.
  Not counted as a failure.
* Starter tests pin 99 total and all 47 sourced cards but range-check nonland
  counts (54..=59) and basics (40..=45); exact 58/41 was verified separately.
* No speculative full-engine audit or refetch of all 61 printings.

The core inventory shows no card-metadata, legality, commander-registry,
ordinary-library, replay, or support-promotion defect in this increment beyond
the two findings above.
