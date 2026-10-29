# Issue 17 — slice 11 human checkpoint (2026-10-26, run 230)

Status: **bounded increment delivered and pushed; full #17 remains unfinished.**
Same card, same branch `aperture/issue-17`, same draft PR
[#46](https://github.com/wulfenbach-jpg/the-aperture/pull/46). No merge, no master
push, no issue closure, no successor activation.

## 1. The blocker this slice inherited

`rust (windows-latest)` failed at `7d6c62e` (run 40349100232, job 121663944496):
`crates/cli/tests/certify.rs:69` compared the certificate's `snapshot_sha256`
against a hash of **raw checkout bytes**. `certify` hashes the list's text with
CRLF folded to LF, so one 1444-byte list has two raw hashes:

| basis | bytes | sha256 |
|---|---|---|
| LF (ubuntu/macOS checkout, certificate) | 1444 | `4312ab2a90a94a4f9c9578b330a093dcec7eb1aa253785c97667cb0fab967491` |
| CRLF (Windows checkout) | 1518 | `1fbaf76e10320beeea6b69fb5b2cadcf4437040a63eb954f3b06dce52d53d25c` |

Reproduced locally by converting only that tracked file to CRLF
(`evidence/issue17/slice11/red-crlf-checkout.log`): the test panicked with
exactly those two values, so the CI failure was a test-side portability defect,
not a product bug.

**Repair.** The basis moved into the library instead of being restated in tests:
`aperture_core::digest::normalize_snapshot_text` / `snapshot_digest` decode UTF-8
and fold CRLF→LF. The `certify` command, the core certification test and the CLI
acceptance test all call it. A new control
(`a_crlf_checkout_of_the_same_list_certifies_to_the_same_snapshot_hash`)
re-encodes the identical list with CRLF, proves the raw bytes really hash
differently, then requires the same certificate hash. Verified by re-running the
CLI suite on a CRLF checkout: 4/4 pass
(`evidence/issue17/slice11/green-crlf-checkout.log`).

## 2. New corpus behaviour (sourced, printed text only)

Two cards, both from the Ayula desired list, both taken verbatim from Scryfall
(`fixtures/starter-corpus/slice11-sourced.json`, retrieved 2026-10-26T16:27Z):

* **Pale Bears** — `{2}{G}`, Creature — Bear, 2/2, ice 256, 1 printing.
* **River Bear** — `{3}{G}`, Creature — Bear, 3/3, 9ed 266, 4 printings.

Oracle text, identical on every printing and byte-equal to the catalog string:
`Islandwalk (This creature can't be blocked as long as defending player controls an Island.)`

New rule: CR 702.14c (702.14b only names landwalk as an evasion ability; the
blocking restriction itself is 702.14c). `tokens::can_block` now takes
`defender_controls_island`, read by the engine from the **defending** player's
battlefield (`Game::controls_island`, substring-free subtype test
`CardDef::is_island`). Two call sites were updated — the block validator and the
greedy blocking plan published as a legal action — so the engine can never offer
a block it would refuse. An islandwalk attacker's own lands, or a third player's
Islands, change nothing.

## 3. Corpus movement (native audit, `evidence/issue17/slice11/corpus-audit.py`)

| metric | before | after |
|---|---|---|
| pool | 114 | **116** |
| eight-list union names | 459 | 459 |
| supported | 102 | **104** |
| missing | 357 | **355** |
| `ayula-bear-counters` missing | 46 | **44** |
| certified lists | 1 | **1** |

Still **1 implemented / 1 certified of 8**; seven starters remain planned. The
manifest's `missing_cards` no longer lists the two supported names (verified:
`stale=0` for every list). Desired snapshot files are untouched.

## 4. Verification actually run

* `crates/core/tests/issue17_slice11.rs` — 6 focused tests: printed text, refusal
  while the defender controls an Island, blockable without one, another player's
  Island is irrelevant, and the paired greedy-plan run (same seed, +/- one Island)
  where the unblocked 3/3 takes exactly 3 life and the blocked one takes 0.
* Genuine RED (`evidence/issue17/slice11/red_islandwalk.py`): removing only the
  CR 702.14c branch from `can_block` fails exactly the two enforcement tests and
  leaves the three controls green (`red-before-islandwalk-enforcement.log`).
* `crates/core/tests/card_printings.rs` — new provenance test plus the existing
  "every pool card added after issue 15 has a recorded printing" guard now chains
  the slice-11 fixture.
* `crates/core/tests/catalog_pool.rs` — the pool guard lists Islandwalk among the
  modelled keywords, keyed on the new `CardDef::islandwalk` flag. Genuine RED for
  that flag (`evidence/issue17/slice11/red_islandwalk_flag.py`): flipping only
  `card_islandwalk!` to `islandwalk: false` makes the guard fail with
  "Pale Bears prints text but has no implementation"
  (`red-before-islandwalk-flag.log`); restored, it passes.
* `cargo test --workspace --all-targets --locked` — **669 passed, 0 failed, 95
  suites, exit 0** (`evidence/issue17/slice11/workspace-tests.log`).
* `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo build --workspace --release --locked` — all clean.
* Real four-seat journey with the release binary on
  `decks/package-islandwalk-slice11.txt` (40-card **package**, not a desired
  100-card starter), seed 1114, commander-combat practice, 40 life:
  **terminal**, winner P1, 1517 steps, 39 turns, digest `8a1e7055798f`; separate
  `replay` run: 1517 actions, 3830 events, digest matches. Re-run after the
  `CardDef` flag landed it reproduces the same digest, so the flag is
  behaviour-neutral. Honest limit: the heuristic journey records 0 blocks and does
  **not** by itself demonstrate the keyword — the focused engine tests above do.

## 5. Exact-head CI

Head of record **`c10ebfe903d0c755236f8134f503b26ae6435030`**: **10/10 check-runs
SUCCESS** — board and roadmap ×2, `rust (ubuntu-latest)` ×2, `rust (macos-latest)`
×2, `rust (windows-latest)` ×2, `windows package` ×2
(`evidence/issue17/slice11/exact-head-ci.json`). The Windows Rust job that was red
at `7d6c62e` is green here, which is the actual proof of the portability repair.

Earlier heads on the same branch and what they mean:

| head | result | note |
|---|---|---|
| `7d6c62e` | windows Rust FAILED | inherited blocker, the reason this slice ran |
| `b6f6471` | 10/10 SUCCESS | portability repair alone, windows green |
| `31c6084` | 3 rust jobs FAILED | islandwalk before the `CardDef` flag; the pool guard caught it, superseded |
| `c10ebfe` | **10/10 SUCCESS** | head of record |

PR #46 is OPEN, draft, MERGEABLE; `mergeStateStatus` is BEHIND current master
because other lanes merged after `8de0961`. No body/state change beyond this
increment's description.


## 6. What is still missing (not claimed)

* Seven of eight starters are still `planned`; 355 union names are unsupported.
  Ayula still needs 44 cards, including Sol Ring, Lightning Greaves, tutors and
  the commander herself.
* Islandwalk is the only landwalk variant; no other evasion, no keyword grants.
* `Pale Bears`/`River Bear` are not yet in the Ayula certified path: a list is
  certified only when every one of its cards is supported **and** the snapshot
  certifies, and that is far from true here.
* The certified credential is still a bounded 40-life commander-combat practice
  game under the built-in heuristic — not the Commander format, not a strength
  claim, not arbitrary Archidekt support.

## 7. Next slice priorities

1. Keep closing sourced Ayula/Krenko packages where the printed text maps to
   existing primitives (counters, tokens, keywords are now implemented).
2. Certify a second list only when its whole snapshot is supported; never infer
   certification from a catalog count.
3. Preserve this head's approval and do not re-audit approved slices 1–10.
