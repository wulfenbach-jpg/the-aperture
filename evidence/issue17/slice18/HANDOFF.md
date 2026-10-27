# Slice 18 handoff — Ayula commander (post-inspection repair)

- Branch: `aperture/issue-17`; original implementation `9c3a594`; repair commit is the branch head recorded in the final receipt (parent `9c3a594`); base `3934d76`.
- **PARTIAL #17**, `full_issue17_accepted=false`, eight-list gate open.
- No push, merge, master push, issue closure, reviewer launch, or certification.

## Delivered
- Sourced **Ayula, Queen Among Bears** `{1}{G}` Legendary Bear 2/2, exact oracle text
  from `fixtures/starter-corpus/slice18-sourced.json` (retrieved
  `2026-10-26T23:40:57Z`).
- Registered in the canonical commander list, catalog, command-zone setup and mana/cast
  path; ordinary-library refusal follows the existing supported-commander policy.
- Trigger condition: another Bear controlled by Ayula's controller. Token entries trigger;
  Ayula entering does not; opposing Bear entries do not.
- Modal choice before targets. Counters mode targets **any Bear**, including an opponent's;
  fight mode requires a Bear you control and a creature you don't control. Modes with no
  legal full target set are not offered.
- Fight revalidates both printed relations at resolution and deals both damage instances
  simultaneously through the existing replacement/prevention pipeline. If a target left,
  no partial fight occurs; the target-departure case is tested with a real Unsummon.
- Counts: **135 catalog / 123 supported / 336 missing**; Ayula 30 missing.
  Manifest and pinned assertions updated.

## Actual commands/results
- `cargo test -q -p aperture-core --test command_zone_setup` -> 41 passed
  (7 Ayula tests including source-left and prevention).
- `cargo test -q -p aperture-core --test abilities` -> 13 passed.
- `cargo test -q -p aperture-core --test rules_conformance` -> 5 passed.
- `cargo test -q -p aperture-core --test issue17_slice17` -> 6 passed.
- `cargo test -q -p aperture-core --lib` -> 13 passed.
- `cargo test -q -p aperture-core --test card_printings --test catalog_pool` -> 8 + 5 passed.
- `cargo test -q -p aperture-core --test issue17_slice15_corpus --test issue13_commander_combat`
  -> 2 + 43 passed.
- `cargo fmt --all --check` -> exit 0.
- `cargo clippy -q --workspace --all-targets -- -D warnings` -> exit 0.
- `cargo build -q --release -p aperture-cli` -> exit 0.

## Journey (executable, evidence-only, NOT product AI)
- `decks/package-slice18.txt`, 4 seats, exact snapshots, 40 life, Ayula injected into each
  command zone by the demonstrator; seed 1616.
- Repaired-binary journey: terminal win P2; 1961 decisions, 43 turns; digest
  `09e6bb25c505c1c090759e379fc442626c9ac04bc5dffbbe6548b5f8dcc3ba24`.
- Replay digest matches; fresh `demo-save.json` round-trips exactly.
- Ayula cast 3 times; counters mode chosen 5, fight mode 4; 95 noncombat damage events.
- Label: 40-card package, **not** a desired 100-card starter certificate.

## Prepublication repairs (inspection at 9c3a594)
- **BLOCKING optional-trigger regression fixed.** `apply_trigger_choice` now performs one
  `match` on `pending.take()`: `TriggerMode`, `TriggerChoice`, or untouched. RED:
  `cargo test -p aperture-core --test abilities` at 9c3a594 failed
  `optional_trigger_is_declined_or_used_by_its_controller` at abilities.rs:391
  ("declining is a typed event"), 12 passed / 1 failed. GREEN: that test now passes,
  including the accept path's actual draw.
- **Queued triggers drained.** Every no-mode/target path now calls
  `stage_triggers(queued)`. Added engine regression with Ayula plus Soul's Attendant in
  one batch. Scratch-revert RED: `... --lib ayula_removed_trigger...` failed
  "the second trigger must still be placed"; GREEN now passes.
- **Rules citations corrected.** Fight is 701.14a; illegality/no-fight is 701.14b; modal
  triggered choice cites 603.3c + 700.2b.
- **Archived-reference traversal extended** to `TriggerTarget.first` and
  `TriggerMode.queued`. A dedicated forged archived-state refusal test is still missing
  (disclosed).
- **Added coverage:** source-left resolution after Ayula leaves; fight prevention with
  Mending Hands (Ayula's damage prevented while the opposing Grizzly still dies);
  fight target departure; unavailable fight mode; token/opponent trigger distinctions.

## RED retained
- abilities optional-trigger failure above (13 passed after repair).
- Scratch-revert queued-trigger failure above; source restored, test GREEN.
- No compile-error RED is claimed.

## Defects / deviations / unfinished
- Modal choice reuses `AnswerTriggerChoice` boolean (true counters, false fight), with
  mode-specific labels/legal offers, instead of adding a new action enum. This avoided
  unrelated UI action-surface changes but is less typed than ideal.
- Production modal/fight code preceded the new tests; no retroactive TDD claim.
- Still missing: Ayula pending-choice save/load+replay while suspended, two-target
  half-answered save/replay, and forged pending-action refusal tests. Source-left,
  prevention, target departure and unavailable-mode tests are now present.
- No full workspace suite rerun after slice18; focused suites, workspace clippy and release
  only.
- Ayula's desired 100-card list still has 30 unsupported names; no certificate or 100-card
  snapshot.
- Parent owns publication/review/CI; no push performed.
