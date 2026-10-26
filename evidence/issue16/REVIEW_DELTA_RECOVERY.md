# PR43 integration-delta recovery review

**Verdict: APPROVED — bounded integration delta only.**

**Reviewed exact HEAD:** `ae858617852ecfb0a2d1d596ed68c98392f16396`.
Old independently approved lane head: `ddae23f7a9a1f230be416aeee3cbb2b77490090a`.
Integrated master: `1ec6d9ad02b46e59069c897ab35f9aea55d6ef4f` via merge `cf46b2ca24ef3291aa7cd0109ab6e3f927656d9e`.

## Findings

- No blocking defect found in the four merge resolutions or the new digest fix. Combined merge diffs preserve the issue15 cards/layer module and the issue16 token variants/library module; catalog and README retain both feature descriptions and bounded exclusions. This is not a fresh audit of the unchanged issue15/issue16 implementations or all incoming master features.
- Empty effects/shields and zero cursors are omitted without altering nonempty serialization. New games begin with zero cursors. Effect IDs remain unique among live records: allocation advances the shared cursor; reset requires **both** live collections to be empty. Shield exhaustion cannot reset while another shield or continuous effect survives. Cleanup retains non-expiring effects before checking that guard. No retained cross-generation effect-ID reference or live timestamp-order corruption was found in the inspected allocation, application, removal and event paths. Historical IDs/timestamps may be reused after the entire live set disappears; this is not global lifetime monotonicity.
- **Compatibility limitation / nonblocking documentation follow-up:** the proven legacy guarantee is the bundled pre15/pre16 v10 fixture, not arbitrary historical saves/replays. In particular, old active pump effects and histories exercising changed layer behavior are not covered. `pre16-save-provenance.txt:29-32,42-44` uses broader language than this one fixture proves. Also, `recompute_layers` can prune the final invalid-target effect/shield without releasing cursors immediately (`engine.rs:520-550`); cleanup subsequently releases them. Thus “whenever no effect or shield is live” is not literally true at every intermediate position. This leaves surplus allocator state, not observed live identity/order corruption. The older `docs/issue15-effects.md:71-75` blanket digest-incompatibility note is now stale for the verified fixture. Approval does **not** certify universal legacy active-effect compatibility.

## Newly executed evidence

All commands exited 0 at the exact reviewed HEAD; each test selected exactly one test and passed:

1. `cargo test -p aperture-core --test counters_tokens_library --locked a_real_pre16_v10_save_still_loads_and_replays`
2. `cargo test -p aperture-core --test issue15_effects --locked prevention_shield_absorbs_its_amount_and_stops_applying`
3. `cargo test -p aperture-core --test issue15_layers --locked until_end_of_turn_effects_end_in_cleanup`
4. `cargo test -p aperture-core --test issue15_layers --locked a_real_game_logs_the_new_effects_and_replays_to_the_same_digest`

Inspected the legacy test body: it loads/validates the stored state, preserves 45 replay steps and 132 log events, asserts the unchanged digest `282eea6340239bf2be48ca8f75e09127a193e22c1e5fdaab3847cedafc739e80`, and independently runs the replay to that digest. `git diff --check` for old approved head through candidate passed.

## Reused evidence and preservation

Earlier FULL review192 approval and root's reported 457-test workspace/fmt/Clippy/release green are carryforward context, **not newly executed or independently reverified here**. No broad suite, generator rebuild, seed matrix, scratch test or prior reviewer loop was rerun. No tooling blocker occurred.

Initial and final pre-report checks confirmed the exact HEAD and no tracked modifications. Existing untracked `crates/core/tests/scratch_pre16_replay_check.rs` was left untouched and unexecuted. Only this review report was created; production code, tests, assertions and recorded digests were not edited. No merge, push, closure or agent launch was performed.
