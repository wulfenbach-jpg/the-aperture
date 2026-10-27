# Slice 25 review — Exuberant Wolfbear

**APPROVED_INCREMENT** at `ea106932d649da6876004fdf5c1a2ce6e441d19c` (code `823680d`; base `3b06249`).

## Verified

- Source fixture matches immutable IKO 151:
  `8c8ff0da-fbe3-4c95-bc39-c975c55b6aea`, `{3}{G}`, 4/4 Wolf Bear, exact oracle.
- Attack trigger is optional, targets a Human controlled by the trigger controller, and
  captures the chosen permanent. Non-Human, opponent-controlled, and wrong-seat submissions
  are refused without state/digest change; multiple legal own Humans produce a real target
  choice.
- The source's effective P/T is read at resolution, not trigger time: two Giant Growth
  responses make the source 7/7, and the target ends 11/11 after its own counter and pump.
  Cleanup removes the setter and pump while preserving the target's counter.
- `Layer::Setting` (7b) runs before counters/7c modifiers. Base setting is timestamped and
  later setters overwrite earlier ones.
- Source-left uses the first battlefield-departure LKI snapshot (real pumped 7/7 after
  Giant Growth + Unsummon), not printed or trigger-time stats. The same-id-recast path
  retains the first LKI by construction (`is_none` guard); no separate test exists.
- Normal target/source departure and cleanup use existing pruning/expiry. New
  `source_pt_lki` fields are serde-default/skipped; legacy literals are mechanically
  initialized. No new pending kind and no AI policy change.
- The native strengthening tests are real behavioral tests, not trigger-freeze proxies.
  The earlier writer test-name issue is corrected by native’s actual target-refusal test.

## Executed / inspected

- `cargo test -p aperture-core --test issue17_slice25 --locked`: **7/7 passed**.
- Release replay `demo-replay.json`: 1760 actions, 4471 events, digest `4d6ea58e…` matches.
- Inspected native focused log: **100 passed** across abilities/card/catalog/commander/corpus/
  slice24/slice25/timing suites; including 7 slice25 and 10 slice24.
- Inspected `native-journey.log`: 8 Wolfbear casts, 9 staged triggers, 9 base-P/T set events;
  exact 4-seat/40-life package journey, save/replay evidence retained.
- `native-corpus-audit.json`: 142 catalog / 459 union = 130 supported + 329 missing; Ayula 23;
  1 certified. `git diff --check`: exit 0.

## Nonblocking limitations

- No reachable mid-optional-ability save/reload test; no dedicated target-bounce/recast,
  target-departure, or source-recast-overrides-LKI regression. Code inspection only for those
  paths; none demonstrated a behavior/security/data-loss defect.
- `native-corpus-audit.json` records code head `823680d`; the final test/docs commit
  `ea10693` does not change catalog membership.
- No full workspace matrix; exact-head CI remains parent-owned.
- Full #17 remains **1/8 certified**, 329 missing, Ayula 23; no merge/closure.
