# Issue 22 — native owner evidence (run 211)

Base: accepted-import master `055fde7bc2ff296dc1d94c07dfd566fc7082bdf2`, branch
`aperture/issue-22`, exclusive worktree `C:/AI/aperture-worktrees/issue-22`.
Scope: `crates/ui` shell plus `docs/`/`README`/`evidence` only. `session.rs`, the
`tick_ai`/request/response/generation boundary and every core/AI/import file are
untouched, so #25's lane has no shared hunk with this change.

## What was built (own implementation on top of the Go/DeepSeek source writer)

* **Text scale** 70–200 % (was 190 % max), `100%` reset button, and `Ctrl+0`
  reset. egui's built-in 20–500 % keyboard zoom is disabled so the app owns the
  range.
* **Accessibility (A) panel**: high-contrast and reduced-motion toggles, each
  with an explicit ON/OFF sentence. `Esc` closes the panel before any other
  cancellation and never triggers a destructive action. `A` is ignored while a
  text field has focus.
* **High contrast** palette is applied to widget strokes, selection, window
  borders and every palette-aware rich-text helper (not just backgrounds).
  Measured worst-case background is pure black `#000000` with body text
  `#FFFFFF` (contrast ratio 21:1); the brightest status/accent pairs stay above
  the 4.5:1 text minimum for the sampled pairs used by statuses.
* **Reduced motion** replaces every shell spinner with static `Working...` and
  sets `animation_time = 0.0`; worker polling and the 120 ms repaint request for
  outstanding work are unchanged.
* **Non-color-only status**: `WAITING: …`, `WAITING ON P2`, `CERTIFIED`,
  `REFUSED`, `ERROR: …`, `SANDBOX-MODIFIED`, `Working...`, `disabled — every seat
  must be certified` are words in the accessibility tree, not colour dots.
* **Layout**: the zoom/help/accessibility row, destructive-confirmation bar,
  primary-action strip and status rows are outside the bounded header scroll,
  so they survive long per-seat reports, a 12-permanent board, 200 % text and a
  1024×640 window.
* **Opt-in instrumentation**: `--performance-report PATH` (bounded 10 s,
  ~60 Hz forced repaint, nearest-rank p50/p95/max, JSON written by a background
  thread; no disk/network on a frame callback), `--performance-note TEXT` to
  disclose a collection condition, and `--a11y high-contrast,reduced-motion` to
  start a capture in that mode.

## Verification actually run (this checkout, this head)

| command | result |
| --- | --- |
| `cargo fmt --check` | clean (`verify4-fmt.log FMT=0`) |
| `cargo test -p aperture-ui --locked` | **77 passed / 0 failed** — 34 lib (16 new renderer/accessibility/perf tests) + 11 + 6 + 4 + 22 integration (`verify4-ui-tests.log`) |
| `cargo clippy -p aperture-ui --all-targets --locked -- -D warnings` | clean (`CLIPPY=0`) |
| `cargo build --release -p aperture-ui --locked` | success (`BUILD=0`) |

Original RED evidence (mine, against the untouched base, not the writer's
sequence): two owner-authored behavioural probes extracted from `055fde7` at
`evidence/issue22/red-base` fail exactly as expected — `set_zoom(2.0)` is
clamped to `1.9`, and `A` exposes no high-contrast control through the real
renderer (`owner-red-base.log`, 2 failed / 23 filtered). The writer's own RED
log is retained at `evidence/issue22/tdd.log` with its nine genuine renderer
gaps.

## Measurements (named reference hardware)

Hardware and method are in `reference-hardware.json`: AMD Ryzen 5 3600
(6C/12T), 34.3 GB RAM, RTX 3070 driver 32.0.16.1062, 2560×1440 @143 Hz,
Windows 10 Pro 19045, rustc/cargo 1.97.1. Shared desktop; the user's other
applications (including a game) stayed open. No GPU, model or remote service was
started.

**Frame time — `perf-baseline.json`, 4-seat bounded table, `--size 1280x720`
(client 1600×900 at 125 % scale), 10 s window, 590 samples:**

| quantity | p50 | p95 | max | target |
| --- | --- | --- | --- | --- |
| eframe callback wall time | 0.50 ms | **0.70 ms** | 13.85 ms | p95 < 33 ms → **met for this measured quantity** |
| inter-callback interval (forced ~60 Hz) | 17.04 ms | 17.69 ms | 97.82 ms | ~58.7 Hz median |

Input-event frames in that run: 2 (p95 not meaningful, so no input-response
claim). Interpretation is deliberately narrow: this is main-thread callback
time, **not** GPU present/frame-present time and **not** end-to-end input
latency; the report states the same limitation in its own body.

**Real-app accessibility/input journey (Windows UI Automation, own exact HWND,
background invocation — no global input, no focus steal):**

* `captures/dump2.json` — 131 named controls read back from the live tree,
  including `Accessibility (A)`, `A+`, `A−`, `100%`, all four seat panels,
  `WAITING: P2: keep or mulligan the opening hand (CR 103.5)`, `WAITING ON P2`
  and the fixed primary strip. Client 1600×900 at DPI 120.
* `captures/after-a.json` — synthetic `A` key dispatched to the exact HWND
  (WM_KEYDOWN/WM_KEYUP) makes the panel appear with
  `High contrast (…)`, `High contrast: ON — white text on near-black surfaces…`,
  `Reduced motion (…)`, `Reduced motion: OFF — normal spinner animation`.
* `captures/journey2.json` — UIA toggles in the live app: reduced motion
  `OFF → ON — spinners are replaced by static 'Working...' text and animation
  time is 0`, high contrast `ON → OFF — original charcoal/brass palette`, with
  the four 40-life seat panels and status words still present. The foreground
  window before and after every action was unchanged (no focus steal).
* Captures: `captures/normal-1280x720.png`, `captures/highcontrast-reduced-1280x720.png`
  (background `#000000`, 4 412 near-white samples vs 1 422 normal),
  `captures/scale200-1024x640.png`.

## Honest unmet / unverified items

* **Input response < 100 ms is NOT measured.** The intended synthetic-key →
  UIA-state-change timer timed out on its first sample in the last instance
  (`input-response.stdout`), so no latency number is recorded and the target is
  **unverified**, not met.
* **The delayed-work condition was not established.** Two instrumented runs
  completed; the second carried a `--performance-note`, but the UIA step that
  would have held the worker never succeeded, so its report was deleted rather
  than published with a misleading disclosure. There is therefore **no measured
  p95 under delayed AI/network work** — the frame-time figure above is an
  idle-worker measurement.
* No GPU present time, compositor latency, screen-reader pass, fractional
  DPI other than the observed 1.25/2.0, or physical keyboard run. Vision
  inspection of the captures failed in this environment (tool error), so the
  screenshots are pixel-sampled evidence, not a human visual review.
* Bounded 40-life / 20-life practice only; this says nothing about 100-card
  Commander support (#17) or #25's model-boundary diagnostics.

## Reproduce

```
cargo build --release -p aperture-ui --locked
./target/release/aperture.exe --performance-report evidence/issue22/perf-baseline.json \
    --seats 4 --profile bounded --size 1280x720
python evidence/issue22/launch-native.py --seats 4 --profile bounded --size 1280x720
powershell -File evidence/issue22/native-input.ps1 -WindowHandle <HWND> -ExpectedPid <PID> -Action Dump
powershell -File evidence/issue22/native-input.ps1 -WindowHandle <HWND> -ExpectedPid <PID> -Action Key -VirtualKey 65
powershell -File evidence/issue22/journey.ps1 -WindowHandle <HWND> -Out <out.json>
```
