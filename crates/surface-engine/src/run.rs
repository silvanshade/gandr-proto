//! The one-shot source-program driver: lower → link → prelude-check → host-run.
//!
//! [`run_source`] is the language-level source entry point: it composes the
//! surface engine's strict lowering, linking, and prelude-aware checking with
//! the host-effect capability of [`gandr_runtime_host`]. The runtime stays a
//! capability adapter — it owns the host seam and the canonical signatures,
//! never the source pipeline — so the dependency points one way, engine →
//! runtime, with no cycle.

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
#[non_exhaustive]
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
