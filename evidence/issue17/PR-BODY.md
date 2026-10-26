## Partial #17 increment — not the eight-list release gate

Adds the first genuinely playable exact starter and the sourced expansion program. **Does not close #17, certify eight lists, claim full Commander, or authorize a merge/release.**

### Implemented
- Jasmine Boreal commander and 47 sourced vanilla green/white creatures; supported catalog grows from 31 to 79.
- Exact Jasmine starter: 99-card library + separately specified commander; 41 basic lands and 58 singleton nonland library cards.
- Per-card printing/cast/resolve conformance, bounded commander registry/tax/return/damage regressions, offline analyzer snapshots, and truthful explicit-file identity.
- Real four-seat 40-life release-CLI journey using those exact snapshots: terminal, 2364 actions / 53 turns, separate replay digest matches. No bundled replacements.

### Expansion program, not implemented support
- Eight fixed archetypes now have exact desired 100-card lists (commander included), real sourced metadata and catalog-derived missing-card/interaction manifests.
- Only Jasmine is `implemented`; seven are `planned`; none is `certified`.
- Planned lists analyze as format-legal but engine-unsupported and are kept under `planned/`, not supplied as playable defaults.
- Archetypes: white aggro, GW vanilla stompy, blue draw-go, goblin tokens, bear counters, WB sacrifice, UG landfall, red spell copying.

### Evidence and boundaries
See `docs/issue17-starter-corpus.md`, `evidence/issue17/IMPLEMENTATION-HANDOFF*.md`, `TDD.md`, exact list/card snapshots, game/replay, and captured commands/results. Focused tests, workspace test/build/fmt/Clippy were exercised by the writer. Independent practical review approved the bounded increment at exact head `86db7d316edd6200f7f598849018e428d11b69eb` after both reproduced findings were fixed and re-tested. All 10 check-runs on that exact head completed SUCCESS, including both Windows package/reproducibility jobs; all configured required checks pass. The prior failed review is retained in the branch. This is **not** full #17 acceptance.

The offline analyzer still reports gameplay unavailable; this uses the existing bounded Commander-combat practice profile, not full Commander. Native defaults remain unchanged. Existing issue14 replay digest mismatch was reproduced on the untouched base and is disclosed, not silently re-certified. Arbitrary Archidekt compatibility, full legend rule/variants, AI strength and v1 acceptance are not claimed.

The operational reporter still targets completed #20; this product change does not edit shared reporting configuration.
