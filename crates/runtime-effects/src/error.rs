//! The host-side failure type for the shell runtime.
//!
//! A [`ShellError`] is raised only when a native syscall fails *fatally* — a
//! failure the operation's reply type cannot represent (a missing file behind
//! `Fs::read`, a program that will not spawn behind `Exec::exec`, a
//! malformed operation payload). It aborts the run
//! ([`crate::ShellOutcome::HostFailed`]), mirroring the fail-fast `error make`
//! a hand-written nushell gate script uses. A *representable* outcome — a
//! non-zero process exit code, a `stat` of a path that does not exist — is NOT
//! an error: it flows back as an ordinary reply value (`exit_code`, `kind =
//! "missing"`).
//!
//! Payloads are owned [`String`]s (not `std::io::Error`) so the type is
//! `Clone`/`Eq`, letting [`crate::ShellOutcome`] be compared in tests.

use alloc::string::String;

use thiserror::Error;

/// A fatal host-side failure that aborts a shell run.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ShellError
{
    /// A process behind `Exec::exec` could not be spawned (e.g. the program is
    /// not on `PATH`). A process that *runs* and exits non-zero is not this —
    /// that is an ordinary `exit_code` reply.
    #[error("exec: could not spawn `{program}`: {detail}")]
    Spawn
    {
        /// The program that failed to spawn.
        program: String,
        /// The underlying OS error rendering.
        detail: String,
    },

    /// A filesystem operation failed fatally (the reply type has no way to
    /// carry the failure).
    #[error("fs {op}: `{path}`: {detail}")]
    Fs
    {
        /// The operation name (`read`, `write`, `glob`, `mkdir`, `tempdir`,
        /// `ls_files`).
        op: String,
        /// The path (or pattern) the operation was applied to.
        path: String,
        /// The underlying OS error rendering.
        detail: String,
    },

    /// An operation's performed payload did not match the operation's expected
    /// shape (e.g. `exec` performed with a non-record, or `args` holding a
    /// non-string element).
    #[error("{sig}::{op}: malformed payload: {detail}")]
    Payload
    {
        /// The effect signature name the operation belongs to.
        sig: String,
        /// The operation name.
        op: String,
        /// What was expected versus found.
        detail: String,
    },

    /// An internal invariant the runtime does not expect to violate — a defined
    /// host-failure variant kept so a surprising internal condition surfaces as
    /// a [`crate::ShellOutcome::HostFailed`] rather than a panic. Unreachable
    /// in normal operation.
    #[error("internal: {detail}")]
    Internal
    {
        /// What the runtime failed to recognize.
        detail: String,
    },
}
