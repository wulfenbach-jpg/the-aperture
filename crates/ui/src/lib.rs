//! THE APERTURE library surface for the native UI: the card table, the session
//! state it drives, and the theme. The `aperture` binary is a thin wrapper so
//! the session logic stays testable.

pub mod app;
pub mod perf;
pub mod review;
pub mod session;
pub mod theme;

pub use app::{ApertureApp, Startup};
pub use session::UiSession;
