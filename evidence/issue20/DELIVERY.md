# issue20 delivery — tabletop controls, zone inspector, honest sandbox

Head `cadbf61` (branch `aperture/issue-20`, PR #45). Base `1ec6d9a`.
Round 2 of the same card: the review's blocking finding (R196-1) and its
follow-up (R196-2) are fixed on top of the round-1 candidate `4ed5c1c`.

## What was verified by command (this head)

| gate | result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | **455 passed, 0 failed** over **69 test binaries + 4 doc-test suites** (450 at `4ed5c1c` plus 5 new tests this round) |
| package (`scripts/package.ps1`, commit `cadbf61`) | `PACKAGE VERIFIED: 12 file hashes re-derived, contents match MANIFEST.sha256`; zip sha256 `a6fa148a75b8849c2758aa0d8126d7338ae340738de0344736bcceb59a5511f2` (two consecutive runs produced the same zip) — full log in `evidence/issue20/package.log` |
| packaged smoke at the programmatic minimum | `aperture.exe --seats 4 --profile bounded --size 1024x640 --smoke evidence/issue20/table-1024x640.png` wrote the PNG from the packaged binary (not visually inspected in this run — the auxiliary vision model was unreachable; the 1024×640 claim itself is asserted by the `render_ui` test below, not by the image) |

## Review round 1 findings — what changed

* **R196-1 (blocking), the `G` shortcut skipped the confirmation.** `G` now
  routes through `ApertureApp::ask_branch`, the same helper the Study panel
  button uses, so the shortcut cannot replace the live position without the
  question. The helper also refuses an empty study path or a still-running table
  operation with a visible `refused:` message instead of doing nothing. Covered
  by `the_g_shortcut_asks_before_it_replaces_the_live_position` (real headless
  frames: the first press only asks, digest and log untouched, the question is in
  the fixed strip, `Esc` withdraws it, and the confirmed run produces
  `branched from …`) and `the_g_shortcut_refuses_an_empty_or_busy_study_request`.
* **R196-2, the branch/taint boundary had no regression test.** Added
  `a_sandbox_taint_cannot_be_branched_into_a_clean_line`
  (`crates/ui/tests/issue20_controls.rs`): a cut that reaches into anything
  recorded after an override is refused by the revision guard (the test asserts
  the `stale submission` reason) and leaves the taint record, log and digest
  untouched; a cut before the override replays the recorded prefix to the exact
  pre-override digest with no `SandboxOverride` event left in the log, which is
  what makes clearing the taint on a branch honest rather than cosmetic.
* **Nits.** (1) `Reveal top 3 · sandbox` is the rendered label — the review nit
  came from the round-1 evidence table below, which listed it without the
  suffix; corrected here, and the 1024×640 test now asserts that label.
  (2) The test arithmetic above is the measured count, not an estimate.
  (3) The new panels are exercised at the **1024×640** programmatic window
  minimum (`the_new_controls_are_reachable_at_the_programmatic_minimum_1024x640`)
  as well as 1280×720, and `docs/issue20-tabletop.md` no longer calls 1280×720
  the minimum. (4) A failed table start / imported start now uses the existing
  `refused:` prefix, and the error-colour predicate is a tested function
  (`message_is_error`) instead of an inline chain
  (`a_failed_destructive_action_is_an_error_state_not_a_hint`). (5) The docs
  record that undo replays the recorded prefix synchronously in the frame (CPU
  only, no IO).

## The real packaged-app journey (this head)

Driven with `computer_use` against
`dist/the-aperture-0.1.0-cadbf61/aperture.exe --seats 4 --profile bounded --study-file the-aperture-save.json --branch-at 4 --size 1400x900`
(read through the UIA accessibility tree; the auxiliary vision model was
unreachable again, so this is the accessibility tree plus the app's own
readouts, stated plainly rather than implied).

1. **Four-seat bounded table up.** `P0 40 / P1 40 / P2 40 / P3 40`,
   `4 seats · profile commander-combat-practice-40 (40 life) · turn 1 · untap
   step · active P2 · stack 0 · digest 862ce08e`; every seat showed
   `Commander Isamaru, Hound of Konda · zone: command`, and opponents read
   `public zones; hand/library contents hidden`.
2. **`G` asks first (the blocking finding, in the real app).** Pressing `G`
   produced, in the fixed primary strip, `CONFIRM: branch from
   the-aperture-save.json at recorded action #4? The live position is replaced by
   the replayed recorded prefix.` with `Confirm branch` / `Cancel` — while the
   table was still `WAITING: P3: keep or mulligan the opening hand`, digest
   `8d51da35`, hand 7 / library 33. **The first press changed nothing.**
3. **`Esc` withdrew the question**; the strip returned to `WAITING: P3: keep or
   mulligan the opening hand` with the same table and digest.
4. **A real recorded line.** `AI autoplay` plus `Let the AI play my seat too`
   walked the opening through the real engine to `turn 1 · declare blockers
   step · active P2`, P2 with an `Island` on the battlefield (hand 7 / library
   32), `P3 holds priority` and live pass-round coaching. Both toggles were then
   switched off and `S` wrote `saved to the-aperture-save.json (digest
   6645a3fcf130)`.
5. **`G` again, then confirm.** `G` re-asked the same question at the same
   prefilled index (table still `6645a3fc…`, P3 priority). `Confirm branch`
   produced `branched from the-aperture-save.json at recorded action #4 — the
   recorded line is replayed to digest f068d645d342, new actions from here are
   branch history`; the table went back to `turn 1 · untap step · active P2 ·
   digest 2ad09882`, P2's Island was gone (hand 7 / library 33) and the log ended
   at the four opening keeps — the live position really was replaced by the
   replayed prefix, exactly what the question said would happen.

The sandbox taint / undo / confirmation-gate journey from round 1 (below) was not
re-walked: that code is unchanged by this round and is still covered by the tests
named there.

## Round-1 journey (unchanged code, kept for the record)

At `d1bf3ea`: 4-seat bounded table → zone inspector opened (130 → 186 AX
elements) → `Start new table` gated and confirmed as 2 seats → sandbox `Draw 1`
producing the strip banner `SANDBOX-MODIFIED — NOT A LEGAL MATCH RESULT (1
sandbox override(s) [draw])`, the panel line `TAINTED — 1 sandbox override(s)
[draw]`, the log line `SANDBOX P0 draw: P0 drew 1 card(s) outside the turn
structure`, the board at `hand 8 · library 32`, and `Undo (U)` withdrawn →
`New game` gated and confirmed (seed 3), clearing the taint. The sandbox strip
listed the labelled overrides `Draw 1 · sandbox`, `Mill 1 · sandbox`,
`Untap all · sandbox`, `Scry 1 · keep on top · sandbox`, `Scry 1 · to bottom ·
sandbox`, `Surveil 1 · keep on top · sandbox`, `Surveil 1 · to graveyard ·
sandbox`, `Reveal top 3 · sandbox`. (The round-1 table in this file omitted the
suffix on `Reveal top 3`, which is the source of review nit 1 — a documentation
error, not a missing label.)

## Focused automated coverage (all heads)

* `crates/core/tests/issue20_sandbox.rs` (6 tests): deterministic draw/mill/
  untap/reveal with trace events and no dies trigger; a scry look visible only to
  the seat that looked while the override stays visible to both; surveil to the
  graveyard; counter-editor bounds and battlefield-only rule; life-edit bounds;
  refusal while a rules choice is pending (state untouched) and past the library;
  the typed taint record surviving save/load with a laundered save refused; and
  the inspector/search returning only projected cards.
* `crates/ui/tests/issue20_controls.rs` (6 tests): undo restores the digest,
  decision serial and RNG state exactly and the same action continues to the
  same digest, with the undone reveal gone from the log; undo refused after an
  override; taint and result labelling across save/load and a finished game;
  pending-choice and oversized-control refusals; a legacy boolean-only taint
  surviving load and re-save; **a post-override branch refused while a
  pre-override cut replays the clean digest**.
* `crates/ui/src/app.rs` viewport/input tests (9, five new this round): the
  inspector is searchable and shows a card with its real zone at 1280×720 while
  the primary strip stays fixed; an opponent hand filter matches nothing and
  explains why; every sandbox control, the counter editor and the life editor
  become visible by scrolling at 1280×720 with the taint banner fixed in the
  strip; a destructive action waits for its confirmation and `Cancel` changes
  nothing; **`G` asks before it replaces the live position**; **`G` refuses an
  empty or busy study request**; **a failed destructive action is an error state,
  not a hint**; **the new panels are reachable at the 1024×640 programmatic
  minimum**.

## Honest limits

* Bounded 28–31-card native practice; one supported commander per seat in the
  bounded 40-life profile. Not arbitrary 100-card Commander, no partners, no
  trainer, no measured playing strength, no release ETA.
* `+1/+1` counters remain storage/display only; the engine's combat math does
  not apply them, and the UI says so next to the editor.
* A sandbox game's embedded replay is uncertified by construction; a tainted
  save still loads (the position is the player's) but never reads as a legal
  match result.
* Undo is refused on a tainted line; that is a deliberate, visible refusal, not
  a silent no-op. Undo replays the recorded prefix synchronously in the frame, so
  a very long recorded line costs the first frame after the key press.
* The packaged GUI journey above is read from the accessibility tree: this run
  could not capture a screenshot of each step (the auxiliary vision model was
  unreachable), and `table-1024x640.png` was produced by the packaged binary but
  not visually inspected.
* A branch *after* an override is refused by the revision guard rather than
  silently rewound; the player starts a new game or branches from before the
  override instead.
