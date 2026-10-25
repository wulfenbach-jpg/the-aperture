# Legacy replay/fixture preservation check

Base: `8a72cc607ea25f203be672c0007ad5e798be7145`.

## What is preserved

* `cargo test --all-targets` passes 467 tests with 0 failures after this change.
  That includes `replay_digest.rs`, `ordering_persistence.rs`,
  `issue13_commander_combat.rs`, `command_zone_setup.rs`,
  `apnap.rs` and the save/load guards. No state field, digest input,
  `RULES_REVISION`, replay version or save version was changed.
* The new starter replay is created and verified by the current build:
  `evidence/issue17/replay-verify.log` (seed 1, 4 seats, 2364 actions, digest
  `d7f8c2adaa17…`, recorded digest matched).
* The issue-17 card enum variants were appended; serialized card identity uses
  variant names, so no existing variant name changed.

## Tracked evidence replay from issue 14

`evidence/issue14-abilities-replay.json` (replay v6) does **not** replay to its
recorded digest in this worktree:

```
Error: replay failed: replay ended early: digest
31b1f522f3c6cf7253e6464d2b1a4895bf40b36cfd83a19b77d899af14a566f5
!= recorded ebd5340fe0212b54c47e672051a4c5ebcd3c30638e4a201a00548b65a6808f63
```

This was checked out at the untouched base commit in a throwaway detached
worktree (`git worktree add --detach C:/AI/tmp/aperture-base-8a72cc6
8a72cc607ea25f203be672c0007ad5e798be7145`, removed afterwards). The base build
produced the **same computed digest and the same mismatch**, with the same
message. The file therefore predates later accepted state fields
(issue 15's replacement/prevention/continuous-effect state) and was already stale
before issue 17. It is evidence, not a compiled fixture, and this change neither
created nor worsened the divergence.

The other tracked `evidence/**/*replay.json` files are replay version 1 and are
rejected explicitly by the current version policy (they could not replay at base
either). There is no tracked immutable v6 replay fixture in the repository that
replays successfully at base.
