# Run 234 — independent slice-11 delta review

## Verdict

**APPROVED_INCREMENT at code head c10ebfe903d0c755236f8134f503b26ae6435030. Full original issue 17 is NOT accepted.** Return the same task to implementation for the seven unfinished starters; do not merge, close issue 17, or unlock the certified release soak.

The current PR tip is 453dac36c1b87a071650a28817f58093f252406a. Its delta from the code head adds 546 evidence-only files (819925 lines added, zero deleted), with no non-evidence changes. The code head has ten completed successful check-runs. The evidence tip is NOT claimed CI-green: latest read showed two successful board checks and eight queued/in-progress jobs. Require fresh successful checks at the eventual publication/integration head.

## Independent checks actually performed

- Inspected the entire production delta from 7d6c62e through c10ebfe, changed conformance tests, source fixture, manifest and checkpoint. No repeat audit of approved slices 1–10.
- Ran `cargo test -p aperture-core --locked --test issue17_slice11 --test issue17_slice10_certification --test card_printings --test catalog_pool`: 21 passed, zero failed (6 + 3 + 7 + 5).
- Ran `cargo test -p aperture-cli --test certify --locked`: four passed, including the deliberately CRLF-encoded exact-snapshot certificate and missing/edited-file refusal controls. Total focused checks: 25 tests passed.
- Verified producer and both certification-test consumers use the shared UTF-8/CRLF-to-LF digest basis, without replacing the expected hash or weakening strict equality.
- Inspected block validation and greedy offered-block generation: both read Islands from the defending controller's battlefield. Actual engine tests reject the forbidden pair, accept the no-Island control, ignore a third player's Island, and resolve paired combat for exactly -3 versus 0 life.
- Built current `aperture-cli` release with `--locked`, then ran an independent four-seat 40-life seed-1114 game using the explicit islandwalk package in all four seats. Terminal P1; 1517 actions, 39 turns; digest 8a1e7055798f71eb45a89ee0148bd4b2f95b6262932bddddfdcb167334a90337. Separate replay verified 3830 events and the same digest. Package has 40 cards, not a desired 100-card starter; its zero-block heuristic trace is NOT keyword conformance proof. Focused tests provide that proof. Package has no white mana; no commander-cast coverage is claimed.
- Independently fetched Scryfall ice/256 and 9ed/266: names, mana costs, type lines, oracle text, power and toughness all match the fixture. Immutable IDs recorded in source-check.json.
- Fresh GitHub reads: PR46 OPEN, draft, MERGEABLE/BEHIND; code head's ten check-runs all completed SUCCESS, including both Windows Rust jobs. Current evidence-tip jobs remain pending, not waived.
- Fresh manifest/deck/catalog audit: eight 99-card libraries plus declared commanders; every per-list missing-card set matches the actual catalog. 116 catalog cards; union 459 = 104 supported + 355 missing; one certified starter, seven planned.
- `cargo fmt --all --check` and `git diff --check` pass. Tracked content diff is empty. No implementation edits, new reviewer process, merge, master push, issue closure or successor activation.
- Inspected child t_3c2fe51a: unassigned TODO release soak; not activated.

## Nonblocking notes

- The checkpoint's section 4 still says five slice-11 tests, while the final suite has six. The actual executed count above is authoritative; do not repeat the old count.
- The pinned rules put the detailed landwalk blocking restriction in CR 702.14c; 702.14b says landwalk is an evasion ability. Production behavior matches 702.14c, but new comments/messages repeatedly cite only 702.14b. Correct those citations opportunistically, without another behavioral audit.

## Original acceptance remains open

Eight supported Commander lists: unmet (one bounded certified starter, seven planned).
Every included card/interaction implemented and covered: unmet for the desired union (355 missing names).
Exact four-seat snapshots: exercised for the existing starter through executable certify tests and for this package through the independently rebuilt CLI; not all eight starters.
Exclusions/non-goals: documented, with package and bounded-practice limits preserved. No arbitrary Archidekt or playing-strength acceptance.

Next implementation must preserve this approved delta, same branch/PR and existing saved writer/reviewer histories; continue sourced missing-card packages or a genuinely completable second starter. A request-changes transition here routes unfinished original scope, not a rejection of the approved slice-11 repair.
