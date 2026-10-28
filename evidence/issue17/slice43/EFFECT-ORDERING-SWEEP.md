# Continuous-effect ordering sweep (review 314 follow-up, slice 43)

Review 314's supervisor note asked for the pattern behind the slice-42 defect to be
swept, not just fixed: **every site that clears or rebuilds `state.effects` before
querying a derived property** (`is_creature`, `has_subtype`, `has_trample`,
`legal_targets` / `can_target_permanent`) is a candidate for the same
"animation silently disappears" bug. This file lists every such site at the
slice-43 head and states, per site, whether the bug is reachable.

Method: `rg -n "mem::take\(&mut self\.state\.effects\)|effects\.clear\(\)|effects\.retain" crates/core/src`
plus a check that no query function mutates state, plus a read of each call site's
surroundings.

## 1. `Game::recompute_layers` — `crates/core/src/engine.rs` (~493)

The only place that rebuilds the effect list as a whole:

```rust
let effects = std::mem::take(&mut self.state.effects);   // ~798
self.state.effects = effects.into_iter().filter(...).collect();
// then: invalid_attachments are collected with effects *present* (~817)
```

- **Was the defect here?** Yes. Before slice 42 the take happened before the
  attachment check, so an animated permanent looked noncreature and lost its
  Equipment. Fixed at `20484f8`: the check now runs after the list is restored.
- **Reachable now?** No. The take and the restore are adjacent statements and no
  derived query runs between them. The only query in the function that depends on
  continuous effects (`is_creature` for attachment validity) runs after the
  restore. Retained RED: `evidence/issue17/slice42/native-red-animation.log`
  (exit 101 at the attachment assertion), proving the test discriminates.

## 2. `Game::detach_object` — `crates/core/src/engine.rs:1607`

```rust
self.state.effects.retain(|effect| effect.target != id);
```

- **Reachable?** No. Every effect whose target is the departing object must stop
  applying (CR 400.7). The function does not query `is_creature`/`has_subtype`/
  `legal_targets` afterwards — it moves the object, clears pairing, and returns.
  The next characteristic pass is a `recompute_layers` call from the caller's
  resolution path, which rebuilds pump values from the surviving effects.

## 3. `Game::finish_cleanup` — `crates/core/src/engine.rs:~5715`

```rust
self.state.effects.retain(|effect| effect.duration != EffectDuration::UntilEndOfTurn);
... self.recompute_layers();
```

- **Reachable?** No. The only work between the retain and the recompute is
  removing shields/draw replacements, `release_idle_effect_cursors`, and zeroing
  battlefield damage — all direct field writes, no derived-property query. The
  recompute that follows runs the attachment check with effects present.

## 4. `Effect::AttachSourceToTarget` — `crates/core/src/engine.rs:~7733`

```rust
// source_ok / target_ok are computed first (they call is_creature)
self.state.effects.retain(|e| !(matches!(e.kind, Equipped{..}) && e.source == Some(item.object)));
... push new attachment ... self.recompute_layers();
```

- **Reachable?** No, and the order is load-bearing: the target-validity query
  happens **before** the retain, so the new attachment is judged against a board
  that still carries every animation effect. The retain only removes the *same
  Equipment's* previous attachment (CR 301.5), which no derived query depends on
  before the recompute.

## 5. Query functions themselves

`is_creature`, `has_subtype`, `has_haste`, `has_trample`, `can_target_permanent`
and `legal_targets` are all `&self` methods that read `state.effects` and never
mutate it. There is exactly one `mem::take(&mut self.state.effects)` in the crate
(site 1) and no `state.effects.clear()` anywhere.

## 6. New for slice 43

- Lumra's characteristic-defining value is a printed static read inside
  `recompute_layers` through `count_value(LandsYouControl)` → `lands_controlled_by`,
  which reads `state.objects`. No effect list is touched before that read.
- Its enters effect (`ReturnLandCardsFromGraveyardTapped`) calls the canonical
  `enter_battlefield`, the same path a searched land uses, so the landfall hook
  fires once per land and the attachment/animation ordering is the existing one.
- **Disclosed limitation:** `Keyword::Vigilance` is currently read from printed
  keywords (`Object::keywords()`), i.e. from `tokens::keywords(card)`, not from the
  live continuous-effect path that `has_haste` uses. Nothing in the pool grants or
  removes vigilance, so no state can differ today; a future grant would have to be
  routed through `has_haste`'s mechanism first, and that is recorded here rather
  than left implicit.
