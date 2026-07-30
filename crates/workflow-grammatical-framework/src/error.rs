//! The crate error vocabulary.

/// Errors from the `GF` runtime-interop and the `.gfd` reader lanes.
#[derive(Debug, thiserror::Error)]
pub enum GfError
{
    /// Filesystem failure with the path context included by the caller.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// The embedded-Python boundary itself failed (interpreter, import, API
    /// mismatch) — distinct from a PGF-level rejection.
    #[error("python interop: {0}")]
    Python(String),
    /// The PGF runtime rejected the document: `PGFError` (unknown function —
    /// the dangling-reference class) or `TypeError` (ill-typed tree).
    #[error("pgf validation: {0}")]
    Pgf(String),
    /// The `.gfd` reader rejected the surface text.
    #[error("gfd parse: {0}")]
    Parse(String),
}
