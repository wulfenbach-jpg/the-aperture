# Issue 17 slice 8 — review round 2 adjudication

**review_outcome: APPROVED_INCREMENT**  
Exact head: `c22b240be304802ed31a7f7f9acb8d656d6218bf`  
Base: `818afdb8afbb5977b6e08b085654304e4b5754bf`  
`full_issue17_accepted: false`.

Both prior findings are fixed:

- **Stale counts:** docs now say Isamaru 55, 99 supported / 360 missing, Krenko
  49 / Talrand 51; manifest free-text now says Talrand 51, Krenko 49, Elas 55.
  Verified catalog 111, union 459, supported 99, missing 360, manifest arrays
  matching; per-list 55/0/51/49/46/55/54/55; desired files unchanged.
- **Unblocked control:** new
  `razorclaw_unblocked_does_not_trigger_or_pump` attacks unblocked and asserts
  power stays 3 through combat damage/end combat. A spurious BecomesBlocked
  trigger would pump to 5 and fail. Slice8 focus now 9/9.

The delta `818afdb..HEAD` contains only the test, manifest free-text, docs and
handoff. No production source/catalog/fixture/package behavior changed.
`cargo fmt --all -- --check` and `git diff --check` pass.

Reused review-1 evidence: 101 focused tests; seed-801 four-seat package journey
(terminal P0, 2377 actions/68 turns, digest `ffbeaee0db38…`, 6087 events, four
exact 40-card snapshots, replay matched); live source check for all eight
records; exclusions for Brightstone Ritual, Friendly Teddy and Aetherize remain
manifest-missing. The handoff now cites retained actual pre-scaffold RED output
and no longer claims retrospective strict-TDD.

**Remaining gaps:** full issue-17 remains unfinished at **1 implemented / 0
certified** of eight lists and 360 missing union names. The three excluded cards
still need bounded mana, serialized multi-player draw continuation/APNAP and
attacker-list commander-replacement continuation. Goblin War Strike is
player-target only because no planeswalker permanents exist. No #16/product
acceptance, release/merge, GUI/import/search or playing-strength claim; this pass
is continuation only.
