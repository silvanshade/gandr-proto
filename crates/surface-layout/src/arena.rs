//! The sealed document arena and the node algebra it stores.
//!
//! A document is a directed acyclic graph of nodes held in flat, grow-only
//! stores. Every stored edge points at an earlier node in the same builder, so
//! the graph is acyclic by construction rather than by a check. Identities are
//! dense insertion ordinals: they never move, never recycle, and are not
//! content identities.
//!
//! Sharing is the point of the arena. A subdocument referenced twice is stored
//! once and resolved once per distinct layout context, which is what keeps a
//! large printed term from re-walking its own shape.
//!
//! # The node semantics
//!
//! | node       | semantics                                                                                                          |
//! | ---------- | ------------------------------------------------------------------------------------------------------------------ |
//! | `Empty`    | Emits nothing and changes neither column nor indentation.                                                           |
//! | `Text`     | Emits its newline-free string at the current column.                                                                |
//! | `Verbatim` | Emits its exact stored bytes; later physical lines begin at column zero with whatever indentation the bytes carry.  |
//! | `Line`     | Emits the configured physical ending and the current indentation; under flattening it emits one space.              |
//! | `HardLine` | Emits the ending and the indentation even under flattening, which is what protects a line comment.                  |
//! | `Concat`   | Unaligned concatenation: resolve the left, then the right at the left's ending column, keeping the indentation.     |
//! | `Nest`     | Resolves its child with the indentation raised by a checked amount.                                                 |
//! | `Align`    | Resolves its child with the indentation set to the current column.                                                  |
//! | `Choice`   | Admits every layout of either child and merges their measure sets.                                                  |
//! | `Flatten`  | Uses the memoized flattened image of its child, turning `Line` into one space and leaving `HardLine` and verbatim.  |
//!
//! Unaligned concatenation is the feature that makes the resolver's second cost
//! dimension necessary: a locally more expensive left layout can leave a column
//! that makes everything after it cheaper.
//!
//! # Physical text
//!
//! Text is newline-free and rejects a carriage return, a line feed, and a tab;
//! a client expands its own tabs before construction. Verbatim text is the
//! separate opaque carrier for content that must survive byte-identical. Its
//! scan produces one record per physical fragment — including the empty final
//! fragment after a trailing ending — and each record stores that fragment's
//! checked scalar width and the exact ending that follows it, so the stored
//! bytes and the stored metrics cannot disagree. Verbatim text is inert to
//! flattening: neither a `Flatten` node nor a surrounding group may rewrite its
//! newlines or its internal indentation.
//!
//! The first fragment extends the incoming column. After a stored ending, every
//! later fragment starts at absolute column zero, so middle widths are absolute
//! line widths and the ending column is the final fragment's width.
//!
//! # What slice one owns here
//!
//! The types below are data and are complete. The arena's own operations need
//! bodies, so their exact intended signatures are stated here:
//!
//! ```text
//! impl DocArena {
//!     pub fn node_count(&self) -> DocNodesUsed;
//!     pub fn contains(&self, doc: DocId) -> DocHandleStatus;
//! }
//! ```
//!
//! `DocHandleStatus` is a two-valued nominal enum rather than a `bool`, and it
//! is slice one's to introduce beside the impl.
//!
//! Identity minting, verbatim scanning, and the flatten interner are private to
//! [`crate::build`]; nothing here hands out a raw ordinal.

use core::num::NonZeroU32;

/// A stable dense document-arena identity.
///
/// # Contract
/// - requires: the value was minted by the builder that owns the node.
/// - ensures: the identity never moves and is never recycled.
/// - provides: the identity stored in every document edge.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct NodeId
{
    /// The dense insertion ordinal.
    index: u32,
}

/// A stable dense text-arena identity.
///
/// # Contract
/// - requires: the value was minted by the builder that owns the text.
/// - ensures: the identity never moves and is never recycled.
/// - provides: the payload of a text node.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct TextId
{
    /// The dense insertion ordinal.
    index: u32,
}

/// A stable dense verbatim-arena identity.
///
/// # Contract
/// - requires: the value was minted by the builder that owns the text.
/// - ensures: the identity never moves and is never recycled.
/// - provides: the payload of a verbatim node.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct VerbatimId
{
    /// The dense insertion ordinal.
    index: u32,
}

/// A process-local token distinguishing one arena from every other.
///
/// The key is what makes a handle checkable. Without it a dense ordinal from
/// one document would silently name a different node in another, and the
/// mistake would surface as wrong output rather than as an error.
///
/// # Contract
/// - requires: the value was minted by the crate's checked monotonic counter.
/// - ensures: two live arenas never share a key.
/// - provides: the arena half of a client-facing document handle.
/// - fails: minting reports `BuildError::ArenaKeyExhausted` when the counter
///   has no value left.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct ArenaKey
{
    /// The non-zero process-local token.
    token: NonZeroU32,
}

/// A checked client handle to a document node.
///
/// The fields are private on purpose: a handle is presented to the arena that
/// minted it and validated there, and a client never constructs one from parts.
///
/// # Contract
/// - requires: the handle is presented to the arena whose key it carries.
/// - ensures: a foreign or out-of-range handle is rejected before any lookup.
/// - provides: the only way a client names a node.
/// - fails: a mismatch surfaces as `BuildError::UnknownDoc` during
///   construction, and as the render-phase unknown-handle error afterwards.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DocId
{
    /// The arena this identity belongs to.
    arena: ArenaKey,
    /// The node named within that arena.
    node: NodeId,
}

/// A physical line ending recorded inside verbatim text.
///
/// # Contract
/// - requires: the value is what the verbatim scan actually found.
/// - ensures: the recorded ending reproduces the original bytes exactly.
/// - provides: the ending half of a verbatim fragment record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    dead_code,
    reason = "slice one reads these; the expectation fails as soon as it does"
)]
pub(crate) enum StoredLineEnding
{
    /// A single line feed.
    Lf,
    /// A carriage return followed by a line feed.
    CrLf,
}

/// One physical fragment of verbatim text, with its width and its ending.
///
/// # Contract
/// - requires: the record comes from the scan of the bytes it describes.
/// - ensures: the width is a checked scalar count and the ending is exactly
///   what follows the fragment, or none at the end of the text.
/// - provides: the metrics the cost and taint rules read without re-scanning
///   the bytes.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct VerbatimLine
{
    /// The fragment's checked scalar width.
    scalar_width: u32,
    /// The ending that follows the fragment, if any.
    ending: Option<StoredLineEnding>,
}

/// Verbatim bytes stored once, beside the fragment records describing them.
///
/// # Contract
/// - requires: the records were produced by scanning exactly these bytes.
/// - ensures: bytes and metrics cannot disagree, because neither is derived a
///   second time.
/// - provides: the byte-identical carrier for comments and other protected
///   content.
/// - panics: none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerbatimText
{
    /// The original bytes, stored exactly once.
    bytes: String,
    /// One record per physical fragment.
    lines: Vec<VerbatimLine>,
}

/// One stored document node.
///
/// # Contract
/// - requires: every identity a variant carries names an earlier node in the
///   same arena.
/// - ensures: the store is a directed acyclic graph by construction.
/// - provides: the complete document algebra, including arbitrary choice and
///   unaligned concatenation.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    dead_code,
    reason = "slice one reads these; the expectation fails as soon as it does"
)]
pub(crate) enum DocNode
{
    /// Emits nothing.
    Empty,
    /// Emits newline-free text.
    Text(TextId),
    /// Emits opaque multiline bytes.
    Verbatim(VerbatimId),
    /// A layout-owned line break, softened to a space by flattening.
    Line,
    /// A line break that survives flattening.
    HardLine,
    /// Unaligned concatenation.
    Concat
    {
        /// Resolved first.
        left: NodeId,
        /// Resolved at the left's ending column.
        right: NodeId,
    },
    /// Raised indentation over a child.
    Nest
    {
        /// The checked amount added to the current indentation.
        amount: u32,
        /// The child resolved under the raised indentation.
        doc: NodeId,
    },
    /// Indentation set to the current column over a child.
    Align
    {
        /// The child resolved under the aligned indentation.
        doc: NodeId,
    },
    /// Arbitrary choice between two children.
    Choice
    {
        /// The left alternative, which wins a tie.
        left: NodeId,
        /// The right alternative.
        right: NodeId,
    },
    /// The flattened image of a child.
    Flatten
    {
        /// The child whose memoized flattened image is used.
        doc: NodeId,
    },
}

/// A finished, immutable, shareable document.
///
/// Finalization has already computed each node's flattened image, so the
/// resolver never needs flattening as a second memo dimension.
///
/// # Contract
/// - requires: the arena came from a builder that finished without refusing.
/// - ensures: identities are stable, the graph is acyclic, and every node has
///   an entry in the flattened image table.
/// - provides: the immutable input the resolver and the renderer read.
/// - panics: none.
#[derive(Clone, Debug)]
#[expect(
    dead_code,
    reason = "slice one reads these; the expectation fails as soon as it does"
)]
pub struct DocArena
{
    /// The arena this identity belongs to.
    arena: ArenaKey,
    /// The document node store.
    nodes: Vec<DocNode>,
    /// The text store.
    texts: Vec<String>,
    /// The verbatim store.
    verbatim: Vec<VerbatimText>,
    /// Each node's flattened image, computed at finalization.
    flattened: Vec<NodeId>,
}
