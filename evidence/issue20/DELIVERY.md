# issue20 delivery — tabletop controls, zone inspector, honest sandbox

Commit `d1bf3ea` (branch `aperture/issue-20`, PR #45). Base `1ec6d9a`.

## What was verified by command

| gate | result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | 73 test binaries, `0 failed` (434 pre-existing + 10 new issue20 + 1 legacy-taint regression) |
| package (`scripts/package.ps1`, commit `d1bf3ea`) | `PACKAGE VERIFIED: 12 file hashes re-derived, contents match MANIFEST.sha256`; zip sha256 `8d87eaf7a371997b0d72f8659c8d989ce4b9c215120d53cb242da256cdab541a` |
| packaged smoke (`aperture.exe --seats 4 --profile bounded --size 1280x720 --smoke …`) | `table-1280x720.png` written from the packaged binary |

## The real packaged-app journey

Driven with `computer_use` against
`dist/the-aperture-0.1.0-d1bf3ea/aperture.exe --seats 4 --profile bounded --size 1400x900`
(UIA element tree; the auxiliary vision model was unreachable in this run, so the
evidence below is the accessibility tree plus the app's own readouts rather than
a screenshot of each step — stated plainly rather than implied).

1. **Four-seat bounded table up.** `P0 40 / P1 40 / P2 40 / P3 40`,
   `4 seats · profile commander-combat-practice-40 (40 life)`, every seat with
   `Commander Isamaru, Hound of Konda · zone: command`, and opponents reading
   `public zones; hand/library contents hidden`.
2. **Opening choices answered through the real engine.** `Let the AI answer
   (step)` / `Keep this hand` walked P2, P3, P0, P1 to `P2 holds priority`; the
   log recorded `P2 keeps a hand of 7 … P0 keeps a hand of 7`.
3. **Zone inspector opened.** `Zones (Z)` took the tree from 130 to 186
   elements: the `Zone inspector — searchable, viewer-masked` window with its
   search field, seat and zone filters, the four per-seat count rows and the
   match list. The persistent `YOUR ACTIONS` strip stayed in place.
4. **Destructive start gated and confirmed.** `2 seats` + `Start new table with
   these options` produced, in the fixed primary strip:
   `CONFIRM: start a new 2-seat table with profile commander-combat-practice-40
   (40 life) and seed 2? The live position is discarded.` with `Confirm new
   table` / `Cancel` — and **the table was still four seats at that moment**.
   Clicking `Confirm new table` restarted as `P0 40 / P1 40`, `2 seats`,
   `started a 2-seat table … human P0, other seats heuristic AI`. `Undo (U)`
   left the toolbar with the new (empty) recorded line.
5. **A manual control applied on the packaged table.** With the sandbox panel
   open, the strip listed the labelled overrides (`Draw 1 · sandbox`,
   `Mill 1 · sandbox`, `Untap all · sandbox`, `Scry 1 · keep on top`, `Scry 1 ·
   to bottom`, `Surveil 1 · keep on top`, `Surveil 1 · to graveyard`,
   `Reveal top 3`, plus the counter and life editors). `Draw 1 · sandbox`
   produced, at once:
   * strip: `SANDBOX-MODIFIED — NOT A LEGAL MATCH RESULT (1 sandbox override(s)
     [draw])` (in the error colour, next to the digest);
   * message: `SANDBOX override applied — P0 drew 1 card(s) outside the turn
     structure. This session is tainted: no certified replay and no legal match
     result from here on.`;
   * panel: `TAINTED — 1 sandbox override(s) [draw]`;
   * log: `SANDBOX P0 draw: P0 drew 1 card(s) outside the turn structure` and
     `P0 draws Forest`;
   * board: `P0 … hand 8 · library 32` (was 7/33);
   * `Undo (U)` disappeared from the toolbar, because the recorded line no
     longer produces this position.
6. **Destructive reset gated and confirmed.** `New game` produced
   `CONFIRM: start a new game with seed 3? The live position, its log and any
   unsaved changes are discarded.` while the tainted position was still live
   (digest `3b252264`, hand 8). `Confirm new game` gave
   `started a new game with seed 3`, `turn 1 · active P0`, `hand 7 · library 33`,
   no `SANDBOX-MODIFIED` banner, no `TAINTED` line, and a log with no sandbox
   entry — the taint cleared with the position, as designed.

## Focused automated coverage added

* `crates/core/tests/issue20_sandbox.rs` (6 tests): deterministic draw/mill/
  untap/reveal with trace events and no dies trigger; a scry look visible only to
  the seat that looked while the override stays visible to both; surveil to the
  graveyard; counter-editor bounds and battlefield-only rule; life-edit bounds;
  refusal while a rules choice is pending (state untouched) and past the library;
  the typed taint record surviving save/load with a laundered save refused; and
  the inspector/search returning only projected cards.
* `crates/ui/tests/issue20_controls.rs` (5 tests): undo restores the digest,
  decision serial and RNG state exactly and the same action continues to the
  same digest, with the undone reveal gone from the log; undo refused after an
  override; taint and result labelling across save/load and a finished game;
  pending-choice and oversized-control refusals; a legacy boolean-only taint
  surviving load and re-save.
* `crates/ui/src/app.rs` viewport tests (4 new): the inspector is searchable and
  shows a card with its real zone at 1280×720 while the primary strip stays
  fixed; an opponent hand filter matches nothing and explains why; every sandbox
  control, the counter editor and the life editor become visible by scrolling at
  1280×720 with the taint banner fixed in the strip; a destructive action waits
  for its confirmation and `Cancel` changes nothing.

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
  a silent no-op.
