# Issue 17 slice 3 — repair-2 adjudication

Verdict: **approve_increment** for the bounded cross-package increment.
`full_issue17_accepted: false`; #16 integration is **not** approved.

Reviewed head: `9cecd62ddb56d44e6853f9f9098fd3a473635b66`
(delta from `0d8a1ee7dfccef39aacaff482b00255e86ff6399`).

## Prior findings

1. **Krenko count — fixed.** `manifest.json:211` now says 51 remaining;
   `docs/issue17-starter-corpus.md:58` says `(51 missing)`. Verified Krenko
   missing is 51.
2. **Elas/Swamp — fixed.** Manifest Elas no longer claims Swamp is unsupported;
   it names 58 unsupported desired-list nonland names plus sacrifice,
   death/drain and tutor/reanimation gaps. Swamp is absent from Elas
   `missing_cards`. The docs paragraph now says the Swamp requirement is met and
   describes the real remaining gaps instead of “no black basic land.”
3. **False changed-file mention — fixed.** The cross-package handoff no longer
   lists `docs/issue15-effects.md` in “Changed” and explicitly records the
   removal.

No remaining blocker.

## Checks

- `git diff --name-status 0d8a1ee..HEAD`: only `decks/starter-corpus/manifest.json`,
  `docs/issue17-starter-corpus.md`, and the cross-package handoff. No code or
  test changes.
- `git diff --check`: exit 0.
- `python evidence/issue17/slice3-native-corpus.py`: catalog 93, union 459,
  supported 81, missing 378, manifest matches; Krenko 51, Elas 58;
  `desired_contents_unchanged: true`; one implemented, seven planned.
- Manifest read: Krenko 51 and Elas 58 missing; Swamp absent from Elas missing;
  interaction text is consistent with those gaps.

Prior independently run checks and the seed-5 exact cross-package journey/replay
remain valid because this repair changes no engine or tests.

## Retained limitations

- Only the bounded cross-package increment is approved. Seven desired starters
  remain planned/unsupported and the 378-name gap remains.
- Shared core hooks are a separate #16 hotspot, not approved here.
- Guttersnipe support is bounded to its printed trigger/damage primitive; old-card
  LKI debt outside that effect is not claimed fixed.
- No full suite, historical matrix, network refresh, GUI/import/search audit or
  playing-strength claim. Native exact-head CI is checked separately.
