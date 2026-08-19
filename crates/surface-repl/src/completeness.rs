//! Parse-completeness for the read-evaluate loop.
//!
//! The validator asks the landed parser whether more tokens are still
//! expected. It does not implement a parser. Holes are typeable, so a
//! hole-bearing buffer can still be complete.

use std::sync::LazyLock;

use gandr_surface_grammar::Pbg;
use gandr_surface_grammar::PbgError;
use gandr_surface_grammar::built_in;
use gandr_surface_parser::CompletionStatus;
use gandr_surface_parser::MeldState;
use gandr_surface_parser::Molder;
use gandr_surface_parser::SourceText;
use gandr_surface_parser::label;
use gandr_surface_syntax::SourceSlice;

/// Why a completeness query could not run.
#[derive(Debug, thiserror::Error)]
pub enum CompletenessError
{
    /// The built-in grammar could not be constructed.
    #[error("built-in grammar failed: {0}")]
    Grammar(GrammarFailure),
}

/// A grammar-construction failure rendered as text so the cache can stay
/// shared.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{0}")]
pub struct GrammarFailure(String);

impl From<&PbgError> for GrammarFailure
{
    #[inline]
    fn from(error: &PbgError) -> Self
    {
        Self(error.to_string())
    }
}

/// The process-wide built-in grammar used by the validator.
static GRAMMAR: LazyLock<Result<Pbg, PbgError>> = LazyLock::new(built_in);

/// Return the cached built-in grammar.
///
/// # Contract
/// - ensures: returns the same grammar for the life of the process.
/// - provides: the PBG the validator molds against.
/// - fails: returns [`CompletenessError::Grammar`] when the grammar cannot be
///   built.
/// - panics: none.
///
/// # Errors
///
/// Returns [`CompletenessError::Grammar`] when [`Pbg`] construction fails.
#[inline]
pub fn grammar() -> Result<&'static Pbg, CompletenessError>
{
    GRAMMAR
        .as_ref()
        .map_err(|error| CompletenessError::Grammar(GrammarFailure::from(error)))
}

/// Ask whether `source` is parse-complete.
///
/// Completeness is "no further token is expected", not "the parse was
/// clean" and not "the term has no holes".
///
/// # Contract
/// - ensures: returns the parser's [`CompletionStatus`] for `source`.
/// - provides: the submit gate the loop consults.
/// - fails: returns [`CompletenessError`] when the grammar cannot be built.
/// - panics: none.
///
/// # Errors
///
/// Returns [`CompletenessError`] when the built-in grammar cannot be
/// constructed.
///
/// # Adequacy
/// - hypothesis: L3 — an open form and a bare atom separate incomplete from
///   complete; a hole-bearing atom stays complete.
/// - witness: `loop::tests::an_open_form_is_incomplete`
/// - witness: `loop::tests::a_bare_atom_is_complete`
/// - witness: `loop::tests::a_hole_is_complete`
#[inline]
pub fn completeness(source: SourceSlice<'_>) -> Result<CompletionStatus, CompletenessError>
{
    let pbg = grammar()?;
    let mut molder = Molder::new(pbg);
    let mut state = MeldState::new(pbg);
    let tokens = label(source);
    molder.mold_stream(&mut state, &tokens, SourceText::from(source.as_ref()));
    Ok(state.expected().is_complete())
}
