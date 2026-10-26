# Issue 22 — responsiveness, accessibility and input ergonomics

Scope: the native UI shell (`crates/ui`) only. This change does not touch the
engine, import/network code, the AI/session protocol, or issue #25's
`tick_ai`/request/response/generation logic. The session layer gained no
behavior; only its existing injectable worker hooks are used by tests.

## Text zoom

* `A−` / `-` zoom out; `A+` / `+` / `=` zoom in. Repeats are handled by egui's
  key-repeat events, so holding the key is repeatable.
* Usable range is **70%–200%**. The old ceiling was 190%.
* `100%` resets zoom, and so does **Ctrl+0**. egui's built-in 20%–500%
  keyboard zoom is disabled so it cannot fight this range; the app owns the
  shortcuts.
* The zoom/help/accessibility row, a pending destructive confirmation, the
  primary action strip, and the status rows are outside the bounded header
  scroll. At 200% in a 1024×640 physical window (512×320 logical points) and at
  2 physical px/point they remain laid out and reachable. The table setup/button
  rows are inside a scroll area capped at 38% of the viewport height so they
  cannot push the fixed controls off screen.

## Accessibility shell panel

`A` (or the `Accessibility (A)` button) toggles the panel. It is ignored while
a text field has focus. `Esc` closes the panel first; a second `Esc` performs
the pre-existing cancellations (target pick, unconfirmed destructive action,
inspector). The panel never runs a destructive action.

* **High contrast:** explicit white body text on near-black surfaces, lighter
  muted text and borders, bright green/yellow/red states, bright brass
  headings, and a 2 px focus outline. Rich text is palette-aware
  (`theme::text_color`, `muted_color`, `brass`, `good`, `warn`, `bad`), not just
  the backgrounds. The normal charcoal/brass palette is unchanged.
* **Reduced motion:** static `Working...` text replaces every spinner in the
  shell and `Style::animation_time` is set to `0.0`. Pending-worker polling and
  the 120 ms repaint request for outstanding work are unchanged, so a slow
  import/health/save/load still completes and is applied.
* Keyboard focus uses egui's native `Tab`/`Shift+Tab` focus and `Enter`
  activation. egui paints keyboard focus with the `active` widget style; that
  style now carries a 2 px focus-coloured frame. No widget is focused
  programmatically per frame.
* Statuses carry words: `CERTIFIED`/`REFUSED`, `ERROR:`, `disabled — every seat
  must be certified`, `Working...`, `SANDBOX-MODIFIED`, and the taint banner.

## Verification surface

Focused renderer tests in `crates/ui/src/app.rs::viewport_tests` drive the real
`render_ui` through an egui `Context` at:

* 1024×640 and 1280×720 physical windows at 200% zoom (512×320 and 640×360
  logical points);
* 2 physical px/point without text zoom;
* an injected large board (12 permanents per seat);
* a 28-entry refused per-seat import report;
* reduced motion with a **synthetic** 500 ms injectable
  `UiSession::request_ai_decision` delay (heuristic result, no model/network
  service; explicitly labelled synthetic in the test).

`theme::apply_mode` is tested for the zero animation time, the explicit HC
body text, the 2 px focus stroke, and a recoloured heading; `perf` has unit
tests for nearest-rank p95 and the bounded sample cap.

## Opt-in performance report

```
cargo run --release -p aperture-ui -- --performance-report evidence/ui-perf.json
```

The report is collected for about **10 s** with a forced ~60 Hz repaint and
bounded in-memory samples (at most 900). It is serialized and written by a
background thread after collection; no disk or network access happens in a
frame callback.

Reported separately, with **nearest-rank** p50/p95/max:

* `callback_duration_us` — main-thread wall time of the eframe update callback.
  This is not thread CPU accounting.
* `inter_callback_interval_us` — wall time between consecutive callback starts.
* `input_frame_render_us` — callback duration on frames with at least one input
  event; marked `limited: true`. This is **not** OS/GPU input-to-photon latency.

Not measured or implied: GPU present time, frame-present time, compositor
latency, OS input latency, or any hardware/manual-input measurement. The
repository's native owner must fill real measurements; this document makes no
hardware claim.

## Measured on the named reference hardware (native owner, this head)

`evidence/issue22/EVIDENCE.md` records the machine, method and raw files. The
one honest frame number is a 10 s real native run on a 4-seat bounded table
(client 1600×900 at 125 % scale, 590 samples): eframe **callback** wall time
p50 0.50 ms / **p95 0.70 ms** / max 13.85 ms, with inter-callback intervals p50
17.04 ms (≈58.7 Hz). That is main-thread callback time, not GPU present time.

Still **unverified**, and therefore not claimed: input response below 100 ms
(the synthetic-key → state readback timer did not complete), p95 frame time
under delayed AI/network work (no controlled delayed-worker run was
established), GPU/input-to-photon latency, screen-reader output, and any
hardware pass on another machine. The live UI Automation readback did confirm
the panel, its worded states and the palette switch working in the real window.

## Honest unmeasured targets

* No automated visual-inspection screenshot for high contrast or reduced motion
  was captured in this slice; the renderer tests assert styles, labels and text
  colours but not a human visual review.
* No manual Tab-order walk, screen-reader pass, or physical keyboard run was
  performed.
* No real 10 s callback/p95 measurement was made by the source writer; the
  native owner's run is recorded in `evidence/issue22/EVIDENCE.md` (idle worker
  only, callback CPU p95 0.70 ms). Nothing under a held worker was measured.
* The 200% layout is verified at logical 512×320/640×360; fractional Windows DPI
  scaling other than 2.0 is not separately exercised.
