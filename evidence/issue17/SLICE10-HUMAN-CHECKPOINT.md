# Issue 17 — slice 10: the first certified starter list

Full original #17 is still UNFINISHED: **1 of 8 desired lists certified**, seven
still planned. This slice converts `jasmine-vanilla-stompy` from `implemented`
to `certified` with a runnable, re-runnable certificate, and adds no new cards.
No merge, master push, issue closure or certified downstream unlock occurred.

## What changed (code)

| file | change |
|---|---|
| `crates/cli/src/certify_cmd.rs` (new) | `aperture-cli certify`: reads the starter's declared deck file, hashes the exact bytes, refuses bundled substitution, checks every card is supported and singleton, plays four-seat 40-life commander-combat games with that snapshot in every seat, requires terminal + replay-digest parity, and reports which cards were actually cast or played. |
| `crates/cli/src/main.rs` | new `Certify` subcommand (exit 0 certified, 1 not certified, 2 invalid request). |
| `crates/core/tests/issue17_slice10_certification.rs` (new) | exact-snapshot integrity; all 11 non-vanilla cards cast and resolve from the exact set; manifest certification record must match the real deck-file bytes and the recorded games. |
| `crates/cli/tests/certify.rs` (new) | runs the shipped binary: a real four-seat certification, a missing-snapshot refusal (no bundled fallback), and an edited-snapshot refusal. |
| `decks/starter-corpus/manifest.json` | jasmine `status: certified` + machine-checked `certification` block (snapshot hash, seats, seeds, games, coverage, conformance suites, exclusions, limits, reproduce command). |
| `crates/core/tests/starter_corpus.rs`, `crates/import/tests/starter_corpus_planned.rs` | the two places that encoded "implemented / never certified" now require the certificate and still forbid certifying any other list. |
| `docs/issue17-starter-corpus.md`, `HANDOFF.md` | certification definition, recorded games, per-interaction exclusions, limits. |

No card was added, changed or claimed. The card pool is unchanged at 114, the
eight-list union unchanged at 459 = 102 supported + 357 missing, and the exact
desired lists are untouched.

## Recorded certificate

`evidence/issue17/slice10/certification.json`, schema
`aperture.corpus-certificate/1`, verdict `certified`:

* snapshot SHA-256 `4312ab2a90a94a4f9c9578b330a093dcec7eb1aa253785c97667cb0fab967491`
  over the deck-file bytes actually parsed; 99 library cards + 1 commander;
  58 unique nonland names + 41 basic lands; exactly one legendary card.
* four seats, seeds 1, 2, 3, all `terminal` with `replay_digest_matches: true`:
  * seed 1 — winner P1, 2364 steps, 53 turns, digest `8e4feea3a9e18c7d…`
  * seed 2 — winner P2, 2802 steps, 69 turns, digest `4677eb42988acd68…`
  * seed 3 — winner P3, 1685 steps, 40 turns, digest `520e8a0dd58d51e3…`
* coverage by execution: 54 of 61 unique names were cast or played in those
  games. The 6 never cast (Knight Errant, Alpha Tyrranax, Enormous Baloth, Fog,
  Swords to Plowshares, Mending Hands) are listed, not glossed: their evidence
  is the engine conformance suites, not these traces.

## Genuine RED before GREEN

`evidence/issue17/slice10/red-stale-snapshot-hash.log`: with the manifest's
recorded `snapshot_sha256` replaced by zeros, the new conformance test fails
(`left: 000…0, right: 4312ab2a…`) — a certificate record cannot drift away from
the real file. Restoring the true hash makes it pass. The CLI suite additionally
proves an edited snapshot and a missing snapshot both fail certification, so the
"never bundled replacements" bullet is tested negatively, not just asserted.

## Documented exclusions for this list

* Legend rule (CR 704.5j) is unmodelled and unreachable here: one commander copy
  per seat and no copy effect. A list that could put two same-named legends under
  one player stays uncertifiable until the SBA and its controller choice exist.
* Equipment, Auras, planeswalkers, nonbasic lands: absent from the list.
* Native Commander table, trainer, measured playing strength, arbitrary Archidekt
  compatibility: explicit non-goals.

## Limits of the claim

Bounded commander-combat practice (40 life, one supported commander per seat,
21 commander damage), not the Commander format; the certified games use the
built-in heuristic, so the certificate proves the list plays to a terminal,
replay-verified game — not that any policy plays it well or that seven more lists
are supported.

## Verification actually run

- `cargo test -p aperture-core --test issue17_slice10_certification` → 3 passed.
- `cargo test -p aperture-cli --test certify --release` → 3 passed.
- `cargo test --workspace --locked` and `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo build --workspace --release --locked`: see
  `evidence/issue17/slice10/workspace-tests.log` and the PR checks.
- `aperture-cli certify --starter jasmine-vanilla-stompy --seats 4 --seeds 1,2,3`
  → `certified`, log in `evidence/issue17/slice10/certify.log`.

## Next gate

Seven lists remain planned (Isamaru 55 missing, Talrand 51, Krenko 48, Ayula 46,
Elas 55, Tatyova 54, Zada 53), plus the unimplemented interactions they need
(sacrifice outlets, tribal/counter commander text, tutors/reanimation, legend
rule, equipment/Auras). Priority for the next slice: keep closing sourced card
packages toward Krenko (48) or Ayula (46), and never promote a list to
`certified` without its own exact-snapshot certificate.
