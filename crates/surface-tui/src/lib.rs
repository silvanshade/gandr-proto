//! Terminal programming environment over the headless session engine.
//!
//! The TUI is a renderer of session transcripts. It does not parse, lower,
//! type, or mark.

#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::print_stderr))]

pub mod app;
pub mod run;
pub mod view;

pub use app::App;
pub use run::SMOKE_NOTE;
pub use run::TuiError;
pub use run::run;
pub use run::run_smoke;
