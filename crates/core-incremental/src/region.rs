//! The parser-agnostic seam for changed-region detection
//! (`incremental-pipeline.md` §"Cold reparse" and §"The structural diff").
//!
//! # Why a seam
//!
//! `incremental-pipeline.md`'s incremental loop begins by re-deriving a
//! program's top-level items from a source revision (§"Cold reparse") and
//! then finding the changed region against the previous revision (§"The
//! structural diff"). This module names the boundary between *producing* those
//! items — a front end's job — and *consuming* them: the changed-region
//! detector and the checkpoint engine ([`crate::checkpoint`]) read only
//! [`Item`] / [`Program`], never a concrete parser.
//!
//! The front end is deliberately unnamed here. Unchanged subtrees are
//! recognized by merkle content hash rather than by parser-carried node
//! addresses ([`Term`] is span-free and parser-free), so a parser is external
//! tooling and an ordinary implementor of [`ItemSource`] — this crate names
//! none and depends on none. `gandr-surface-engine`'s item-source module
//! supplies the melder-and-lowering front end; the in-tree test double in this
//! crate's differential gate drives the engine without one.
//!
//! # What crosses the seam
//!
//! An [`Item`] is one lowered top-level item: an optional definition name, an
//! optional type ascription, and the lowered core [`Term`]. It carries no
//! surface syntax, byte ranges, or parser identity — the checkpoint engine's
//! unchanged-region test is structural equality over exactly this data, so the
//! detection is parser-agnostic by construction. A [`Program`] is the ordered
//! item list of one revision.
//!
//! [`Term`]: gandr_core_term::syntax::Term

use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::syntax::Term;
use gandr_core_term::types::Ty;

/// One lowered top-level item: the parser-agnostic unit the checkpoint engine
/// aligns, footprints, and types.
///
/// This is the incremental pipeline's per-item granularity
/// (`incremental-pipeline.md` §"Checkpoints and the reuse rule"): top-level
/// items lower independently and are typed against an accumulating context
/// threaded item to item, so an item's identity is its name, its ascription,
/// and its lowered term — the content key the unchanged-region test compares.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Item
{
    /// The defined name (`def` items); [`None`] for an expression item.
    pub name: Option<String>,
    /// The recorded ascription: an explicit signature or the type a definition
    /// sugar derived, when the front end supplies one.
    pub ascription: Option<Ty>,
    /// The lowered core term — the content key for the unchanged-region test.
    pub term: Term,
}

impl Item
{
    /// Builds a lowered item from its name, ascription, and term.
    ///
    /// # Contract
    /// - ensures: returns the item verbatim; the checkpoint engine, not this
    ///   constructor, decides typing and reuse.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        name: Option<String>,
        ascription: Option<Ty>,
        term: Term,
    ) -> Self
    {
        Self {
            name,
            ascription,
            term,
        }
    }
}

/// The ordered top-level items of one program revision — everything the
/// changed-region detector and checkpoint engine read about a revision.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Program
{
    /// The lowered items, in source order.
    pub items: Vec<Item>,
}

impl Program
{
    /// Builds a program from its ordered items.
    ///
    /// # Contract
    /// - ensures: returns the items verbatim, in the given order.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(items: Vec<Item>) -> Self
    {
        Self { items }
    }
}

impl FromIterator<Item> for Program
{
    #[inline]
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Item>,
    {
        Self {
            items: iter.into_iter().collect(),
        }
    }
}

/// The parser-agnostic seam: a front end that lowers a source revision to its
/// ordered top-level items.
///
/// The checkpoint engine ([`crate::checkpoint`]) consumes only the [`Program`]
/// this yields, so it depends on no concrete parser. [`Self::Revision`] is the
/// front end's own revision representation — source text, an edit script, a
/// structure-editor state — opaque to the engine.
///
/// # Why the result is fallible
///
/// A *total* front end recovers every input-dependent failure into the program
/// it returns: an out-of-fragment construct or an error region lowers to a hole
/// rather than aborting (`incremental-pipeline.md` §"Holes"). Totality in that
/// sense is a property of how the front end treats its **input**, and it leaves
/// a residue that no input recovers from — the front end being unable to run at
/// all, because its grammar, table, or parser is unavailable. Those failures do
/// not name a region, so they cannot become one, and a seam that could not
/// report them would force every implementor to swallow or panic on them.
/// [`Self::Error`] is that residue, and an implementor that genuinely cannot
/// fail names [`core::convert::Infallible`].
pub trait ItemSource
{
    /// The revision representation the front end reads. The engine never
    /// inspects it — only the [`Program`] produced from it.
    type Revision: ?Sized;

    /// The failure a front end reports when it cannot produce items at all.
    ///
    /// This is the input-independent residue described on the trait, never an
    /// input-dependent lowering failure: those recover into holes. A front end
    /// with no such residue names [`core::convert::Infallible`].
    type Error;

    /// Lowers one program revision to its ordered top-level items.
    ///
    /// # Contract
    /// - ensures: returns the revision's items in source order; a total front
    ///   end recovers every input-dependent failure into a hole rather than an
    ///   error (`incremental-pipeline.md` §"Holes").
    /// - fails: only where the front end cannot run at all — see
    ///   [`Self::Error`].
    /// - panics: none required of an implementor.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] when the front end cannot produce items for any
    /// input, never for a construct it merely cannot lower.
    fn items(
        &self,
        revision: &Self::Revision,
    ) -> Result<Program, Self::Error>;
}
