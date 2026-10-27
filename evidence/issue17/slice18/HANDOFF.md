# Slice 18 handoff — Ayula commander

- Branch: `aperture/issue-17`; implementation commit `9c3a594`; parent `3934d76`.
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
- `cargo test -q -p aperture-core --test command_zone_setup` -> 39 passed
  (5 Ayula tests: commander/counters, fight lethal, opponent Bear no trigger + token
  trigger/opposing-Bear counter target, fight target departure, fight mode omitted with no
  opposing creature).
- `cargo test -q -p aperture-core --test issue17_slice17` -> 6 passed.
- `cargo test -q -p aperture-core --test card_printings --test catalog_pool` -> 8 + 5 passed.
- `cargo test -q -p aperture-core --test issue17_slice15_corpus --test issue13_commander_combat`
  -> 2 + 43 passed.
- `cargo fmt --all --check` -> exit 0.
- `cargo clippy -q --workspace --all-targets -- -D warnings` -> exit 0.
- `cargo build -q --release -p aperture-cli` -> exit 0.

## Journey (executable, evidence-only, NOT product AI)
- `decks/package-slice18.txt`, 4 seats, exact snapshots, 40 life, Ayula injected into each
  command zone by the demonstrator; seed 1616.
- Terminal win P2; 1961 decisions, 43 turns; digest
  `09e6bb25c505c1c090759e379fc442626c9ac04bc5dffbbe6548b5f8dcc3ba24`.
- Replay digest matches; fresh `demo-save.json` round-trips exactly.
- Ayula cast 3 times; counters mode chosen 5, fight mode 4; 95 noncombat damage events.
- Label: 40-card package, **not** a desired 100-card starter certificate.

## Defects / deviations / unfinished
- Modal choice reuses `AnswerTriggerChoice` boolean (true counters, false fight), with
  mode-specific labels/legal offers, instead of adding a new action enum. This avoided
  unrelated UI action-surface changes but is less typed than ideal.
- Production modal/fight code preceded the new tests; no retroactive TDD claim.
- Missing focused coverage: Ayula pending-choice save/load/replay, ability-source departure,
  and an explicit fight prevention/replacement test. Target departure is covered.
- No full workspace suite rerun after slice18; focused suites, workspace clippy and release
  only.
- Ayula's desired 100-card list still has 30 unsupported names; no certificate or 100-card
  snapshot.
- Parent owns publication/review/CI; no push performed.
