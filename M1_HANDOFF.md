# THE APERTURE — M1 execution handoff

## Integration 3-1 candidate (current — supersedes the historical sections below)

This candidate integrates the independently reviewed Kernel sequence
`63094a6, c57e79c, 5892783, 4c127b4, 878e28d, c1bd36f, 2f907f1` (frozen review
head `2f907f1ba822cfd120ae91b59e4d4d1a88808863`, review6 `passed=true`, no
security or logic findings) onto the published rules/CI preparation
`62359b2`. The equivalent `f3211fd` CI cherry-pick was skipped because
`e18a471`/`62359b2` already contain the same `mem::take` fix.

The retained rules conformance harness was adapted to the reviewed API — seat
vectors, `Turn.passed`, and `Action::DeclareAttackers { defender }` — without
removing any existing case, rule reference or assertion: all 8 cases across the
same four fixture files still run, with the two-seat fixtures deriving their
only live opponent when no defender is named and refusing an implicit defender
when multiple opponents are live.

Verification for this candidate (full raw logs in `run33/integration3-1/`):

- `cargo fmt --check` — exit 0
- `cargo test --all-targets --locked` — 160 passed, 0 failed, 0 ignored
- `cargo clippy --all-targets --locked -- -D warnings` — exit 0
- `cargo build --release --locked` — exit 0
- retained conformance suite — 5 test functions / 8 cases, exit 0
- paired harness, seed base 4100 — 12/12 terminal, 0 capped, 0 failed, all replay
  digests verified, exit 0
- capped probe, seed base 4200, `--max-steps 5` — 6/6 capped, exit 2, all replay
  digests verified

Scope and limitations, explicitly unresolved:

- **Not Commander.** The pool remains 15 cards; 2–4 seats are headless/CLI only;
  the native table remains two-seat practice.
- APNAP simultaneous multi-player choices (CR 101.4) remain unsupported and
  unexercised.
- The masked-digest test covers P0 and P2, not every viewer; all-viewer coverage
  is still missing.
- `RunStatus::Failed` classification exists but is not covered by a permanent
  harness test; the failing branch was exercised only through the repaired
  empty-attack regression history.
- No OS-level pointer/keyboard automation was performed; UI changes were
  verified by deterministic worker-message tests and all-target tests.
- Replay v2/save v3 change serialized two-seat digests once; older versions are
  rejected explicitly rather than migrated.
- This candidate is local only: no push, no PR update, no merge, no issue
  closure, and no claim that M1 is complete.

The historical sections below are preserved verbatim from the supervisor-reviewed
partial handoff. Their older status statements (`62359b2` as the final
deliverable, Kernel at `4c127b4`/not integrated, local head `9acf247`) are
superseded by this section; they are retained so the corrections and evidence
trail are not lost.

# THE APERTURE — M1 execution handoff

Status date: 2026-10-19. Supervisor-finalized partial handoff, not M1 completion.
The supervisor corrections to this file are deliberately uncommitted. Local
integration commit 9acf247 is also unpushed; public PR head remains 62359b2.
The authoritative machine-readable snapshot is scratch/aperture-v1-setup/m1-summary.json.

## Remote checkpoint

| Item | Value |
| --- | --- |
| Public repository | `wulfenbach-jpg/the-aperture` |
| Integration checkout | `C:/AI/aperture-m1`, branch `milestone/m1-multiplayer` |
| Published remote head | `62359b24a84ddcca56dc9daab33054d6ff7479f5` (`62359b2`) — local head, pushed and CI-green |
| Prior green checkpoint | `e18a471df6879a44301601c258d00c6cf8403fb7` (focused CI fix only) |
| Pull request | <https://github.com/wulfenbach-jpg/the-aperture/pull/34> (base `master`) |
| Focus | CI/portability remediation, public board documentation, and reviewed rules-provenance/conformance preparation — **not M1 completion** |
| Merge state | `CLEAN`; no merge performed and no push to `master` |

PR 34 is intentionally a focused gate-remediation branch. It is not a
multiplayer feature PR, and the branch must not be treated as an M1 delivery
until reviewed Kernel/Rules commits are integrated coherently.

## Final bounded-run status

The final deliverable of this bounded run is the **reviewed CI + rules
preparation at the public head `62359b2`**, still open as PR 34 with all four
required contexts green. The Kernel multiplayer migration is **not integrated
and not published**: it is preserved as clean local commit `4c127b4` on
`aperture/kernel`, and its remaining gates are
unmet or unverified.

| Item | Status |
| --- | --- |
| Public remote/PR head | `62359b24a84ddcca56dc9daab33054d6ff7479f5` — CI-green, `CLEAN` |
|| Local integration head | `9acf247` — unpushed draft; supervisor corrections to handoff uncommitted |
|| Kernel local head | `4c127b4` on `aperture/kernel` — reviewed, **not integrated, not pushed** |
|| Kernel job-2 verification | settled; OS-level native input and release build still missing |
| M1 acceptance #6–#10 | **open**; not delivered by this run |

Dispatch ledger: **9 dispatches against the conservative 8-job limit**
(transparently recorded, no hiding). Jobs 1–8 were the planned scoped work —
1 gatekeeper, 2 rules, 3 kernel (primaries), 4 Rules CI review, 5 gatekeeper
Rules review, 6 gatekeeper integration/handoff, 7 gatekeeper Kernel independent
review, 8 kernel final verification (same `kernel` session, `kernel-t2`). Job 9
was a handoff-only correction turn: the control tool started `gatekeeper-t2`
because turn 1 settled before its completion notification arrived. That turn
performed no implementation or review expansion; it only corrected stale fields
in this handoff and the structured summary. No spare dispatches remain.

Route ledger at run time: `gatekeeper` = `opencode-go/deepseek-v4.1-flash:low`;
`rules` = `opencode-go/deepseek-v4.1-flash:high`; `kernel` = same session,
`kernel-t2` = `opencode-go/deepseek-v4.1-flash:high`. The `rules` control path
broke during the run: two `vibe_send` calls returned "session rules no longer
resolves to this parent session" even though the roster showed it idle for one
turn, so the reserved job 7 was reassigned to the gatekeeper session instead of
respawning (session policy forbids respawn).

## Completed and verified

### CI gate remediation

All three `rust (*-latest)` jobs in run 39325975468 failed only at
`cargo clippy --all-targets --locked -- -D warnings` on
`clippy::drain_collect` at `crates/core/src/engine.rs:961`. The `board and
roadmap` job passed. The fix replaces `hand.drain(..).collect()` with
`std::mem::take(&mut hand)` in `apply_mulligan`, preserving the London mulligan
behavior: the hand is returned to the library and the library is shuffled before
a fresh seven is drawn; bottoming still happens when the player keeps.

Required contexts on exact head `e18a471df6879a44301601c258d00c6cf8403fb7`:

| Context | Push run 39329424928 | PR run 39329454337 |
| --- | --- | --- |
| `rust (ubuntu-latest)` | pass | pass |
| `rust (windows-latest)` | pass | pass |
| `rust (macos-latest)` | pass | pass |
| `board and roadmap` | pass | pass |

Evidence: `scratch/gatekeeper-evidence/pr34-checks.json`,
`run-39329424928.json`, `run-39329454337.json`.

### Local verification for the CI commit

- `cargo fmt --check` — exit 0
- `cargo clippy --all-targets --locked -- -D warnings` — exit 0
- `cargo test --all-targets --locked` — 136 passed, 0 failed, 0 ignored
- `cargo build --locked --bins` — exit 0
- Targeted mulligan regression `a_mulligan_that_could_never_be_bottomed_is_refused` — passed
- CLI selfplay seed 77 — 432 steps, replay verified, digest `c2a7c3993a7c`
- Node board tests 4/4; Python board tests 5/5 — exit 0
- Real headless Edge browser smoke against a loopback docs server — 33 tickets,
  6 columns, milestone filter, real search input, mobile no overflow, dated
  snapshot fallback, complete failure state, live recovery, 0 JavaScript
  exceptions; rendered board footer distinction observed directly
- Evidence directory: `C:/Users/alex3/AppData/Local/hermes/cache/scratch/aperture-v1-setup/gatekeeper-evidence/`

On the expanded integration head `62359b2`: `cargo fmt --check` exit 0,
`cargo clippy --all-targets --locked -- -D warnings` exit 0,
`cargo test --all-targets --locked` 141 passed / 0 failed, `cargo build --locked
--bins` exit 0, the conformance suite exit 0, and Node/Python board tests exit 0.
Raw logs: `integration-fmt.log`, `integration-clippy.log`, `integration-tests.log`,
`integration-build.log`, `integration-conformance.log`, `integration-board-node.log`
and `integration-board-python.log` in the same evidence directory.

### Public board documentation

`README.md`, `CONTRIBUTING.md`, `docs/ROADMAP.md` and `docs/index.html` now state
that the internal native Hermes execution board `the-aperture` is separate from
the public GitHub issue list and Pages mirror, with no automatic two-way
synchronization. The public issues remain the authoritative public status record.

### Independent review

Rules independently reviewed CI commit `c16a36b` (five paths, no blockers) and
independently reran fmt, the mulligan/core tests, all-target tests (136), Clippy
and board tests at exit 0. The reviewed commit was cherry-picked into
`milestone/m1-multiplayer` as `e18a471`.

### Rules provenance and conformance (reviewed; published in `62359b2`)

Rules branch `aperture/rules` final commit `a8b253f` adds
`docs/rules-reference.md`, `crates/core/tests/rules_conformance.rs` and five
fixture JSON files (four case files plus `manifest.json`, eight cases).

Provenance and revision history:

- `5037dd8` — first local commit, not published.
- `f9e9f58` — amended after review: removed `reason_contains`/`contains` wording
  pins, dropped `StateBasedAction` text matching (SBAs proven by observable
  graveyard/outcome state), corrected lethal damage to CR 704.5g and kept
  zero-toughness at CR 704.5f, stopped pinning Priority-before-SBA emission
  order.
- `a8b253f` — final amend changing only the `docs/rules-reference.md` case-index
  row for the multi-blocker case from CR 704.5f to CR 704.5g so the document
  matches the fixture `rule_refs`. No other file changed in that amendment.

Independently reviewed by the integration owner at `a8b253f` (read-only): fmt
exit 0, `clippy -p aperture-core --all-targets -- -D warnings` exit 0, conformance
5 tests/8 cases exit 0, and the docs case index cross-checked against every fixture
`rule_refs`. The commit was then cherry-picked into `milestone/m1-multiplayer` as
`62359b2`, which additionally passes local `cargo fmt --check`,
`cargo clippy --all-targets --locked -- -D warnings`, `cargo test --all-targets
--locked` (141 passed), `cargo build --locked --bins` and the board tests.

The official 2026-09-25 text is retained externally, not bundled: 977752
bytes/9372 lines, SHA-256
`8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`. After
director authorization, `62359b2` was pushed to `milestone/m1-multiplayer` and
all four required contexts passed on the exact head in both push run 39330884102
and pull-request run 39330891101 (`mergeStateStatus` CLEAN).

After that publication, the integration owner corrected one wording issue in
`docs/rules-reference.md` in the integration checkout only: the first-turn draw
and the free first mulligan are keyed to the **configured** game size (a game
that *begins* with more than two players; CR 100.1b, 800.1), never to the number
of surviving players after an elimination, and the draw-skip row says the same.
That correction and the worker handoff are committed locally as `9acf247`, which
remains intentionally unpushed; the public head stays `62359b2`.

## Open work — not implemented

### Kernel / M1 multiplayer

An ownership/API contract was proposed and accepted for discussion (including
the CR 800.4 active-departure correction and fixed-size pass tracking). That
contract is a proposal, not implementation.

The Kernel branch `aperture/kernel` contains the CI fix and migration through
clean commit `4c127b4407652483ddece481fc9f84a3a1173fdf`. None is integrated or
pushed. Supervisor verification of this exact local head:

- `cargo check --workspace --all-targets --locked`, `cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings`: passed; supervisor
  independently reran fmt, tests and Clippy successfully.
- `cargo test --workspace --all-targets --locked`: 147 passed / 0 failed.
- Independent gatekeeper harness `--seats 2,3,4 --seed-base 900
  --games-per-seat 2`: 12 terminal, 0 capped, 0 failed, all replay digests
  verified, exit 0 — after the empty-attack fix. The same harness failed 4/12
  including a 2-seat regression at `c57e79c`; that fix is verified.
- Supervisor fresh paired/rotated harness: seed-base 1200, seats 2/3/4,
  games-per-seat 2: 12 terminal, 0 capped, 0 failed; every replay digest matched.
- Earlier replay/save/object-reference review blockers were repaired at 4c127b4
  and checked by Gatekeeper. New tests include active-departure and four-seat
  turn order. Do not confuse historical 5892783 findings with the final head.
- Still missing: OS-level native UI input, release build, conformance-fixture
  API migration/integration and exact-head cross-platform CI for Kernel.
  Worker native rendering and egui injected-input tests do not prove OS input.
- All worker turns settled; supervisor exited Vibe (mode none, three worker
  tombstones persisted) and terminated the outer process. No worker resumed.
- Final live quota was only 4% Codex five-hour remaining (96% used). The earlier
  cached 18% snapshot was stale. No further dispatch is permitted in this run.

M1 acceptance #6–#10 — 2–4 live seats, multiplayer priority/elimination,
all-seat hidden-information projections, versioned replay and a four-seat
headless harness — remains **open**. The existing two-seat practice slice is the
only playable published scope, and no Commander or universal-coverage claim is
made.

No Commander profile/card support, universal card coverage or stable-v1 claim is
part of this handoff.

## Safety and ownership

- Do not force-push, merge to `master`, or push any worker branch
  (`aperture/ci-gate`, `aperture/rules`, `aperture/kernel`) as the integration
  head.
- One integration owner owns `C:/AI/aperture-m1` commits, evidence and issue
  transitions.
- Do not integrate unreviewed Rules/Kernel work or silently resolve design
  conflicts.
- Use command-scoped GitHub credentials only; do not print tokens or change
  global Git configuration.
- Re-check provider quota before expanding concurrency; reserve supervisor
  headroom and stop new dispatch below 15% remaining Codex capacity.
- No orphan agents: the three persistent Vibe sessions are
  `gatekeeper`/`rules`/`kernel`; no nested workers were spawned. The local docs
  server and the isolated headless Edge CDP process started for the browser smoke
  were stopped, and the evidence is preserved under
  `C:/Users/alex3/AppData/Local/hermes/cache/scratch/aperture-v1-setup/`.
