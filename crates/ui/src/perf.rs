//! Opt-in native frame instrumentation (`--performance-report PATH`).
//!
//! Nothing here runs unless the flag is supplied. Samples are kept only in
//! bounded memory (a 10 s window at ~60 Hz), and the filesystem write happens
//! on a background thread after collection ends — never in a frame callback.
//!
//! Measured quantities, and what they are not:
//! * `callback_duration` — wall-clock time spent inside the eframe update
//!   callback on the main thread. This is the CPU-bound render/layout work; it
//!   is not thread CPU accounting and it excludes any GPU presentation.
//! * `inter_callback_interval` — wall-clock time between consecutive callback
//!   starts while a forced ~60 Hz repaint was requested.
//! * `input_frame_render` — the callback duration of frames that had at least
//!   one input event. This is explicitly *not* OS/GPU input-to-photon latency.
//!
//! No GPU present/frame timing is claimed anywhere in the report.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Samples retained at most: 15 s at 60 Hz. The 10 s measurement window
/// finishes well below this, so memory is bounded even if the window is
/// extended by a test.
pub const MAX_SAMPLES: usize = 900;

pub const DEFAULT_WINDOW: Duration = Duration::from_secs(10);

/// One callback sample. `None` interval for the first frame of a run.
#[derive(Clone, Copy, Debug)]
pub struct FrameSample {
    pub callback_duration: Duration,
    pub interval: Option<Duration>,
    pub had_input: bool,
}

/// Declared conditions under which a report was collected. These are written
/// into the report so a synthetic worker delay can never be mistaken for real
/// model/network latency (`synthetic_delays` is a disclosure, not a measurement).
#[derive(Clone, Debug, Default)]
pub struct Conditions {
    pub synthetic_delays: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct WriteState {
    pub status: Option<String>,
    pub done: bool,
}

/// Bounded in-memory collector for one opt-in run.
pub struct PerformanceProbe {
    path: PathBuf,
    window: Duration,
    conditions: Conditions,
    started: Instant,
    last_callback: Option<Instant>,
    samples: Vec<FrameSample>,
    finished: bool,
    write: Arc<Mutex<WriteState>>,
}

impl PerformanceProbe {
    pub fn new(path: PathBuf) -> Self {
        Self::with_window(path, DEFAULT_WINDOW)
    }

    /// Record what was running while the samples were collected. The
    /// conditions are only a disclosure in the report, never a measurement.
    pub fn with_conditions(mut self, conditions: Conditions) -> Self {
        self.conditions = conditions;
        self
    }

    pub fn with_window(path: PathBuf, window: Duration) -> Self {
        PerformanceProbe {
            path,
            window,
            conditions: Conditions::default(),
            started: Instant::now(),
            last_callback: None,
            samples: Vec::with_capacity(64),
            finished: false,
            write: Arc::new(Mutex::new(WriteState::default())),
        }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// True while the bounded window is still being sampled (the frame loop
    /// then requests a ~60 Hz repaint).
    pub fn collecting(&self) -> bool {
        !self.finished
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    pub fn finished(&self) -> bool {
        self.finished
    }

    pub fn status(&self) -> Option<String> {
        self.write
            .lock()
            .ok()
            .and_then(|state| state.status.clone())
    }

    pub fn write_done(&self) -> bool {
        self.write.lock().map(|state| state.done).unwrap_or(true)
    }

    /// Record the start of a callback. `None` is returned when collection has
    /// already finished.
    pub fn callback_started(&mut self, now: Instant) -> Option<Instant> {
        if self.finished {
            return None;
        }
        if self.samples.is_empty() && self.last_callback.is_none() {
            // The window starts with the first real callback, not with app
            // construction / window creation.
            self.started = now;
        }
        Some(now)
    }

    /// Record one completed callback. Returns true exactly once when the
    /// bounded window is complete and `samples` moved out for the writer.
    pub fn callback_finished(&mut self, started: Instant, ended: Instant, had_input: bool) -> bool {
        if self.finished {
            return false;
        }
        let interval = self
            .last_callback
            .map(|previous| started.saturating_duration_since(previous))
            .filter(|interval| *interval > Duration::ZERO);
        self.last_callback = Some(started);
        if self.samples.len() < MAX_SAMPLES {
            self.samples.push(FrameSample {
                callback_duration: ended.saturating_duration_since(started),
                interval,
                had_input,
            });
        }
        if self.started.elapsed() < self.window {
            return false;
        }
        self.finished = true;
        true
    }

    /// Move the samples to a background thread that writes the JSON report.
    /// No filesystem call happens on the calling (frame) thread.
    pub fn finish_and_write(&mut self) {
        let samples = std::mem::take(&mut self.samples);
        let path = self.path.clone();
        let window = self.window;
        let conditions = self.conditions.clone();
        let state = Arc::clone(&self.write);
        if let Ok(mut write) = state.lock() {
            write.status = Some(format!("collecting finished; writing {}", path.display()));
        }
        std::thread::spawn(move || {
            let report = build_report(&samples, window, &conditions);
            let outcome = std::fs::write(&path, report);
            if let Ok(mut write) = state.lock() {
                write.done = true;
                write.status = Some(match outcome {
                    Ok(()) => format!("written {}", path.display()),
                    Err(err) => format!("performance report failed ({}): {err}", path.display()),
                });
            }
        });
    }

    #[cfg(test)]
    fn samples(&self) -> &[FrameSample] {
        &self.samples
    }
}

fn micros(duration: Duration) -> u64 {
    duration.as_micros().min(u64::MAX as u128) as u64
}

/// Nearest-rank percentile over an unsorted slice. `p` in (0.0, 1.0].
pub fn nearest_rank(values: &[u64], p: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = ((p.clamp(0.0, 1.0) * sorted.len() as f64).ceil() as usize)
        .max(1)
        .min(sorted.len());
    Some(sorted[rank - 1])
}

fn stats(values: &[u64]) -> serde_json::Value {
    let sorted_p50 = nearest_rank(values, 0.50);
    serde_json::json!({
        "samples": values.len(),
        "p50": sorted_p50,
        "p95": nearest_rank(values, 0.95),
        "max": values.iter().copied().max(),
        "method": "nearest-rank over bounded in-memory samples",
    })
}

fn build_report(samples: &[FrameSample], window: Duration, conditions: &Conditions) -> String {
    let callback: Vec<u64> = samples
        .iter()
        .map(|sample| micros(sample.callback_duration))
        .collect();
    let intervals: Vec<u64> = samples
        .iter()
        .filter_map(|sample| sample.interval.map(micros))
        .collect();
    let input: Vec<u64> = samples
        .iter()
        .filter(|sample| sample.had_input)
        .map(|sample| micros(sample.callback_duration))
        .collect();
    let generated = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    let report = serde_json::json!({
        "schema": "aperture-ui-performance/1",
        "generated_at_unix": generated,
        "opt_in": "--performance-report",
        "window_seconds": window.as_secs_f64(),
        "forced_repaint_hz": 60,
        "samples": samples.len(),
        "callback_duration_us": stats(&callback),
        "inter_callback_interval_us": stats(&intervals),
        "input_frame_render_us": {
            "samples": input.len(),
            "p95": nearest_rank(&input, 0.95),
            "max": input.iter().copied().max(),
            "limited": true,
            "meaning": "callback duration on frames that had at least one input event; NOT OS/GPU input-to-photon latency",
        },
        "synthetic_delays": conditions.synthetic_delays,
        "declared_conditions": conditions.notes,
        "method": [
            "samples are collected in memory only inside the eframe update callback",
            "the report is serialized and written by a background thread after collection",
            "no disk or network access happens in a frame callback",
            "no GPU present or frame-present timing is measured or implied",
        ],
        "limitations": [
            "callback_duration is main-thread wall time, not thread CPU accounting",
            "inter_callback_interval depends on the platform event loop and forced repaint",
            "input_frame_render is not an end-to-end input latency measurement",
            "no synthetic worker delay is injected by this native probe; worker tests use injectable hooks",
        ],
    });
    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_p95_uses_the_smallest_value_covering_95_percent() {
        let values: Vec<u64> = (1..=100).collect();
        assert_eq!(nearest_rank(&values, 0.95), Some(95));
        assert_eq!(nearest_rank(&values, 0.50), Some(50));
        assert_eq!(nearest_rank(&[7], 0.95), Some(7));
        assert_eq!(nearest_rank(&[], 0.95), None);
        assert_eq!(nearest_rank(&[9, 1, 5], 1.0), Some(9));
    }

    #[test]
    fn samples_are_bounded_and_the_window_finishes_once() {
        let probe_path =
            std::env::temp_dir().join(format!("aperture-perf-bounded-{}.json", std::process::id()));
        let mut probe = PerformanceProbe::with_window(probe_path, Duration::from_secs(60));
        let base = Instant::now();
        for index in 0..(MAX_SAMPLES + 50) {
            let started = base + Duration::from_micros(index as u64 * 16_000);
            probe.callback_started(started);
            assert!(!probe.callback_finished(started, started + Duration::from_micros(500), false));
        }
        assert_eq!(probe.samples().len(), MAX_SAMPLES);
    }
}
