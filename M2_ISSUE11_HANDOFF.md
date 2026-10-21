# M2 issue #11 handoff — Commander profile legality (analysis only)

Worktree `C:/AI/aperture-worktrees/issue-11`, branch `aperture/issue-11`,
base `f7772b963cca155228ccfa85ba0f7431543fcd4d`. Working tree only: **no
commit, push, PR, issue closure, or board mutation by this worker.**

## Scope delivered

Offline Commander **deck-profile legality analysis** in `crates/import` plus a
real `aperture-cli commander-check` command. Commanders are a distinct zone;
results are tri-state; format legality is separate from engine/card support;
gameplay is unconditionally unavailable. The 40-card practice import/gameplay
is unchanged except that Commander/Companion/Sideboard text is now refused
instead of silently flattened.

## Honest TDD incident

The first implementation attempt wrote a broad `commander.rs` (71,008 bytes)
after only a missing-module compile RED. The director identified this as a
vertical-TDD violation. That file was deleted (it is not retained anywhere) and
the feature was rebuilt from a minimal compiling tracer with selected
behavioral RED → minimal GREEN slices. The historical compile-only RED log is
kept as `evidence/m2-commander-missing-module-compile-red.log`; the broad
attempt's compile log is kept as
`evidence/m2-commander-unearned-broad-compile-incident.log`. The 943-line
design matrix was removed after its cases were migrated to executable tests; it
was never counted as evidence.

The `missing_card_metadata` case in the metadata slice was already
`not_certified` via `CMD-FORMAT-MISSING`; its failure was an internal-code
expectation, not a status transition. It is not counted as behavioral proof.
The genuine transitions in that slice were partner, chosen-color, and
non-normal status changes.

## Staged evidence (raw logs in `evidence/`)

| Slice | RED log | GREEN log | Result |
| --- | --- | --- | --- |
| Practice-file guard | `m2-commander-practice-red.log` | `m2-commander-practice-green.log` | 2 behavioral failures (Commander/Companion flattened) → 3/3 pass |
| Minimal tracer | — | `m2-commander-tracer-green.log` | parses/binds snapshots, claims nothing |
| Legal-100 vertical | `m2-commander-legal-red.log` | `m2-commander-legal-green.log` | status `not_certified` → `legal`; not playable |
| Identity | `m2-commander-identity-red.log` | `m2-commander-identity-green.log` | legal → illegal for off-color Island |
| Metadata/variants/faces | `m2-commander-metadata-red.log` | `m2-commander-metadata-green.log` | partner, chosen-color, non-normal transitions; face evidence |
| Singleton | `m2-commander-singleton-red.log` | `m2-commander-singleton-green.log` | duplicate nonbasic illegal; Rat Colony NC |
| Companions/sideboard | `m2-commander-zones-red.log` | `m2-commander-zones-green.log` | not counted, NC |
| Adversarial provenance | `m2-commander-adversarial-red.log` | `m2-commander-adversarial-green.log` | conflicting oracle order, null text, unknown type all NC |
| Same-name/different-oracle ambiguity | coverage only | `m2-commander-adversarial2.log` | added after the probes; not claimed as TDD RED |

Additional executable coverage: `commander_edges.rs` (8 tests),
`commander_acceptance.rs` (7 tests: 99/100/101, commander quantity,
face completeness, Background variants, bans/staleness, missing/tampered
metadata, no eligible commander), `commander_laundering.rs`, CLI and native
file-consumer tests. Some acceptance cases cover behavior already implemented
before the test was written; they are regression coverage, not TDD proof.

## Supplied adversarial CLI probes re-run

`evidence/supervisor_probes_rerun.py` is the supplied probe logic copied
unchanged in behavior; it writes to `evidence/supervisor-probes/` so the phase
originals are untouched. Results (`evidence/m2-commander-supervisor-probes-rerun.log`):

| probe | before | after |
| --- | --- | --- |
| valid-control | legal / 0 | legal / 0 |
| conflicting same-oracle append | legal / 0 | not_certified / 2 |
| conflicting same-oracle prepend | illegal / 1 | not_certified / 2 |
| null oracle_text commander | legal / 0 | not_certified / 2 |
| unknown type_line token | legal / 0 | not_certified / 2 |

## Assignment-5 repair (post-freeze findings)

Two independent review findings were reproduced with actual CLI output, then
fixed with a failing regression first:

* **Grist, the Hunger Tide** (`0efb0d7e-dea0-4817-a243-15066e9ef333`) is a
  legendary planeswalker whose static ability makes it a 1/1 Insect creature
  while it isn't on the battlefield. It is Commander-legal, but the slice must
  refuse the zone-dependent class as `not_certified`. Before:
  `illegal` / exit 1 / "commander is not a legendary creature"; after:
  `not_certified` / exit 2 / "zone-dependent/type-altering off-battlefield
  commander eligibility (CR 903.3) is not certified". Logs:
  `m2-repair5-grist-red.log`, `m2-repair5-grist-green.log`, CLI output
  `m2-repair5-grist-cli.json`.
* **Shorikai, Genesis Engine** is `Legendary Artifact — Vehicle`; Vehicle is a
  subtype after the em dash, which the previous type-segment check missed. It
  is a CR 903.3 Vehicle commander category this slice does not model. Before:
  `illegal` / exit 1; after: `not_certified` / exit 2 / "Vehicle commander
  (CR 903.3 category; power/toughness eligibility not modelled)". Logs:
  `m2-repair5-shorikai-red.log`, `m2-repair5-shorikai-green.log`, CLI output
  `m2-repair5-shorikai-cli.json`.

Both fixtures are under `crates/import/tests/fixtures/commander/real-grist/`
and `real-shorikai/` with manifests. The pre-repair hashes are recorded in
`evidence/m2-repair5-pre-repair-hashes.txt`; Rules was notified before writes.
Doc/allocation cleanups were also made in `commander.rs` and
`core/src/deck.rs` (no per-line header `to_lowercase` allocation).

Independent review subsequently supplied concrete findings for unknown header
forms such as `Commander (1):`, the detached full-artifact `raw_sha256` claim,
and partner text with an empty keyword array. Those were authorized and fixed in
the continuation section below. The interchangeable/alias case was investigated
and required no production change.

## Assignment-5 continuation (header, provenance, partner text, alias)

These four concrete review findings were addressed under the same assignment;
each production change had a failing behavioral regression first except the
alias case, which Rules confirmed was already fail-closed:

* **Section-header bypass.** `Commander (1):` / `Sideboard (15):` were skipped
  by the old `ends_with(':')` rule and a 100-card list started practice
  self-play. `parse_practice` now accepts card lines and the acknowledged
  `Deck`/`Mainboard` headers only and rejects every other non-card header. Behavioral RED at
  the core parser, real CLI `selfplay`, and native `load_explicit_deck`:
  `m2-repair5-header-{core,cli,ui}-red.log`; GREEN:
  `m2-repair5-header-{core,cli,ui}-green.log`. Actual CLI: exit 1 with the
  unknown-header refusal (`m2-repair5c-smoke.log`).
* **Detached `raw_sha256`.** A zero/unrelated `raw_sha256` was accepted and
  echoed even though only `cards_sha256` was checked. `raw_sha256` now binds
  the exact embedded `raw_cards_jsonl` bytes (same domain as `cards_sha256`);
  a detached full-artifact claim is rejected. RED/GREEN:
  `m2-repair5-rawsha-{red,green}.log`; phase fake-sha CLI probe now exit 2.
  Fixtures/manifests were updated to the embedded-byte domain; the recorded
  source-download digest is explicitly unverified, not presented as candidate
  provenance. The supplied probe copy now refreshes both digests so it tests
  rule behavior rather than provenance failure.
* **Partner rules text with empty `keywords`.** Detection now cross-checks the
  Oracle text for the word `partner` (and partner with) as well as keywords, so
  an empty keyword array cannot produce `legal`. RED/GREEN:
  `m2-repair5-partner-{red,green}.log`; actual CLI `not_certified`/2.
* **Mainboard regression (assignment-6 finding).** The docs/spec accepted
  `Deck`/`Mainboard` but the parser accepted only `Deck`, so a valid 40-card
  `Mainboard:`-prefixed practice file exited 1. The shared parser now accepts
  exactly `Deck` or `Mainboard`; all other headers (including
  `Commander (1):`, `Sideboard (15):`, `Deck (40):`, `Mainboard (40):`) remain
  refused. RED/GREEN at shared parser, real CLI, and native startup:
  `m2-repair5-mainboard-{core,cli,ui}-{red,green}.log`; actual `coach` exit 0,
  `selfplay` no longer refuses, Commander paren header still exit 1
  (`m2-repair5c-smoke.log`).
* **Interchangeable/alias names.** Rules investigated and confirmed Scryfall
  represents an alternate printed name as `printed_name` on the same oracle id;
  the canonical-name mismatch already yields ambiguous identity / NC, and
  `flavor_name` has no deck-construction meaning. No production name mechanism
  was added. Alias regression: `m2-repair5-alias.log` (coverage, not TDD RED).

## Verification (all after assignment-5/6 repairs)

| gate | command | result |
| --- | --- | --- |
| format | `cargo fmt --check` | clean (`m2-repair5c-fmt.log`) |
| tests | `cargo test --workspace --all-targets --locked` | **251 passed / 0 failed** (`m2-repair5c-full-tests.log`) |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean (`m2-repair5c-clippy.log`) |
| release | `cargo build --release --locked` | clean (`m2-repair5c-release.log`) |
| practice CLI | `selfplay --seed 101` | terminal P0, 414 steps, replay verified (`m2-repair5c-smoke.log`) |
| 2/3/4 seats | `harness --seats 2,3,4 --seed-base 1 --games-per-seat 2` | 12 terminal / 0 capped / 0 failed, replay verified (`m2-harness.json`) |
| practice refusal | `selfplay --deck <Commander: text> ...` | nonzero, explicit practice-scope refusal |
| Mainboard practice | `coach`/`selfplay` with valid `Mainboard:` 40-card file | accepted (coach 0; selfplay not refused), count-variant headers still refused (`m2-repair5c-smoke.log`) |
| commander CLI | legal/illegal/NC + Grist/Shorikai/header/rawsha/partner | exits 0/1/2; Grist, Shorikai, partner-text NC/2; paren header practice refusal 1; fake raw_sha256 2 (`m2-repair5c-smoke.log`) |
| native file consumer | full workspace tests include UI session | 13/13, startup accepts `Mainboard:` and refuses `Commander:`/`Commander (1):` (`m2-repair5c-full-tests.log`) |

## Frozen candidate hashes (working tree, not committed)

HEAD remains `f7772b963cca155228ccfa85ba0f7431543fcd4d`.

```text
258a9404b4bc7b9165abd0510f69f70449eede853f20a07fbcf95932120bf66e  crates/import/src/commander.rs
a638a8ae62367e9753cb8725e8429034e251818b57b75024235d73303e22fe9f  crates/cli/src/commander_cmd.rs
2095fe9c252a30c285dfdd55a7de3cc1f6c27b31cfdabe1864aafd8ca249a3cb  crates/core/src/deck.rs
a6c679e32b60cf6ab1a564d638a5489f5236bb53aad5504da4e37f0e1aa11681  crates/import/tests/fixtures/commander/real/cards-scryfall.json
3f5795c975cc62269e672e8b3294f7f49f6b78b1144e155a7c31d8fc701dac36  crates/import/tests/fixtures/commander/real/deck-legal-100.json
5c8f5c656a8f093b5f437d92a9312a8c7dfebe7f4daec7cf0dcbdc5295f7f829  crates/import/tests/fixtures/commander/real/manifest.json
68ab2b5eaf360d1115ad0edd7ad5f2da9565aa47f22d9da8106e25d217b5214b  crates/import/tests/fixtures/commander/real-grist/cards.json
01448f2ef0018057e32edbd283e21fc6be8dcb8079ff8db7be103981466021b2  crates/import/tests/fixtures/commander/real-grist/deck.json
571ac99acb84cd78423084b20ef8ece0bc610cc8d9b28df0bea567f836b47a6b  crates/import/tests/fixtures/commander/real-shorikai/deck.json
```

Post-repair freeze manifest: `evidence/m2-repair5-freeze.txt` (hashes, git status,
251-test gate, Mainboard/Grist/Shorikai CLI and regression probes). The candidate is the
uncommitted working tree above. The earlier `evidence/m2-commander-freeze.txt`
is retained as the pre-repair freeze for comparison.

## Touched paths

Modified: `Cargo.lock`, `crates/cli/Cargo.toml`, `crates/cli/src/main.rs`,
`crates/core/src/deck.rs`, `crates/import/src/lib.rs`,
`crates/ui/tests/session.rs`, `README.md`, `docs/ROADMAP.md`, `ASSETS.md`.

Added: `crates/import/src/commander.rs`, `crates/cli/src/commander_cmd.rs`,
`crates/cli/tests/commander_check.rs`,
`crates/core/tests/practice_commander_guard.rs`,
`crates/import/tests/commander_{legal_tracer,identity_tracer,metadata_tracer,
singleton_tracer,zones_tracer,adversarial,acceptance,edges,laundering,grist,shorikai}.rs`,
`crates/import/tests/fixtures/commander/real/{cards-scryfall.json,
deck-legal-100.json,manifest.json}`, `real-grist/{cards,deck,manifest}.json`,
`real-shorikai/{deck,manifest}.json`, `docs/commander-legality.md`, and the
`evidence/m2-*` logs/probes/smoke artifacts.

## Limits / remaining gates

* No commit/push/PR; independent frozen review and native-owner confirmation are
  required. Reviewer must inspect the actual source/fixtures/logs, not this
  summary.
* Playable Commander remains absent: command zone/return, tax, 40 life, 21
  commander damage, multiplayer seats.
* All non-normal layouts and all 2+ commander variants are deliberately
  `not_certified`; no partner/background certification, no face-aware engine.
* Provenance proves embedded-byte integrity, not remote origin authenticity.
* Ban status is as of the recorded capture; stale (>30 days) is disclosed, not
  refreshed live.
* No Archidekt adapter; offline snapshots are the canonical input.
* The native GUI was not manually clicked; its actual startup file consumer was
  exercised by `load_explicit_deck` tests. No `--smoke` screenshot was taken for
  this analysis-only path.
