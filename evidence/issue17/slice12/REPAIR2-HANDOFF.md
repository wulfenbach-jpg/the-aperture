# Slice 12 repair 2 — save/token archive and test-sentinel repairs

Repair head `5870716` was committed and normally pushed to origin/aperture/issue-17. Reviewed head `9ec90aa`.
Full #17 unfinished; no independent review claim.

## R12-F1 — saved artifact journey was unreloadable

Reproduced exactly:

```
target/release/aperture-cli.exe replay --file evidence/issue17/slice12-review-demo-save.json
  Error: invalid game state: token #541 (Goblin Token) is in the removed from
  the game zone; tokens cease to exist off the battlefield
target/release/aperture-cli.exe replay --file evidence/issue17/slice12/save.json
  ... token #507 ...
```

The writer handoff’s earlier “save replay matches” claim was **false** for both saves; it is superseded by this repair and the failed records are preserved. This is inherited departure/validation
behavior, not artifact casting.

**Genuine RED:** `removed_tokens_are_valid_only_as_inert_archives_of_departed_owners`
failed with the same “removed from the game zone” validator error before the fix.

**Fix:** `GameState::validate` admits a token in internal `Zone::Removed` **only**
when its owner has already left the game. That record must also have no list,
stack, combat or other live reference, or validation fails. Live-owner Removed
tokens and tokens in Hand/Graveyard/Exile/Stack/Library remain refused. Ordinary
battlefield→graveyard token cessation and removal semantics are unchanged. At
departure, ephemeral counters are cleared on archived objects.

Compatibility preserved; no version bump. Verified unchanged digests/events:

```
slice12/save.json                 -> 328dadbdbdc6, 7051 events
slice12-review-demo-save.json     -> d3eade58f133, 6981 events
fresh --save-out of demo-replay   -> d3eade58f133, 6981 events
old slice8 replay                 -> ffbeaee0db38, 6087 events
```

## R12-F2 — stale Elas prose

`docs/issue17-starter-corpus.md` corrected from 55 to **53** unsupported Elas
nonland names, matching manifest/audit. Artifact/current counts unchanged:
catalog 119, union 459 = 107 supported + 352 missing.

## CI sentinel — test-only compatibility

CI on `9ec90aa` failed four `crates/import/tests/start_flow.rs` cases because
`Sol Ring` is now supported and was used as the unsupported sentinel. Replaced
the sentinel with **Darksteel Ingot** (genuine still-unsupported colorless
artifact) in test fixtures only; all Unsupported/NotCertified/singleton/start
refusal assertions preserved. Added a guard asserting Darksteel Ingot is absent
from the catalog. No production import or UI change.

## Verification (actual)

```
issue17_slice12           10 passed
issue17_tokens            10 passed
issue17_slice6            13 passed
issue13_commander_combat  43 passed
catalog_pool               5 passed
card_printings             7 passed   (review count 7 confirmed)
aperture-import start_flow 21 passed
cargo fmt --all --check    exit 0
cargo clippy --all-targets -D warnings  exit 0
cargo build --release -p aperture-cli   exit 0
```

No new game was run; existing artifact/old journeys were reused with unchanged
digests.

## Remaining

Artifact package itself is unchanged. No 100-card desired starter certification,
no merge/closure, no AI policy changes. Archived-Removed tokens are inert
internal leave-game storage only, not a real Magic zone.
