# Issue 22 implementation — responsiveness, accessibility, input ergonomics

Branch: `aperture/issue-22` at `055fde7` (no commit; native owner inspects and
commits). Only the native UI shell was changed. `session.rs`,
`tick_ai`/request/response/generation logic and issue #25's AI protocol were not
modified.

Session transcript: `evidence/issue22/omp-sessions/2026-10-26T07-11-37-577Z_01a1770c-ea69-7000-b45d-1d160eb01aae.jsonl`.

## Changed files

| file | change |
| --- | --- |
| `crates/ui/src/theme.rs` | normal + explicit high-contrast palettes, palette-aware richtext helpers, 2 px focus stroke, reduced-motion `animation_time = 0`, built-in keyboard zoom disabled, per-context palette publication |
| `crates/ui/src/perf.rs` (new) | opt-in bounded frame probe, nearest-rank p95, background JSON writer, unit tests |
| `crates/ui/src/app.rs` | 70%–200% zoom + `100%` reset + Ctrl+0, `Accessibility (A)` panel, Escape ordering, bounded header scroll + fixed controls/status/actions, static `Working...`, palette-aware colors, probe integration, focused renderer tests |
| `crates/ui/src/main.rs` | `--performance-report PATH` parser and help text |
| `crates/ui/src/lib.rs` | exposes `perf` |
| `README.md` | native UI bullet documents accessibility, zoom and the opt-in report |
| `docs/issue22-ergonomics.md` (new) | scope, controls, method, limitations and honest unmeasured targets |
| `evidence/issue22/tdd.log` | retained focused RED then GREEN test log |
| `evidence/issue22/ui-shell.png` | real native smoke capture of the 4-seat bounded shell |

`crates/ui/src/session.rs` is untouched.

## Original RED → GREEN

Red log (retained in `evidence/issue22/tdd.log`): `cargo test -p aperture-ui
--lib viewport_tests --locked -- --nocapture` gave **9 failed / 20 passed**.
Real gaps it exposed:

* fixed status rows (life labels, generic waiting, sandbox taint) were inside the
  new bounded header scroll and disappeared;
* the first frame could exceed the synthetic delay used by the worker test;
* the performance window closed before an interval sample existed;
* a fresh test context saw the normal palette because publication was tied to
  style changes;
* the focused-text fixture dropped focus between passes;
* Ctrl+0 read the global modifier state that the test event did not update.

All nine were fixed in the implementation/test fixture, not weakened.

Green: `cargo test -p aperture-ui --locked` — **75 passed, 0 failed**
(32 library renderer tests, 11 + 6 + 4 + 22 integration tests, 0 doc tests).

## Actual command results

```
cargo fmt -p aperture-ui -- --check                  # 0, clean
cargo test -p aperture-ui --locked                   # 0, 75 passed / 0 failed
cargo clippy -p aperture-ui --all-targets --locked -- -D warnings  # 0, clean
cargo build --release -p aperture-ui --locked        # 0
./target/release/aperture.exe --smoke evidence/issue22/ui-shell.png \
    --size 1280x720 --seats 4 --profile bounded      # 0; real window captured
```

The smoke capture shows the fixed header/accessibility row, fixed `YOUR
ACTIONS` strip, four seat panels, status rows, stack and coaching at the
acceptance viewport. No high-contrast/reduced-motion or 200% smoke capture was
taken; those are asserted by the renderer tests.

## Focused verification coverage

`crates/ui/src/app.rs::viewport_tests` performs real `render_ui` frames:

* `zoom_is_repeatable_caps_at_200_percent_and_has_a_100_percent_reset`
* `accessibility_opens_only_without_text_focus_and_escape_closes_it_first`
* `the_accessibility_panel_states_are_words_and_install_high_contrast_richtext`
* `fixed_controls_stay_visible_at_200_percent_and_high_dpi_minimum_windows`
  (1024×640 and 1280×720 physical at 200%; 2 px/point)
* `a_large_board_and_long_per_seat_reports_stay_renderable_and_reachable`
  (12 permanents per seat; 28-entry refused import report)
* `reduced_motion_uses_static_working_text_without_blocking_worker_polling`
* `the_opt_in_performance_probe_reports_bounded_native_samples`

The delayed-worker test is **synthetic**: it calls the existing injectable
`UiSession::request_ai_decision` with a 500 ms `thread::sleep` and then computes
with the deterministic heuristic. It never contacts a model, network or GPU
service. The comment in the test and the report say so.

## Exact measurement commands

10 s native frame report (opt-in; writes after collection on a background
thread; no disk/network in frame callbacks):

```
cargo build --release -p aperture-ui --locked
./target/release/aperture.exe --performance-report evidence/ui-performance-report.json
```

Inspect with:

```
python -c "import json; d=json.load(open('evidence/ui-performance-report.json')); print(json.dumps({k:d[k] for k in ('samples','callback_duration_us','inter_callback_interval_us','input_frame_render_us','limitations')}, indent=2))"
```

Shortened-window renderer proof:

```
cargo test -p aperture-ui --lib the_opt_in_performance_probe_reports_bounded_native_samples --locked -- --nocapture
```

Synthetic delayed-worker proof:

```
cargo test -p aperture-ui --lib reduced_motion_uses_static_working_text_without_blocking_worker_polling --locked -- --nocapture
```

## Report semantics and limitations

* `callback_duration_us`: main-thread wall time inside the eframe update
  callback; p50/p95/max by nearest rank. Not thread CPU accounting.
* `inter_callback_interval_us`: wall time between callback starts while a
  forced ~60 Hz repaint is requested.
* `input_frame_render_us`: callback duration on frames with input events,
  explicitly `limited: true`; not OS/GPU input-to-photon latency.
* No GPU present/frame-present timing is measured or claimed.
* The report is capped at 900 in-memory samples (15 s at 60 Hz); the default
  window is 10 s and starts at the first callback.
* No real 10 s hardware measurement was made in this slice. No manual keyboard,
  screen-reader or visual-inspection pass was performed. No fractional Windows
  DPI value other than 2.0 is covered. These are recorded as unmeasured targets
  in `docs/issue22-ergonomics.md` for the native owner.
