# Native review 255 — slice 16

Verdict: APPROVED_INCREMENT at exact 6f294cfc7c82fa00ea3344c738f6ea4f96e97dc8. Full original #17 NOT ACCEPTED; return same card to implementation, not completion.

Inspected delta d2f9c9c..6f294cf: core effects/target spec/abilities/catalog, seven new behavioral tests, fixtures, package list and current corpus bookkeeping. No blocking delta defect found. Prior approved increments were not re-audited. No implementation files edited.

## Independently executed

- Focused locked core tests: issue17_slice16 7, card_printings 8, catalog_pool 5, issue13_commander_combat 43, issue17_slice15_corpus 2: 65 passed, zero failures.
- My path-dependent probe outside production/test source: 2 passed. (1) Shock kills Little Bear's chosen target in response; its trigger fizzles and gives the graveyard object no counter. (2) Fade against three qualifying permanents at one opponent seat makes exactly one Bear for that seat and destroys all three, not one Bear per permanent. Both drain until stack, trigger queue, resolving and pending are empty. Initial probe counted all Destroyed events and saw four because the engine also emits that event for the resolving sorcery's graveyard move; refined the assertion to the three qualifying permanent card IDs. This was a probe expectation correction, not an implementation repair.
- Fresh release build/run of the supplied evidence-only masked driver, copied to review255 with only its output directory changed: exact four-seat/40-life package, seed1616, terminal P2,1957 actions/44 turns/5060 events. Fade8 casts/LittleBear7 casts;8 Bear tokens;6 LittleBear counter resolutions; SolRing6/Bearscape6 destroyed. Digest ad5cd86d3432bcd69f25a1a7e4f975a51cbe399d9dbb11976f37dd32682c7084. Separate existing release CLI replay of my newly generated file matches. Explicitly a 40-card package/Jasmine injected each seat, not another certified starter or product-policy/strength evidence.
- cargo fmt --all --check and git diff --check pass. Full local workspace/clippy not repeated: exact-head CI supplies that gate.
- Fresh gh PR read: OPEN/draft/MERGEABLE/BEHIND, head exactly 6f294cf. Fresh commits/<exact SHA>/check-runs:10 completed SUCCESS, every head_sha exact (rust macos/windows/ubuntu, package, board, two workflows).
- Fresh immutable Scryfall reads with curl: all six recorded text/cost/type/P/T fields match for both printings. urllib initially returned HTTP400; curl succeeded. Source/retrieval records exist in slice16-sourced.json.
- Independent regex extraction of 131 catalog names and parsing all eight actual list files: each library99 plus named commander; union459 =119 supported +340 missing. All eight missing_cards arrays exactly match source-derived sets. Gaps: Isamaru55/Jasmine0/Talrand51/Krenko48/Ayula34/Elas52/Tatyova52/Zada53. Only Jasmine certified. Desired snapshots untouched (manifest alone changed under starter-corpus in this delta).
- Prior record corrections checked: slice14 commander injection/cast disclosure and four-record note fixed; slice11 checkpoint says6 focused tests and blocking restriction702.14c; enforcement sites use c. Generic card.rs subtype-helper comment still uses b for landwalk generally, not its blocking restriction; nonblocking, no further polishing gate.

## Original acceptance remains open

Eight desired lists exist with differing styles, but seven remain planned/unsupported and340 union names remain missing. Thus every included card/interaction is NOT implemented/conformance-covered, and exact four-seat games for all eight desired100-card snapshots are NOT proven. Current documents retain exclusions and arbitrary compatibility non-goal; the package game does not discharge these gates.

Inspected children t_26a7a4ff (scheduled certified-input dependency wait) and t_3c2fe51a (unassigned todo soak); neither is an implementation review child. Neither unlocked or modified. No merge/master push/issue closure/PR mutation, no global config/cache cleanup or retired builds. Preserve same PR46/branch/saved histories and continue sourced missing-card/list implementation. This lifecycle return is for unfinished original acceptance, NOT rejection of the approved slice16 increment.
