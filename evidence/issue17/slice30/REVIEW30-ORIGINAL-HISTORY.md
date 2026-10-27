# Slice 30 review — Drover Grizzly / Saddle

**APPROVED_INCREMENT** at `44b3351db4ab1743ba9fcd8fa927648d7d2e2e26` (implementation `dea6fab`; base `de3ba07`).

This approves the bounded slice increment only; it is not full Drover Grizzly
conformance, not a starter certification, and not full #17.

## Verified

- OTJ 161 source matches immutable `560062cd-34f8-4d30-9e25-099b03961724`, exact oracle.
- Saddle 1 uses a typed sorcery-timed action, excludes the source, rejects duplicate,
  tapped, opposing, noncreature, and insufficient-power helpers, and taps valid helpers
  as the cost. Direct validation accepts a general legal set.
- Saddle resolves through a source-bound stack item; if the source leaves, its `affected`
  binding is invalidated and it resolves without marking the new incarnation.
- `Saddled` is serialized until end of turn and removed on cleanup or battlefield departure.
- The attack trigger is typed `SourceIsSaddled`; it does not fire unsaddled and grants
  trample at resolution to all creatures then controlled, including an animated Stuffed
  Bear. The grant uses the captured card, so source departure does not panic or alter the
  recipient set.
- Animated Stuffed Bear works as both helper and trample recipient through `Game::is_creature`.
- Existing 1–29 behavior remained green.

## Executed / inspected

- `cargo test -p aperture-core --test issue17_slice30 --locked`: **6/6 passed**.
- Release replay `demo-replay.json`: 2653 actions, 6968 events, digest `b711d966…` matches.
- Inspected native focused log: **61 passed**, including 6 slice30, 10 slice24, 9 slice29,
  printings/catalog/pins/replay; fmt/clippy/release exit 0.
- Inspected journey: 9 Drover casts, 116 saddle activations, 22 trample-grant events;
  save/replay evidence retained.
- Fresh corpus audit: 146 catalog / 459 union = 134 supported + 325 missing; Ayula 19;
  1 certified. `git diff --check` exit 0.

## Bounded limitation / residual scope

- The offer enumerator intentionally caps eligible helpers at 10; above that it emits no
  saddle offers even though direct `SaddleMount` validation still accepts a legal set.
  This is an acceptable bounded-practice increment, not complete card conformance. It is
  not treated as a blocking common-path defect.
- `RESULT.md` says departure effect pruning was narrowed to the saddle designation, but
  the actual `detach_object` still removes every effect targeting the departed object.
  All such effects should end on a zone change, and no behavioral defect was demonstrated;
  this is a documentation/verification discrepancy.
- No direct source-left **attack-trigger** test; the source-left no-effect test covers
  saddle activation. Code inspection shows the attack trigger is queued before departure
  and the grant has no source lookup, so it should persist; not claimed as executed.
- No same-ID legal reentry test; no full workspace matrix. Exact-head CI remains
  parent-owned.
- Full original #17 remains **1/8 certified**, 325 missing, Ayula 19; no merge/closure or
  child unlock.
