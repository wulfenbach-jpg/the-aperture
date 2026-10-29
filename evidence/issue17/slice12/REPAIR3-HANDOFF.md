# Slice 12 repair 3 — strict inert-archive references and UI sentinel

Repair head is the commit containing this file (reported in the final message).
No merge/master push. Full #17 unfinished.

## R12-R2-F1 — archived token validator was too permissive

Genuine RED: `archived_tokens_cannot_anchor_live_engine_references` loaded a
departed owner’s inert `Zone::Removed` token and added it to `trigger_queue`;
`validate()` returned `Ok(())`. The same held for `revealed`.

Fix: a typed `GameState::object_is_referenced(id)` predicate now covers actual
`ObjectId` references: trigger queue sources, stack object and targets,
`resolving`, all `Pending` variants (OrderCard viewed/to-top, APNAP choices and
source-left provenance, commander return/replacement/draw, trigger target/legal/
queued, trigger choice), combat attackers/blocked/blockers, continuous effect
source/target, damage shield source/target, `activation_uses`, `revealed`,
commander registry objects, and player zone lists. No string/JSON scanning.

An archived token is admitted only when its owner is departed **and**
`object_is_referenced` is false. Live-owner Removed tokens and any token in a
real off-battlefield zone remain refused. Ordinary token BF→GY-cessation and
source-left triggers whose source object no longer exists are unchanged.

Focused negative controls now cover trigger_queue, revealed, activation_uses,
stack object, stack target, pending trigger choice, resolving, continuous effect
target, and shield target. All are RED before the predicate and GREEN after.

Save compatibility unchanged:

```
slice12/save.json               -> 328dadbdbdc6, 7051 events
slice12-review-demo-save.json   -> d3eade58f133, 6981 events
fresh --save-out demo replay    -> d3eade58f133, 6981 events
old slice8 replay               -> ffbeaee0db38, 6087 events
```

## CI sentinel — remaining UI test fixtures

Test-only fix: `crates/ui/src/app.rs` inline viewport test and
`crates/ui/tests/session.rs` still used now-supported `Sol Ring` as their
unsupported sentinel, making `!certified()`/unsupported assertions wrong.
Replaced with genuinely unsupported `Darksteel Ingot`, preserving every
Unsupported/NotCertified/refusal assertion. No production UI/import behavior
change. Import `start_flow` remains 21/21 with the same sentinel and its
absent-catalog guard.

## Verification (actual)

```
issue17_slice12            11 passed
issue17_tokens             10 passed
issue17_slice6             13 passed
cargo test -p aperture-ui --lib app::viewport_tests::the_import_panel_reports_both_axes_and_every_unsupported_card_before_start
                          1 passed
cargo test -p aperture-ui --test session
                          22 passed
cargo fmt --all --check   exit 0
cargo clippy --all-targets -D warnings  exit 0
cargo build --release -p aperture-cli   exit 0
```

## Scope

Archived Removed tokens remain internal leave-game storage, not a real Magic
zone, and cannot anchor triggers, visibility, pending choices, stack/effects,
activation history, combat or zone lists. No gameplay/replay algorithm changes,
no version bump, no weaker live/off-battlefield token rejection.
