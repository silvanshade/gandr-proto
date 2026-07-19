//! Documentation-domain gates retained from the project gate suite.
//!
//! The submodules keep manifest drift, reference integrity, page-balance, and
//! rumdl conflict-marker behavior as typed Rust data paths. CLI wiring lives
//! elsewhere; this module intentionally owns only reusable domain operations.

/// External documentation command probes and wrappers.
pub mod commands;
/// Shared documentation manifest and drift verification model.
pub mod manifest;
/// Markdown reference-integrity analysis over registered manifest nodes.
pub mod references;
