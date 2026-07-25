//! The parser-agnostic seam for changed-region detection (A2.3;
//! `incremental-pipeline.md` §§2-3).
//!
//! # Why a seam
//!
//! `incremental-pipeline.md`'s incremental loop begins by re-deriving a
//! program's top-level items from a source revision (§2, the cold reparse) and
//! then finding the changed region against the previous revision (§3). This
//! module names the boundary between *producing* those items — a front end's
//! job — and *consuming* them: the changed-region detector and the checkpoint
//! engine ([`crate::checkpoint`]) read only [`Item`] / [`Program`], never a
//! concrete parser.
//!
//! The front end is deliberately unnamed here. The reboot's tree-sitter
//! node-address path is retired (`crate::syntax` recognizes unchanged subtrees
//! by merkle content hash, not parser-carried addresses); should a parser
//! return, it is external tooling and an ordinary implementor of [`ItemSource`]
//! — this crate never depends on one. The real implementation is the surface
//! lane's to supply (it owns lowering); until then [`ItemSource`] plus an
//! in-tree test double (`tests/incremental.rs`) is the current consumer.
//!
//! # What crosses the seam
//!
//! An [`Item`] is one lowered top-level item: an optional definition name, an
//! optional type ascription, and the lowered core [`Term`]. It carries no
//! surface syntax, byte ranges, or parser identity — the checkpoint engine's
//! unchanged-region test is structural equality over exactly this data, so the
//! detection is parser-agnostic by construction. A [`Program`] is the ordered
//! item list of one revision.

use alloc::string::String;
use alloc::vec::Vec;

use crate::syntax::Term;
use crate::types::Ty;

/// One lowered top-level item: the parser-agnostic unit the checkpoint engine
/// aligns, footprints, and types.
///
/// This is the reboot's realization of the incremental pipeline's per-item
/// granularity (`incremental-pipeline.md` §4): top-level items lower
/// independently and are typed against an accumulating context threaded item to
/// item, so an item's identity is its name, its ascription, and its lowered
/// term — the content key the unchanged-region test compares.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
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
#[non_exhaustive]
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
/// structure-editor state — opaque to the engine. The surface lane supplies the
/// real (lowering) implementor; the differential gate's test double is the
/// current one.
pub trait ItemSource
{
    /// The revision representation the front end reads. The engine never
    /// inspects it — only the [`Program`] produced from it.
    type Revision: ?Sized;

    /// Lowers one program revision to its ordered top-level items.
    ///
    /// # Contract
    /// - ensures: returns the revision's items in source order; a total front
    ///   end never fails structurally (out-of-fragment regions lower to holes,
    ///   `incremental-pipeline.md` §7).
    /// - panics: none required of an implementor.
    fn items(
        &self,
        revision: &Self::Revision,
    ) -> Program;
}
