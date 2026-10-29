# Slice 24 review — Surrak and Goreclaw

**APPROVED_INCREMENT** at `3b06249d8bfc3f58cfa454e30ed2be417e23639f` (code `c2d9b35`; base `632c5e7`).

## Verified

- Source fixture matches immutable MOM 337: `9c10934d-9016-43c4-a7ab-56cc7d8f671f`,
  `{4}{G}{G}`, Legendary Creature — Human Bear, 6/5, exact oracle text.
- Trample: printed trample is read by combat assignment; `OtherControlledCreaturesHaveTrample`
  is a typed static scan requiring the source on the battlefield and same controller.
- Trigger: another nontoken creature you control is captured as non-target `affected`;
  +1/+1 counter and `GrantsHaste` are applied only to that still-battlefield incarnation.
  Self-entry, tokens, and opponents are filtered.
- Source-left resolution is independent: the trigger remains and buffs its captured
  creature after Surrak leaves.
- Affected departure: `detach_object` invalidates affected bindings in queue, stack and
  pending trigger states while retaining the mandatory trigger; no retarget/identity
  transfer.
- Haste is consulted by attack legality, tap-cost activation validation, and the actual
  mana-source offer list; cleanup expires it.
- New `TriggerRef.affected`/`StackItem.affected` are serde-default/skipped; queue/stack
  references are validated and included in archived-object protection; absent fields
  serialize exactly as before.
- Both native REDs are real behavior failures at `c457e49`
  (`native-behavior-red.log`), and both repairs are present in `c2d9b35`.

## Executed / inspected

- `cargo test -p aperture-core --test issue17_slice24 --locked`: **10/10 passed**.
- External same-id bounce/recast probe: after the old trigger resolves, bounce, remove
  Surrak, and recast the same card id, the old `GrantsHaste` effect was absent — **probe
  passed**; probe source removed, log retained at `evidence/issue17/slice24/review24-probe.log`.
- Release replay `demo-replay.json`: 1602 actions, 4127 events, digest `a41bbfd2…` matches.
- Inspected `native-journey.log`: 11 entry-buff and 11 haste events, 6 Surrak casts,
  20 blocked-trample damage events; save replay evidence retained.
- `native-corpus-audit.json` recomputes every list from catalog membership:
  141 catalog / 459 union = 129 supported + 330 missing; Ayula 24; 1 certified.

## Nonblocking limitations

- `corpus-audit.log`’s final summary line still says “Ayula 25” while its own per-list
  output, JSON, and docs say 24. Docs/current count are consistent; this is a stale audit
  summary line.
- Midtrigger save test uses a sandbox/direct fixture, not a reachable action-history save.
- No separate opponent-entry regression and no separate ordinary tap-ability haste
  regression; both are code-inspected only.
- `GameState::validate` covers queue/stack affected references, not pending-trigger copies;
  departure still clears pending copies.
- Full #17 remains **1/8 certified**, 330 missing, Ayula 24; no merge/closure; exact-head
  CI remains parent-owned.
