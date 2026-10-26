# Issue 17 slice 3 — cross-package final delta review

Verdict: **request_changes** for the bounded cross-package increment.
`full_issue17_accepted: false`.

Reviewed head: `0d8a1ee7dfccef39aacaff482b00255e86ff6399`
(delta from `3661353aad1c3d628827e28faf187e23ca5b4cb9`). This is not full
issue-17, not #16 integration, and not approval of the shared core hooks.

## Findings

1. **Medium — stale Krenko missing count.**
   `decks/starter-corpus/manifest.json:211` still says “remaining 52 desired-list
   cards”, and `docs/issue17-starter-corpus.md:58` says “(52 missing)”.
   Guttersnipe reduced the Krenko missing set to **51**. Update both to 51 or
   remove the hard-coded count.

2. **Medium — stale Elas/Swamp claim.**
   `decks/starter-corpus/manifest.json:352` still says the black basic land
   (Swamp) is not in the supported pool, and
   `docs/issue17-starter-corpus.md:74-75` says the pool has no black basic land
   and Elas cannot be built until Swamp is added. Swamp is now supported
   (`mdd/296`, black mana), its Elas missing-card entry is gone, and Elas has 58
   missing names (its nonland cards/interactions). The same doc’s roadmap bullet
   8 correctly says black basic support is implemented, so lines 74-75
   contradict it. Repair the manifest interaction string and the paragraph; no
   code change.

No behavioral defect was found in the five new cards, ETB draw, typed
each-opponent Guttersnipe trigger, source-left resolution, Angrath multiplier
pipeline, controller/live-seat filtering, or damage event reporting.

## Independently executed checks

| command | result |
| --- | --- |
| `git diff --name-status 3661353..HEAD` | code/tests/package/manifest/source/docs/evidence only; no bundled defaults, UI, import/search or desired-list changes |
| focused core tests (`issue17_cross_package`, `issue17_tatyova`, `card_printings`, `catalog_pool`, `starter_corpus`, `issue13_commander_combat`) with `--locked` | exit 0: 8, 5, 4, 5, 8, 43 passed |
| `cargo fmt --all -- --check`; `git diff --check`; release CLI build | all exit 0 |
| four-seat exact `decks/package-cross-corpus.txt`, seed 5, ordinary practice | exit 0; terminal P2, 1112 actions / 25 turns, 20 life, digest `4572cd11e893…`, not capped |
| replay seats vs exact package text | all four are exactly the 40-card package multiset; ordinary practice, no commanders |
| real game log | 8 Guttersnipe triggers and 8 resolutions; each resolution damages only still-live opponents (P0/P1/P3 as applicable), never controller P2 |
| separate replay | exit 0; 2819 events; digest `4572cd11e893…` matches recorded |
| source comparison to archived planned-list Scryfall objects | all five match mana cost, type, power/toughness, Oracle text and Commander legality; no fabrication found |
| read-only corpus audit | catalog 93; union 459 all names = 81 supported + 378 missing; manifest union 378 and per-list sets match; 7 planned, 1 implemented, 0 certified; desired contents unchanged |
| defaults/list diff | empty for bundled defaults and all eight desired lists |

The previously approved Tatyova journey/replay is reused unchanged; the new
source-left test suite drains the full stack and the Angrath case correctly
produces 8 trigger damage across two triggers with the controller’s multiplier.

## Scope and limitations

- Approval is limited to the bounded cross-package increment after the two
  guide/manifest truthfulness repairs. Seven desired starters remain
  planned/unsupported; the 378-name gap remains.
- Guttersnipe support is its printed trigger and damage primitive only; old-card
  LKI debt outside this effect is not claimed fixed.
- Defender adapter/shared core hooks remain a separate #16 integration hotspot,
  not approved here.
- No full suite, historical replay matrix, network/source refresh, GUI/import/
  search audit or playing-strength claim. Native exact-head CI is checked
  separately.
- The handoff lists `docs/issue15-effects.md` as changed, but no such delta is
  present; this is noted, not a support claim.
- Next focused command:
  `cargo test -q -p aperture-core --test issue17_cross_package --test issue17_tatyova --test issue17_tokens --test issue17_talrand`
