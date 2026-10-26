Draft — **partial** issue-17 work on the starter corpus. This PR does **not**
close #17 and does not certify seven of the eight desired lists.

## This increment (slice 11): islandwalk, and one snapshot hash basis

### The Windows CI blocker, fixed at the source

`rust (windows-latest)` failed at `7d6c62e` because `crates/cli/tests/certify.rs`
hashed **raw checkout bytes** while `certify` hashes the list's text with CRLF
folded to LF. The same 1444-byte list therefore had two hashes:

| basis | bytes | sha256 |
|---|---|---|
| LF (ubuntu/macOS, certificate) | 1444 | `4312ab2a…967491` |
| CRLF (Windows checkout) | 1518 | `1fbaf76e…d53d25c` |

The basis now lives in the library: `aperture_core::digest::normalize_snapshot_text`
and `snapshot_digest` decode UTF-8 and fold CRLF→LF, and the `certify` command,
the core certification test and the CLI acceptance test all call them. A new
control re-encodes the identical list with CRLF, proves the raw bytes hash
differently, and requires the **same** certificate hash. Reproduced locally on a
CRLF checkout (RED and GREEN logs in `evidence/issue17/slice11/`).

### Two sourced cards and CR 702.14b

`Pale Bears` ({2}{G} 2/2) and `River Bear` ({3}{G} 3/3) print exactly
`Islandwalk (This creature can't be blocked as long as defending player controls an Island.)`
— live Scryfall, ice 256 and 9ed 266, one distinct printed text across 1 and 4
printings (`fixtures/starter-corpus/slice11-sourced.json`, retrieved 2026-10-26).

`CardDef::islandwalk` carries the keyword, `tokens::keywords` serves it to
`can_block`, which now takes whether the **defending** player controls an Island.
Both call sites were updated — the block validator and the greedy blocking plan
published as a legal action — so the engine cannot offer a block it would refuse.
A card merely named "Island", an attacker's own Island, or a third player's
Island changes nothing.

## Corpus state

Pool **116** cards (was 114); eight-list union **459 = 104 supported + 355
missing** (was 102/357); `ayula-bear-counters` 46 → **44** missing; desired deck
lists untouched; **1 of 8 lists certified**, seven still planned. Native audit
(`evidence/issue17/slice11/corpus_audit.py`, `corpus-audit.json`) recomputes all of
it from the tracked files, and the manifest's per-list gap lists match exactly
(`stale=0`).

Still required for #17: the other seven lists end to end, including Sol Ring,
Lightning Greaves, tutors, the legend rule and much more.

## Documented exclusions and limits

Legend rule (CR 704.5j) unmodelled and unreachable in the certified list (one
commander copy per seat, no copy effects); equipment, Auras, planeswalkers and
nonbasic lands absent; only islandwalk among landwalk variants; native table,
trainer, measured playing strength and arbitrary Archidekt compatibility stay
explicit non-goals. The credential is a bounded commander-combat practice game
under the built-in heuristic — not the Commander format and not a strength claim.

## Verification at `c10ebfe`

* `cargo test --workspace --all-targets --locked` → **669 passed, 0 failed, 95
  suites, exit 0** (`evidence/issue17/slice11/workspace-tests.log`).
* Focused slice-11 core suite 6 tests: printed text, refusal while the defender
  controls an Island, blockable without one, another player's Island irrelevant,
  the paired greedy-plan run (+/- one Island, exact life deltas), and
  printed-text/flag/blocking-table agreement for every pool card.
* Genuine RED twice: removing only the CR 702.14b branch fails exactly the two
  enforcement tests; flipping only `card_islandwalk!` to false fails the pool guard
  with "Pale Bears prints text but has no implementation".
* Provenance: the new `card_printings.rs` test asserts the catalog string is
  byte-equal to the recorded printing, and the existing "every pool card added
  after issue 15 has a recorded printing" guard now chains the slice-11 fixture.
* `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo build --workspace --release --locked` — clean.
* Real four-seat journey, release binary, `decks/package-islandwalk-slice11.txt`
  (40-card package, **not** a desired 100-card starter), seed 1114, 40 life:
  terminal, winner P1, 1517 steps, 39 turns, digest `8a1e7055798f`; separate
  replay run 1517 actions / 3830 events, digest matches. The heuristic journey
  records 0 blocks and does not by itself demonstrate the keyword — the focused
  engine tests do.
