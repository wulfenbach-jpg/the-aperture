# Slice59 final repair checkpoint (supersedes the first checkpoint)

- Branch `aperture/issue-17`, same writer/session/card. Initial local heads
  `c6b62eb` + `cc5cd00` are superseded by `333fe76` plus the native legend
  correction committed after it. No merge, master push, closure, unlock, new team
  or nested agents. Native owner publication/CI only.
- **Native prepublication REDs (both retained):** the first Ring draft was rejected
  by `issue17_slice59_repairs` **0 passed / 5 failed, exit 101**
  (`slice59/native-initial-red.log`) with direct declaration/damage side effects,
  empty Ring state on ordinary combat, a foreign bearer and a forged candidate
  validating and sandbox-loading. After the repair, a second native test found the
  legend rule counted a whole same-name group when only the Ring-bearer was
  legendary: **5 passed / 2 failed, exit 101** (`slice59/native-legend-red.log`).
  Both are now green: native **7/7** and the focused Ring suite **10/10**.
- **Master-drift reconciliation (supervisor diagnosis accepted):** the published f359dc7
  merge ref failed to compile because master's #34 repair removed `sample_discard`. The
  branch is merged with `origin/master` 73820d7; the Ring discard now uses master's
  canonical `hand_choice_offers(..., HandChoice::Discard)` enumeration instead of a
  one-sample offer, and a new regression asserts one named offer per card in hand and that
  every offer validates. Merge commit `2f8cbd8`; product diff after it is the offer fix and
  that regression only.
- **Legend-rule correction (CR 704.5j):** only printed legendary permanents or
  their controller's level-1+ Ring-bearer form a legend group; one legend beside an
  ordinary same-name card is not a choice, the offer must be the complete group, and
  a forged truncated choice is refused by `GameState::validate` and a real
  `SessionSave::from_json` load. The commander-only refusal messages no longer claim
  the legend rule is unimplemented, and `command_zone_setup`'s reason assertion was
  updated to the real command-zone ownership reason (44/44 green on rerun).
- **The Ring package:** `CardId::TheRing` is an engine-only emblem excluded from
  catalog/by_name/import/pool count. Attack draw/discard, becomes-blocked delayed
  EndCombat sacrifice and combat-damage life loss are normal `TriggerRef`/stack
  abilities; the level-2 discard is a distinct resumable `EffectDiscard` that does
  not end combat. Level 4 triggers only on positive combat damage, so Fog prevents
  it. Empty Ring state is omitted until a temptation. Source-left Rangers use the
  captured controller and intervening-if.
- **Corpus:** fresh whole-field `AUDIT OK`: **165 catalog / 459 union = 152
  supported + 307 missing**, per-list `[55, 0, 51, 48, 1, 52, 52, 53]`, Ayula one
  name (Evercoat Ursine), 1 of 8 certified. Both Ayula gap fields were rewritten
  together.
- **Gates** (`slice59/final/gates.log`, each command's own exit): fmt 0, focused 0,
  corpus 0, workspace all-target Clippy `-D warnings` 0, locked workspace release 0.
  A whole-workspace run recorded one failure (the commander-only reason assertion
  that encoded the retired legend claim); that suite was corrected and rerun
  **44/44 exit 0** (`slice59/final/command-zone-setup-rerun.log`). No full green
  workspace total is claimed here; exact-head CI supplies that gate.
- **Final journey (release, after the legend correction):** 4 seats, 40 life, exact
  40-card package `decks/package-slice59.txt`, seed 2323, terminal P3, **2017
  decisions / 49 turns / 5090 events**, digest
  `7936133df8f6aa883cfb6f03e514896c62ae33b4d2c40b8f2e83e356620a1e50`,
  snapshot `0607cfa7…f4c0`; 9 Rangers casts, 4 Ring choices, levels `[0,1,1,2]`,
  68 combat-damage events. Non-sandbox save loads and restores the same digest; the
  separate release CLI replay reproduces 2017 actions / 5090 events and matches
  (`slice59/final/replay.log`, exit 0). It is an evidence-only masked demonstrator,
  not product AI, strength or 100-card certification, and it reaches level 2 only;
  levels 3-4 are covered by focused tests.
- **Limits:** the emblem has no physical zone object (per-seat `RingState` is the
  persisted emblem; the bearer id on a trigger is event provenance); no full
  control-change/replacement Ring interaction set; 40-card journey is not a starter
  certificate. Original #17 remains **1 of 8 certified** and unfinished.
- **Exact-head CI, recorded only after both workflows finished:** at `0e7ee70`
  `gh api commits/0e7ee70/check-runs` returns **10 runs, every `head_sha` =
  `0e7ee70b449e20e620a04f22ddbbd9d3b9104497`, 10 completed `success`, 0 cancelled,
  0 failed** — the push run `40825526651` and the `pull_request` merge-ref run
  `40825533912` (receipt
  `evidence/issue17/slice59/exact-head-ci-0e7ee70.json`). The green merge-ref run clears
  the supervisor's defect 1 by execution: that tree contains `hand_choice_offers(..., HandChoice::Discard)`
  and no `sample_discard` reference at all. No evidence commit was pushed before these
  runs completed.
- **Evidence-tip CI, observed rather than claimed:** the first evidence tip `7367d42`
  finished **10/10 completed `success`, every run keyed to `7367d42`** (push
  `40826725232`, `pull_request` merge-ref `40826731646`; receipt
  `evidence/issue17/slice59/exact-head-ci-7367d42-evidence-tip.json`). The evidence commit
  that carries this paragraph is later still and claims no CI of its own; the gate for the
  code is `0e7ee70`.
- **Correction to the earlier conservative claim — a full green local workspace total does
  exist at the final code:** the first whole-workspace run failed one stale reason assertion
  (`slice59/final/workspace-post-merge.log`: `jasmine_is_refused_in_every_ordinary_library`,
  1 failed, exit 101). After `0e7ee70` corrected that assertion, the workspace run was
  repeated: `slice59/final/workspace-final.log` was created 22:48:48 and finished 22:56:06
  local (both after the 22:45:32 commit) and reports **148 result lines / 1104 passed /
  0 failed / 2 ignored, `WORKSPACE_EXIT=0`**. No crate source changed after it; the failing
  run is retained rather than overwritten. `FINAL-RESULT.json` keeps its earlier
  "no full green workspace total claimed" wording as own-time history.
