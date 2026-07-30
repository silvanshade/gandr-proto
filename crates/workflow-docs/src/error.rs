//! The crate error vocabulary.

/// Errors from the migration, runtime-interop, and rendering lanes.
#[derive(Debug, thiserror::Error)]
pub enum GfDocsError
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
    /// The legacy XML parser reported a structural error.
    #[error("legacy model: {0}")]
    Model(String),
    /// The model-to-tree translation hit a construct outside the `PoC` grammar.
    #[error("translation: {0}")]
    Translation(String),
    /// The `.gfd` reader rejected the surface text.
    #[error("gfd parse: {0}")]
    Parse(String),
}

/// Map a runtime-lane rejection into the crate error vocabulary.
impl From<gandr_workflow_grammatical_framework::GfError> for GfDocsError
{
    #[inline]
    fn from(error: gandr_workflow_grammatical_framework::GfError) -> Self
    {
        use gandr_workflow_grammatical_framework::GfError as Gf;
        match error {
            | Gf::Io(source) => Self::Io(source),
            | Gf::Python(message) => Self::Python(message),
            | Gf::Pgf(message) => Self::Pgf(message),
            | Gf::Parse(message) => Self::Parse(message),
            | _ => Self::Model("unknown GF interop error".into()),
        }
    }
}
