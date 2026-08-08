//! The one-shot source-program driver: lower → link → prelude-check → host-run.
//!
//! [`run_source`] is the language-level source entry point: it composes the
//! surface engine's strict lowering, linking, and prelude-aware checking with
//! the host-effect capability of [`gandr_runtime_host`]. The runtime stays a
//! capability adapter — it owns the host seam and the canonical signatures,
//! never the source pipeline — so the dependency points one way, engine →
//! runtime, with no cycle.
//!
//! [`run_source_file`] is the script face of the same composition: it reads a
//! path and hands the bytes to [`run_source`], reporting the path in its own
//! error so a caller that never opened the file can still say which file
//! failed. The script runner (`gandr <file>`) is its production consumer, and
//! the shebang line a `#!/usr/bin/env gandr` script opens with is grammar
//! trivia, so no caller strips it.

use alloc::string::String;
use std::path::Path;
use std::path::PathBuf;

use gandr_core_checker::checker;
use gandr_core_checker::error::TypeError;
use gandr_runtime_host::ShellOutcome;
use gandr_runtime_host::run_program_with_prelude;

use crate::boundary::PipelineSource;
use crate::link;
use crate::lower;
use crate::prelude_ctx;
use crate::prelude_env;

/// Lowers, links, type-checks, and runs one source program under the host.
///
/// # Errors
///
/// Returns [`RunError`] when lowering, linking, or type-checking the source
/// fails before execution.
///
/// # Contract
/// - ensures: a source with a final runnable item is checked under the same
///   prelude used by [`crate::session::Session`], then executed once with both
///   that prelude and the host-effect handler installed;
/// - fails: malformed source, an invalid runnable-item layout, or an ill-typed
///   linked computation;
/// - panics: none.
#[inline]
pub fn run_source<'source, S>(source: S) -> Result<ShellOutcome, RunError>
where
    S: Into<PipelineSource<'source>>,
{
    let lowered = lower::lower_source(source.into())?;
    let comp = link::link_program(&lowered)?;
    checker::infer_comp(prelude_ctx(), comp.clone())?;
    let prelude = prelude_env();
    Ok(run_program_with_prelude(&comp, prelude.as_bindings()))
}

/// A failure preparing a source program for [`run_source`].
#[derive(Debug, thiserror::Error)]
pub enum RunError
{
    /// The source failed to lower to core CBPV.
    #[error("lowering failed: {0}")]
    Lower(#[from] lower::LowerError),
    /// The source has no final runnable item.
    #[error("no runnable program: the source has no items or only declarations")]
    NoProgram,
    /// The lowered item stream is not a valid runnable source file.
    #[error("program linking failed: {0}")]
    Program(#[source] link::LinkError),
    /// The linked computation is ill-typed and must not reach the host seam.
    #[error("type checking failed: {0}")]
    Type(#[from] TypeError),
}

impl From<link::LinkError> for RunError
{
    #[inline]
    fn from(error: link::LinkError) -> Self
    {
        match error {
            | link::LinkError::NoFinalProgram { named_count: 0 } => Self::NoProgram,
            | other => Self::Program(other),
        }
    }
}

/// Reads a source file and runs it exactly as [`run_source`] runs source text.
///
/// This is the seam the script runner (`gandr <file>`) stands on. The file is
/// read whole — the lowerer consumes a complete source — and the path travels
/// into [`RunFileError::Read`] so a diagnostic can name the file even though
/// the failure surfaced inside the read.
///
/// # Errors
///
/// Returns [`RunFileError::Read`] when the file cannot be read or does not hold
/// UTF-8, and [`RunFileError::Run`] when the source itself fails to lower,
/// link, or type-check.
///
/// # Contract
/// - requires: `path` names a readable UTF-8 gandr source file; a leading `#!…`
///   shebang line is ordinary grammar trivia and needs no stripping.
/// - ensures: on success the file's program has been run once under the same
///   prelude and host-effect handler [`run_source`] installs.
/// - provides: the path-shaped source entry point for script-running faces.
/// - fails: an unreadable or non-UTF-8 file surfaces as [`RunFileError::Read`]
///   carrying the path; every source-level failure surfaces as
///   [`RunFileError::Run`] wrapping the [`RunError`] unchanged.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the read failure, the source failure, and the
///   successful run are separated by three inputs: an absent path, a file whose
///   text has no runnable item, and a file whose program returns a value.
/// - witness: `run::run_source_file_runs_a_script_file`
/// - witness: `run::run_source_file_reports_the_path_of_an_absent_file`
/// - witness: `run::run_source_file_surfaces_a_source_failure_unchanged`
#[inline]
pub fn run_source_file(path: &Path) -> Result<ShellOutcome, RunFileError>
{
    let source = std::fs::read_to_string(path).map_err(|source| RunFileError::Read {
        path: path.to_path_buf(),
        detail: source.to_string(),
    })?;
    let outcome = run_source(source.as_str())?;
    Ok(outcome)
}

/// A failure running the source program held in a file.
#[derive(Debug, thiserror::Error)]
pub enum RunFileError
{
    /// The file could not be read as UTF-8 source text.
    #[error("cannot read `{path}`: {detail}", path = path.display())]
    Read
    {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying host error rendering.
        detail: String,
    },
    /// The file was read, but its source failed before execution.
    #[error(transparent)]
    Run(#[from] RunError),
}
