Draft — **partial** issue-17 work on the starter corpus. This PR does **not**
close #17 and does not certify seven of the eight desired lists.

## This increment: the first certified starter list

`jasmine-vanilla-stompy` moves from `implemented` to **`certified`** on the
strength of a runnable certificate, not a catalog count.

New `aperture-cli certify` (`crates/cli/src/certify_cmd.rs`):

* reads the starter's declared deck file through
  `decks/starter-corpus/manifest.json`, hashes **those exact bytes**, and
  refuses to substitute a bundled practice deck;
* verifies every card in the list is supported and every nonland card is
  singleton;
* plays real four-seat 40-life commander-combat games with that snapshot in every
  seat (`repeat_n(deck.clone(), seats)`, never a package);
* requires a terminal outcome whose recorded replay reproduces the same state
  digest, and reports which cards were actually cast or played instead of
  inferring coverage from catalog membership.

Recorded certificate `evidence/issue17/slice10/certification.json`
(schema `aperture.corpus-certificate/1`):

* snapshot SHA-256 `4312ab2a90a94a4f9c9578b330a093dcec7eb1aa253785c97667cb0fab967491`,
  99 library cards + 1 separately supplied commander;
* seeds 1/2/3, all `terminal`, all replay-verified — winners P1/P2/P3,
  2364/2802/1685 steps, 53/69/40 turns;
* coverage: **55 of 61** unique names cast or played in those games; the six the
  games never cast are listed explicitly and covered by conformance suites.

The manifest record is machine-checked: `issue17_slice10_certification.rs`
recomputes the deck-file hash, requires the record, the evidence file and the
recorded games to agree, and fails when they drift (genuine RED captured in
`evidence/issue17/slice10/red-stale-snapshot-hash.log`). `crates/cli/tests/certify.rs`
runs the shipped binary, including refusals for a missing and an edited
snapshot.

## Documented exclusions and limits

Legend rule (CR 704.5j) is unmodelled and unreachable in this list (one
commander copy per seat, no copy effects); equipment, Auras, planeswalkers and
nonbasic lands are absent; native table, trainer, measured playing strength and
arbitrary Archidekt compatibility stay explicit non-goals. The credential is a
bounded commander-combat practice game under the built-in heuristic — not the
Commander format and not a strength claim.

## Corpus state

Unchanged card pool: **114 cards**; eight-list union **459 = 102 supported + 357
missing**; **1 of 8 lists certified**, seven still planned; desired deck lists
untouched.

## Verification

`cargo test --workspace --locked` → 660 passed / 0 failed (log in
`evidence/issue17/slice10/workspace-tests.log`), focused slice-10 core 3, CLI
certify 3, `starter_corpus` 8, `starter_corpus_planned` 1, `cargo fmt --all
--check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo build --workspace --release --locked`.

One independent review (fresh context, own probes) found a real coverage
counting defect; it is repaired with a regression and the claims corrected —
see `evidence/issue17/slice10-review-1.md`.

No merge, no master push, no issue closure, no downstream unlock.
