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

use crate::error::BuildAllocationSite;
use crate::error::BuildArithmetic;
use crate::error::BuildError;
use crate::units::DocNodesUsed;
use crate::units::ScalarWidth;
use crate::units::TextBytesUsed;
use crate::units::VerbatimLinesUsed;

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

/// The result of checking a client document handle against an arena.
///
/// # Contract
/// - requires: the status came from [`DocArena::contains`].
/// - ensures: `Present` means the arena key and dense node ordinal are valid;
///   `Absent` means at least one check failed.
/// - provides: a nominal two-valued handle status without exposing `bool`.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DocHandleStatus
{
    /// The handle belongs to this arena and names a stored node.
    Present,
    /// The handle is foreign or outside this arena's node store.
    Absent,
}

/// Borrowed newline-free text destined for a `Text` node.
///
/// The wrapper is owned by this module because this module validates and stores
/// its bytes; the raw borrow never crosses the arena boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TextSource<'source>
{
    /// The borrowed text.
    text: &'source str,
}

/// Owned newline-free text destined for a `Text` node.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TextOwned
{
    /// The owned text.
    text: String,
}

/// Borrowed opaque multiline text destined for a `Verbatim` node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VerbatimSource<'source>
{
    /// The borrowed opaque bytes.
    text: &'source str,
}

/// Owned opaque multiline text destined for a `Verbatim` node.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VerbatimOwned
{
    /// The owned opaque bytes.
    text: String,
}

/// A validated, owned newline-free text identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct CheckedText
{
    /// The owned newline-free bytes.
    text: String,
    /// The checked scalar width.
    width: ScalarWidth,
}

impl CheckedText
{
    /// Returns the nominal byte charge for this text identity.
    #[inline]
    pub(crate) fn bytes_used(&self) -> Result<TextBytesUsed, BuildError>
    {
        TextBytesUsed::try_from(self.text.len())
    }

    /// Returns the checked scalar width carried by this identity.
    #[inline]
    pub(crate) fn width(&self) -> ScalarWidth
    {
        self.width
    }
}
/// A physical line ending recorded inside verbatim text.
///
/// # Contract
/// - requires: the value is what the verbatim scan actually found.
/// - ensures: the recorded ending reproduces the original bytes exactly.
/// - provides: the ending half of a verbatim fragment record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum StoredLineEnding
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
pub struct VerbatimLine
{
    /// The fragment's checked scalar width.
    scalar_width: ScalarWidth,
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

impl VerbatimLine
{
    /// Returns this fragment's nominal scalar width.
    #[inline]
    #[must_use]
    pub fn scalar_width(&self) -> ScalarWidth
    {
        self.scalar_width
    }

    /// Returns the exact ending following this fragment, if any.
    #[inline]
    #[must_use]
    pub fn ending(&self) -> Option<StoredLineEnding>
    {
        self.ending
    }
}

impl VerbatimText
{
    /// Returns the nominal byte charge for this identity.
    #[inline]
    pub(crate) fn bytes_used(&self) -> Result<TextBytesUsed, BuildError>
    {
        TextBytesUsed::try_from(self.bytes.len())
    }

    /// Returns the nominal physical-fragment charge for this identity.
    #[inline]
    pub(crate) fn lines_used(&self) -> Result<VerbatimLinesUsed, BuildError>
    {
        VerbatimLinesUsed::try_from(self.lines.len())
    }
}

impl<'source> From<&'source str> for TextSource<'source>
{
    #[inline]
    fn from(text: &'source str) -> Self
    {
        Self { text }
    }
}

impl From<String> for TextOwned
{
    #[inline]
    fn from(text: String) -> Self
    {
        Self { text }
    }
}

impl<'source> From<&'source str> for VerbatimSource<'source>
{
    #[inline]
    fn from(text: &'source str) -> Self
    {
        Self { text }
    }
}

impl From<String> for VerbatimOwned
{
    #[inline]
    fn from(text: String) -> Self
    {
        Self { text }
    }
}

impl<'source> TryFrom<TextSource<'source>> for CheckedText
{
    type Error = BuildError;

    #[inline]
    fn try_from(source: TextSource<'source>) -> Result<Self, Self::Error>
    {
        let width = checked_text_width(source.text)?;
        let mut text = String::new();
        text.try_reserve(source.text.len())
            .map_err(|_error| BuildError::AllocationFailed {
                site: BuildAllocationSite::TextArena,
            })?;
        text.push_str(source.text);
        Ok(Self { text, width })
    }
}

impl TryFrom<TextOwned> for CheckedText
{
    type Error = BuildError;

    #[inline]
    fn try_from(source: TextOwned) -> Result<Self, Self::Error>
    {
        let TextOwned { text } = source;
        let width = checked_text_width(&text)?;
        Ok(Self { text, width })
    }
}

impl<'source> TryFrom<VerbatimSource<'source>> for VerbatimText
{
    type Error = BuildError;

    #[inline]
    fn try_from(source: VerbatimSource<'source>) -> Result<Self, Self::Error>
    {
        let lines = scan_verbatim(source.text)?;
        let mut bytes = String::new();
        bytes
            .try_reserve(source.text.len())
            .map_err(|_error| BuildError::AllocationFailed {
                site: BuildAllocationSite::TextArena,
            })?;
        bytes.push_str(source.text);
        Ok(Self { bytes, lines })
    }
}

impl TryFrom<VerbatimOwned> for VerbatimText
{
    type Error = BuildError;

    #[inline]
    fn try_from(source: VerbatimOwned) -> Result<Self, Self::Error>
    {
        let VerbatimOwned { text } = source;
        let lines = scan_verbatim(&text)?;
        Ok(Self { bytes: text, lines })
    }
}

/// Validates newline-free text and returns its checked scalar width.
fn checked_text_width(text: &str) -> Result<ScalarWidth, BuildError>
{
    if text
        .chars()
        .any(|character| matches!(character, '\r' | '\n' | '\t'))
    {
        return Err(BuildError::InvalidText);
    }
    ScalarWidth::try_from(text.chars().count())
}

/// Scans LF and CRLF text into nominal physical-fragment records.
fn scan_verbatim(text: &str) -> Result<Vec<VerbatimLine>, BuildError>
{
    let mut lines = Vec::new();
    let mut chars = text.chars();
    let mut width = 0u32;
    while let Some(character) = chars.next() {
        let ending = match character {
            | '\n' => Some(StoredLineEnding::Lf),
            | '\r' => match chars.next() {
                | Some('\n') => Some(StoredLineEnding::CrLf),
                | Some(_) | None => return Err(BuildError::InvalidVerbatimLineEnding),
            },
            | _ => {
                width = width
                    .checked_add(1u32)
                    .ok_or(BuildError::ArithmeticOverflow {
                        operation: BuildArithmetic::VerbatimLines,
                    })?;
                None
            },
        };
        if let Some(ending) = ending {
            lines
                .try_reserve(1usize)
                .map_err(|_error| BuildError::AllocationFailed {
                    site: BuildAllocationSite::VerbatimArena,
                })?;
            lines.push(VerbatimLine {
                scalar_width: ScalarWidth::from(width),
                ending: Some(ending),
            });
            width = 0u32;
        }
    }
    lines
        .try_reserve(1usize)
        .map_err(|_error| BuildError::AllocationFailed {
            site: BuildAllocationSite::VerbatimArena,
        })?;
    lines.push(VerbatimLine {
        scalar_width: ScalarWidth::from(width),
        ending: None,
    });
    Ok(lines)
}

/// A stored document node.
///
/// # Contract
/// - requires: every identity a variant carries names an earlier node in the
///   same arena.
/// - ensures: the store is a directed acyclic graph by construction.
/// - provides: the complete document algebra, including arbitrary choice and
///   unaligned concatenation.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
pub struct DocArena
{
    /// The arena this identity belongs to.
    arena: ArenaKey,
    /// The document node store.
    nodes: Vec<DocNode>,
    /// The text store.
    texts: Vec<CheckedText>,
    /// The verbatim store.
    verbatim: Vec<VerbatimText>,
    /// Each node's flattened image, computed at finalization.
    flattened: Vec<NodeId>,
}

impl DocArena
{
    /// Returns the number of stored document nodes, including flattened images.
    ///
    /// # Contract
    /// - requires: the arena was sealed by a successful builder finalization.
    /// - ensures: the result equals the arena's dense node-store length.
    /// - provides: the public storage observation used by build accounting.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn node_count(&self) -> DocNodesUsed
    {
        let nodes = u64::try_from(self.nodes.len()).map_or(u64::MAX, |nodes| nodes);
        DocNodesUsed::from(nodes)
    }

    /// Returns the stored newline-free text for a text handle.
    ///
    /// # Contract
    /// - requires: `doc` names a stored `Text` node in this arena.
    /// - ensures: the returned nominal value contains the exact stored bytes.
    /// - provides: a read-only semantic projection for tests and renderers.
    /// # Errors
    /// Returns `UnknownDoc` when the handle is foreign, invalid, or names a
    /// non-text node.
    /// - panics: none.
    #[inline]
    pub fn stored_text(
        &self,
        doc: DocId,
    ) -> Result<TextOwned, BuildError>
    {
        let node = self.node_id_for(doc)?;
        let index = usize::try_from(u32::from(node)).map_err(|_error| BuildError::UnknownDoc)?;
        let Some(DocNode::Text(text)) = self.nodes.get(index).copied()
        else {
            return Err(BuildError::UnknownDoc);
        };
        let index = usize::try_from(u32::from(text)).map_err(|_error| BuildError::UnknownDoc)?;
        self.texts
            .get(index)
            .map(|text| TextOwned::from(text.text.clone()))
            .ok_or(BuildError::UnknownDoc)
    }

    /// Returns the checked scalar width stored beside a text identity.
    ///
    /// # Contract
    /// - requires: `doc` names a stored `Text` node in this arena.
    /// - ensures: the result is the width computed during ingestion.
    /// - provides: a nominal width projection without exposing raw bytes.
    /// # Errors
    /// Returns `UnknownDoc` for a foreign, invalid, or non-text handle.
    /// - panics: none.
    #[inline]
    pub fn stored_text_width(
        &self,
        doc: DocId,
    ) -> Result<ScalarWidth, BuildError>
    {
        let node = self.node_id_for(doc)?;
        let index = usize::try_from(u32::from(node)).map_err(|_error| BuildError::UnknownDoc)?;
        let Some(DocNode::Text(text)) = self.nodes.get(index).copied()
        else {
            return Err(BuildError::UnknownDoc);
        };
        let index = usize::try_from(u32::from(text)).map_err(|_error| BuildError::UnknownDoc)?;
        self.texts
            .get(index)
            .map(CheckedText::width)
            .ok_or(BuildError::UnknownDoc)
    }

    /// Returns the stored verbatim bytes for a verbatim handle.
    ///
    /// # Contract
    /// - requires: `doc` names a stored `Verbatim` node in this arena.
    /// - ensures: the returned nominal value contains the exact stored bytes.
    /// - provides: a read-only byte-identity projection.
    /// # Errors
    /// Returns `UnknownDoc` when the handle is foreign, invalid, or names a
    /// non-verbatim node.
    /// - panics: none.
    #[inline]
    pub fn stored_verbatim(
        &self,
        doc: DocId,
    ) -> Result<VerbatimOwned, BuildError>
    {
        let node = self.node_id_for(doc)?;
        let index = usize::try_from(u32::from(node)).map_err(|_error| BuildError::UnknownDoc)?;
        let Some(DocNode::Verbatim(verbatim)) = self.nodes.get(index).copied()
        else {
            return Err(BuildError::UnknownDoc);
        };
        let index =
            usize::try_from(u32::from(verbatim)).map_err(|_error| BuildError::UnknownDoc)?;
        self.verbatim
            .get(index)
            .map(|verbatim| VerbatimOwned::from(verbatim.bytes.clone()))
            .ok_or(BuildError::UnknownDoc)
    }

    /// Returns the stored fragment records for a verbatim handle.
    ///
    /// # Contract
    /// - requires: `doc` names a stored `Verbatim` node in this arena.
    /// - ensures: widths and endings are the records produced by ingestion.
    /// - provides: a read-only nominal metric projection.
    /// # Errors
    /// Returns `UnknownDoc` when the handle is foreign, invalid, or names a
    /// non-verbatim node.
    /// - panics: none.
    #[inline]
    pub fn verbatim_lines(
        &self,
        doc: DocId,
    ) -> Result<Vec<VerbatimLine>, BuildError>
    {
        let node = self.node_id_for(doc)?;
        let index = usize::try_from(u32::from(node)).map_err(|_error| BuildError::UnknownDoc)?;
        let Some(DocNode::Verbatim(verbatim)) = self.nodes.get(index).copied()
        else {
            return Err(BuildError::UnknownDoc);
        };
        let index =
            usize::try_from(u32::from(verbatim)).map_err(|_error| BuildError::UnknownDoc)?;
        self.verbatim
            .get(index)
            .map(|verbatim| verbatim.lines.clone())
            .ok_or(BuildError::UnknownDoc)
    }

    /// Returns the finalized flattened image of a document handle.
    ///
    /// # Contract
    /// - requires: `doc` names any stored node in this arena.
    /// - ensures: the returned handle names the memoized flattened image.
    /// - provides: a read-only finalization projection for identity checks.
    /// # Errors
    /// Returns `UnknownDoc` when the handle is foreign or invalid.
    /// - panics: none.
    #[inline]
    pub fn flattened_image(
        &self,
        doc: DocId,
    ) -> Result<DocId, BuildError>
    {
        let node = self.node_id_for(doc)?;
        let index = usize::try_from(u32::from(node)).map_err(|_error| BuildError::UnknownDoc)?;
        self.flattened
            .get(index)
            .copied()
            .map(|flattened| DocId::from_parts(self.arena, flattened))
            .ok_or(BuildError::UnknownDoc)
    }

    /// Validates a handle and returns its internal node identity.
    ///
    /// # Errors
    /// Returns `UnknownDoc` when the handle is foreign or out of range.
    fn node_id_for(
        &self,
        doc: DocId,
    ) -> Result<NodeId, BuildError>
    {
        let DocHandleStatus::Present = self.contains(doc)
        else {
            return Err(BuildError::UnknownDoc);
        };
        Ok(doc.node_id())
    }

    /// Checks whether `doc` is a valid handle for this arena.
    ///
    /// # Contract
    /// - requires: `doc` is any client handle, including one from another
    ///   arena.
    /// - ensures: foreign and out-of-range handles return `Absent` before
    ///   lookup.
    /// - provides: a non-panicking identity check for callers and later phases.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn contains(
        &self,
        doc: DocId,
    ) -> DocHandleStatus
    {
        let Ok(node_count) = u32::try_from(self.nodes.len())
        else {
            return DocHandleStatus::Absent;
        };
        if self.arena == doc.arena_key() && u32::from(doc.node_id()) < node_count {
            DocHandleStatus::Present
        }
        else {
            DocHandleStatus::Absent
        }
    }
}
impl From<u32> for NodeId
{
    #[inline]
    fn from(index: u32) -> Self
    {
        Self { index }
    }
}

impl From<NodeId> for u32
{
    #[inline]
    fn from(node: NodeId) -> Self
    {
        node.index
    }
}

impl From<u32> for TextId
{
    #[inline]
    fn from(index: u32) -> Self
    {
        Self { index }
    }
}

impl From<TextId> for u32
{
    #[inline]
    fn from(text: TextId) -> Self
    {
        text.index
    }
}

impl From<u32> for VerbatimId
{
    #[inline]
    fn from(index: u32) -> Self
    {
        Self { index }
    }
}

impl From<VerbatimId> for u32
{
    #[inline]
    fn from(verbatim: VerbatimId) -> Self
    {
        verbatim.index
    }
}

impl From<NonZeroU32> for ArenaKey
{
    #[inline]
    fn from(token: NonZeroU32) -> Self
    {
        Self { token }
    }
}

impl From<ArenaKey> for NonZeroU32
{
    #[inline]
    fn from(key: ArenaKey) -> Self
    {
        key.token
    }
}

impl DocId
{
    /// Creates a crate-internal handle from its checked arena and node parts.
    ///
    /// # Contract
    /// - requires: `arena` and `node` were minted by the same builder.
    /// - ensures: the resulting public handle carries both identity components.
    /// - provides: the builder's only internal handle assembly operation.
    /// - panics: none.
    #[inline]
    pub(crate) fn from_parts(
        arena: ArenaKey,
        node: NodeId,
    ) -> Self
    {
        Self { arena, node }
    }

    /// Returns the crate-internal arena component of this handle.
    #[inline]
    pub(crate) fn arena_key(self) -> ArenaKey
    {
        self.arena
    }

    /// Returns the crate-internal dense node component of this handle.
    #[inline]
    pub(crate) fn node_id(self) -> NodeId
    {
        self.node
    }
}

impl From<(ScalarWidth, Option<StoredLineEnding>)> for VerbatimLine
{
    #[inline]
    fn from(parts: (ScalarWidth, Option<StoredLineEnding>)) -> Self
    {
        Self {
            scalar_width: parts.0,
            ending: parts.1,
        }
    }
}

impl From<(String, Vec<VerbatimLine>)> for VerbatimText
{
    #[inline]
    fn from(parts: (String, Vec<VerbatimLine>)) -> Self
    {
        Self {
            bytes: parts.0,
            lines: parts.1,
        }
    }
}

impl DocArena
{
    /// Creates a sealed arena from crate-internal finalized stores.
    ///
    /// # Contract
    /// - requires: `flattened` contains one image entry for every stored node.
    /// - ensures: all finalized stores move into one immutable arena.
    /// - provides: the builder-to-arena ownership boundary.
    /// - panics: none.
    #[inline]
    pub(crate) fn from_parts(
        arena: ArenaKey,
        nodes: Vec<DocNode>,
        texts: Vec<CheckedText>,
        verbatim: Vec<VerbatimText>,
        flattened: Vec<NodeId>,
    ) -> Self
    {
        Self {
            arena,
            nodes,
            texts,
            verbatim,
            flattened,
        }
    }
}
