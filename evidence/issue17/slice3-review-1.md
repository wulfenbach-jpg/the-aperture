# Issue 17 slice 3 — independent delta review

Verdict: **approve_increment** for this bounded partial increment.
`full_issue17_accepted: false`. This is not full issue-17 certification, not #16
integration, and not approval of arbitrary Commander.

Reviewed head: `3661353aad1c3d628827e28faf187e23ca5b4cb9`
(delta from approved `e16185c81c66e11e5bef686f03c2a3977a791301`).

## Findings

No blocking or non-blocking correctness defect found in the reviewed delta.
Tatyova/Jaddi source text, controller-only landfall, life-then-draw order,
source-left resolution, no-premature-priority, defender offer/refusal behavior,
canonical catalog consistency, commander-only refusal, unchanged defaults and
manifest/count truthfulness all checked out.

## Verified behavior

- `triggered ability from Tatyova` is queued only for the land player's
  permanents and staged before priority. The real game log shows 20 Tatyova and
  60 Jaddi landfall triggers; a Tatyova resolution logs `gains 1 life`, then
  `draws a card`, then ability resolution.
- Focused tests cover the source-left case (Tatyova returned to hand by
  Unsummon after the trigger is queued; trigger still gains life and draws,
  with no Priority event between life, draw and `AbilityResolved`) and the
  forged Jaddi attack refusal with unchanged canonical digest.
- Jaddi is excluded from `legal_attackers`, direct attack declaration is
  refused under CR 702.3b, and the heuristic filter prevents the AI from
  submitting the forbidden attack. The real seed-21 game contains zero Jaddi
  attack lines.
- `catalog::all` and `catalog::def` agree on Jaddi's `defender`; the single
  `card_defender!` macro entry is used consistently.
- Source fixture `tatyova-sourced.json` matches the archived Scryfall response
  for Tatyova (`fdn/247`) and Jaddi (`tdc/260`): cost, type line, body, Oracle
  text and Commander legality. No fabricated metadata observed.

## Commands and results

| command | result |
| --- | --- |
| focused core tests (`issue17_tatyova`, `card_printings`, `catalog_pool`, `starter_corpus`, `issue13_commander_combat`) with `--locked` | exit 0: 5, 4, 5, 8, 43 passed |
| focused import `starter_corpus_planned` | exit 0: 1/1 |
| `cargo test --locked -p aperture-cli` | exit 0: lib 7/7 including four-commander help assertion; commander_check 8/8; explicit_decks 1/1 |
| `cargo fmt --all -- --check`; `git diff --check`; release build | all exit 0 |
| four-seat exact `decks/package-tatyova-landfall.txt`, seed 21, Tatyova commander each seat | terminal P0, 2993 actions / 65 turns, 40 life, digest `b31adfd97c31…`, not capped |
| replay seats vs exact package text | all four are exactly the 40-card package multiset; all four commanders Tatyova |
| separate replay | exit 0; digest `b31adfd97c31…` matches recorded, 7549 events |
| read-only corpus audit | catalog 88; union 459 all names = 76 supported + 383 missing; manifest union 383 and per-list sets match; Tatyova planned with 56 missing; one implemented, zero certified |
| defaults/desired-list diff | empty for bundled decks, all eight desired lists and the Jasmine list |

## Scope, retained limitations and next command

- Only the bounded Tatyova/Jaddi package is approved. Seven target starters
  remain planned/unsupported and the 383-name corpus gap remains.
- Landfall is limited to land drops by Tatyova/Jaddi; other landfall payoffs,
  extra land drops, broader ramp, other keyword/counter/token packages and
  general defender semantics are not claimed.
- Core hooks are a separate #16 integration hotspot; no #16 integration,
  release, merge or GUI/playing-strength approval.
- No full suite, historical replay matrix, network refetch or import/search
  audit was run. Native exact-head CI is checked separately.
- Next focused command:
  `cargo test -q -p aperture-core --test issue17_tatyova --test issue17_tokens --test issue17_talrand`
