//! Compact arena model for gandr concrete syntax trees.

use alloc::sync::Arc;

use crate::BuildError;

/// Sentinel stored in [`NodeData::parent`] for the root node.
const NO_PARENT: u32 = u32::MAX;

/// Shared source text owned by a concrete syntax tree.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SourceText(Arc<str>);

impl From<&str> for SourceText
{
    #[inline]
    fn from(value: &str) -> Self
    {
        Self(Arc::from(value))
    }
}

impl From<String> for SourceText
{
    #[inline]
    fn from(value: String) -> Self
    {
        Self(Arc::from(value))
    }
}

impl From<Arc<str>> for SourceText
{
    #[inline]
    fn from(value: Arc<str>) -> Self
    {
        Self(value)
    }
}

impl AsRef<str> for SourceText
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

/// Borrowed source text covered by a node range.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceSlice<'source>(&'source str);

impl<'source> From<&'source str> for SourceSlice<'source>
{
    #[inline]
    fn from(value: &'source str) -> Self
    {
        Self(value)
    }
}

impl AsRef<str> for SourceSlice<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

/// Compact public arena slot carried by [`NodeId`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the arena-slot boundary is a single compact node-slot integer"
)]
pub struct NodeSlot(pub u32);

impl From<NodeSlot> for u32
{
    #[inline]
    fn from(value: NodeSlot) -> Self
    {
        value.0
    }
}

/// Host collection index for a dense arena slot.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeIndex(pub usize);

impl From<NodeIndex> for usize
{
    #[inline]
    fn from(value: NodeIndex) -> Self
    {
        value.0
    }
}

/// Number of nodes stored in a CST arena.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the arena-count boundary is a single host-sized node count"
)]
pub struct NodeCount(pub usize);

impl NodeCount
{
    /// Iterate every compact node slot covered by this count.
    ///
    /// # Contract
    /// - requires: `self` is a host-sized count from a dense CST arena.
    /// - ensures: validates the count fits the compact node-id width before an
    ///   iterator is returned; the iterator then yields exactly one slot per
    ///   representable dense arena position.
    /// - fails: returns [`BuildError::NodeCountOverflow`] when the count
    ///   exceeds the compact arena width.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::NodeCountOverflow`] when the node count cannot be
    /// represented by the compact `u32` slot domain.
    #[inline]
    pub fn slots(
        self
    ) -> Result<impl ExactSizeIterator<Item = NodeSlot> + DoubleEndedIterator, BuildError>
    {
        let count = u32::try_from(self.0)
            .map_err(|_error| BuildError::NodeCountOverflow { len: self.0 })?;
        Ok((0 .. count).map(NodeSlot))
    }
}

/// Inclusive byte offset in source text.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the text-coordinate boundary is a single inclusive byte offset"
)]
pub struct TextOffset(pub u32);

impl From<TextOffset> for u32
{
    #[inline]
    fn from(value: TextOffset) -> Self
    {
        value.0
    }
}

/// Source byte length.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the text-length boundary is a single source byte length"
)]
pub struct TextLen(pub u32);

/// Host collection index into source text.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceIndex(pub usize);

impl From<SourceIndex> for usize
{
    #[inline]
    fn from(value: SourceIndex) -> Self
    {
        value.0
    }
}

/// Emptiness flag for a source byte range.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the range-emptiness boundary is a single boolean flag"
)]
pub struct TextRangeEmptiness(pub bool);

impl From<TextRangeEmptiness> for bool
{
    #[inline]
    fn from(value: TextRangeEmptiness) -> Self
    {
        value.0
    }
}

/// Emptiness flag for a CST arena.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the arena-emptiness boundary is a single boolean flag"
)]
pub struct CstEmptiness(pub bool);

impl From<CstEmptiness> for bool
{
    #[inline]
    fn from(value: CstEmptiness) -> Self
    {
        value.0
    }
}

/// Fingerprint of the grammar whose mold table produced a CST.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the grammar-fingerprint boundary is a single 64-bit fingerprint"
)]
pub struct GrammarFingerprint(pub u64);

/// Stable framed subtree hash.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the stable-hash boundary is a single 64-bit framed subtree hash"
)]
pub struct StableHash(pub u64);

impl core::fmt::LowerHex for StableHash
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        core::fmt::LowerHex::fmt(&self.0, f)
    }
}

/// Grout sort tag assigned by the producing grammar.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the grout-sort boundary is a single grammar-assigned sort tag"
)]
pub struct GroutSort(pub u16);

impl From<GroutSort> for u16
{
    #[inline]
    fn from(value: GroutSort) -> Self
    {
        value.0
    }
}

/// Flattened child arena starting slot.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChildStart(pub u32);

impl TryFrom<usize> for ChildStart
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error>
    {
        u32::try_from(value).map(Self)
    }
}

impl From<ChildStart> for u32
{
    #[inline]
    fn from(value: ChildStart) -> Self
    {
        value.0
    }
}

/// Flattened child arena span length.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChildCount(pub u32);

impl TryFrom<usize> for ChildCount
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error>
    {
        u32::try_from(value).map(Self)
    }
}

impl From<ChildCount> for u32
{
    #[inline]
    fn from(value: ChildCount) -> Self
    {
        value.0
    }
}

/// Single byte mixed into the stable hash frame.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HashByte(pub u8);

/// Count of significant children included in an interior hash frame.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignificantChildCount(pub u64);

impl SignificantChildCount
{
    /// The empty significant-child count.
    #[inline]
    pub(crate) const fn zero() -> Self
    {
        Self(0)
    }

    /// Return this count after adding one significant child.
    #[inline]
    pub(crate) fn increment(self) -> Result<Self, BuildError>
    {
        let Some(next) = self.0.checked_add(1)
        else {
            return Err(BuildError::IntegerConversion);
        };
        Ok(Self(next))
    }
}

/// Dense identity of a node in a [`Cst`] arena.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId
{
    /// Zero-based arena slot.
    raw: u32,
}

impl NodeId
{
    /// Create an identity from its dense arena slot.
    #[inline]
    pub(crate) const fn from_raw(raw: NodeSlot) -> Self
    {
        Self { raw: raw.0 }
    }

    /// Convert a dense slot into a node identity.
    ///
    /// # Contract
    /// - requires: `slot` is intended to index the node arena being built.
    /// - ensures: returns a [`NodeId`] carrying `slot` when `slot` fits in
    ///   `u32`.
    /// - provides: the checked boundary between host-sized collection lengths
    ///   and the compact CST identity width.
    /// - fails: returns [`BuildError::NodeCountOverflow`] when `slot` exceeds
    ///   `u32::MAX`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::NodeCountOverflow`] when `slot` cannot be
    /// represented as a `u32` node identity.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the representable/non-representable boundary is
    ///   exactly `u32::MAX` versus the first larger `usize` value, plus an
    ///   ordinary slot.
    /// - witness: `builder::tests::rejects_node_count_overflow`
    #[inline]
    pub(crate) fn from_index(slot: NodeIndex) -> Result<Self, BuildError>
    {
        let raw = u32::try_from(slot.0)
            .map_err(|_error| BuildError::NodeCountOverflow { len: slot.0 })?;
        Ok(Self::from_raw(NodeSlot(raw)))
    }

    /// Return the raw dense arena slot for crate internals.
    #[inline]
    #[must_use]
    pub(crate) const fn raw(self) -> NodeSlot
    {
        NodeSlot(self.raw)
    }

    /// Return the raw dense arena slot for read-only consumers.
    #[inline]
    #[must_use]
    pub const fn slot(self) -> NodeSlot
    {
        NodeSlot(self.raw)
    }

    /// Return the raw dense arena slot as a collection index.
    #[inline]
    pub(crate) fn index(self) -> Result<NodeIndex, BuildError>
    {
        let index = usize::try_from(self.raw)
            .map_err(|_error| BuildError::NodeCountOverflow { len: usize::MAX })?;
        Ok(NodeIndex(index))
    }
}

/// Byte range in the source buffer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextRange
{
    /// Inclusive start byte.
    start: u32,
    /// Exclusive end byte.
    end: u32,
}

impl TextRange
{
    /// Create a byte range after checking monotonicity.
    ///
    /// # Contract
    /// - requires: `start` and `end` are byte offsets in the source buffer's
    ///   compact coordinate space.
    /// - ensures: returns a range exactly covering `start..end` when `end >=
    ///   start`.
    /// - provides: the only public constructor for monotone CST text ranges.
    /// - fails: returns [`BuildError::InvalidTextRange`] when `end < start`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::InvalidTextRange`] when `end` precedes `start`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the order boundary is exhausted by equal endpoints,
    ///   ordinary increasing endpoints, and one decreasing pair.
    /// - witness: `model::tests::text_range_rejects_decreasing_end`
    #[inline]
    pub fn new(
        start: TextOffset,
        end: TextOffset,
    ) -> Result<Self, BuildError>
    {
        let range = Self {
            start: start.0,
            end: end.0,
        };
        if end < start {
            let start_index = range.start_index()?;
            let end_index = range.end_index()?;
            return Err(BuildError::InvalidTextRange {
                start: usize::from(start_index),
                end: usize::from(end_index),
            });
        }
        Ok(range)
    }

    /// Return the inclusive start byte.
    #[inline]
    #[must_use]
    pub const fn start(self) -> TextOffset
    {
        TextOffset(self.start)
    }

    /// Return the exclusive end byte.
    #[inline]
    #[must_use]
    pub const fn end(self) -> TextOffset
    {
        TextOffset(self.end)
    }

    /// Convert the inclusive start byte into a source index.
    #[inline]
    pub(crate) fn start_index(self) -> Result<SourceIndex, BuildError>
    {
        let index = usize::try_from(self.start).map_err(BuildError::from)?;
        Ok(SourceIndex(index))
    }

    /// Convert the exclusive end byte into a source index.
    #[inline]
    pub(crate) fn end_index(self) -> Result<SourceIndex, BuildError>
    {
        let index = usize::try_from(self.end).map_err(BuildError::from)?;
        Ok(SourceIndex(index))
    }

    /// Return the range width in bytes.
    #[inline]
    #[must_use]
    pub fn len(self) -> TextLen
    {
        TextLen(self.end.saturating_sub(self.start))
    }

    /// Return whether this range covers no bytes.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> TextRangeEmptiness
    {
        TextRangeEmptiness(self.start == self.end)
    }
}

/// Compact syntax tag for a node.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NodeKind
{
    /// Layout cell.
    Cell,
    /// Meld grouping.
    Meld,
    /// Wald grouping.
    Wald,
    /// Token leaf.
    Token,
}

/// Material significance class for hashing and diffing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Material
{
    /// Insignificant layout; does not contribute to parent significant hashes.
    Space,
    /// Significant syntax grout; contributes tag and mold but not source bytes.
    Grout,
    /// Significant tile; contributes tag, mold, and exact source bytes.
    Tile,
}

/// Opaque reference into the producing grammar's mold-definition table.
///
/// A `MoldId` names a mold `{rctx, prec, sort}` — a zipper into the grammar —
/// owned by `gandr-grammar`; layer A stores only this compact reference. Ids
/// are fingerprint-scoped: the grammar fingerprint recorded on a [`Cst`]
/// identifies the table these ids index, so ids never migrate silently across
/// grammar revisions.
///
/// # Contract
/// - requires: raw slots come from the mold table of the grammar whose
///   fingerprint the owning [`Cst`] records.
/// - ensures: preserves the compact `u32` slot exactly and participates in
///   mold-sensitive hashing.
/// - provides: the opaque tile-mold reference carried by significant tiles,
///   with checked host-index construction through [`TryFrom<usize>`].
/// - fails: [`TryFrom<usize>`] returns the integer-conversion error when a host
///   index cannot fit the compact `u32` mold-id slot.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 — a transparent newtype with no branches; mold-sensitivity
///   is witnessed where the id reaches the hash, and the host-index boundary is
///   the ordinary `usize`→`u32` integer boundary.
/// - witness: `tests::two_tiles_differing_only_in_mold_id_hash_differently`
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MoldId(u32);

impl From<u32> for MoldId
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<MoldId> for u32
{
    #[inline]
    fn from(value: MoldId) -> Self
    {
        value.0
    }
}

impl TryFrom<usize> for MoldId
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error>
    {
        u32::try_from(value).map(Self::from)
    }
}

/// Grout's own tip shape (tylr `Grout.re`), independent of tile mold identity.
///
/// The four grout shapes belong to grout material, ending the historical
/// conflation of grout tip shape with tile mold identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GroutShape
{
    /// Convex on both sides.
    Convex,
    /// Prefix: convex-left, concave-right.
    Prefix,
    /// Postfix: concave-left, convex-right.
    Postfix,
    /// Infix: concave on both sides.
    Infix,
}

impl GroutShape
{
    /// Return the stable frame tag for this grout shape.
    #[inline]
    #[must_use]
    pub(crate) const fn tag(self) -> HashByte
    {
        match self {
            | Self::Convex => HashByte(b'C'),
            | Self::Prefix => HashByte(b'P'),
            | Self::Postfix => HashByte(b'S'),
            | Self::Infix => HashByte(b'I'),
        }
    }
}

/// Material-governed mold payload carried by an arena node.
///
/// Tiles carry a [`MoldId`] into the grammar mold table; grout carries its own
/// [`GroutShape`] and a grammar sort tag; space carries neither. The builder
/// enforces the material-to-payload pairing.
///
/// # Contract
/// - requires: the payload variant matches the node material —
///   [`Material::Tile`] with [`MoldPayload::Tile`], [`Material::Grout`] with
///   [`MoldPayload::Grout`], and [`Material::Space`] with
///   [`MoldPayload::Space`].
/// - ensures: preserves the payload bits exactly for hashing and diffing.
/// - provides: the closed mold-payload vocabulary carried by [`NodeData`].
/// - fails: never; pairing is checked at construction.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the three variants distinguish the material classes at
///   the builder's payload gate.
/// - witness: `tests::malformed_ranges_and_material_boundaries_return_typed_errors`
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MoldPayload
{
    /// Layout space: no mold identity.
    Space,
    /// Grout: its own tip shape and a grammar sort tag.
    Grout
    {
        /// Grout's own tip shape.
        shape: GroutShape,
        /// Grammar sort tag carried for provenance.
        sort: GroutSort,
    },
    /// Tile: an opaque reference into the producing grammar's mold table.
    Tile(MoldId),
}

/// Crate-private arena node payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeData
{
    /// Stable framed FNV-1a hash for this subtree.
    hash: StableHash,
    /// Source byte range covered by this node.
    range: TextRange,
    /// Material-governed mold payload for this node.
    payload: MoldPayload,
    /// First child slot in the flattened child arena.
    child_start: ChildStart,
    /// Number of children in the flattened child arena.
    child_len: ChildCount,
    /// Parent node raw id, or [`NO_PARENT`] for the root.
    parent: u32,
    /// Syntax node tag.
    kind: NodeKind,
    /// Significance class.
    material: Material,
}

impl NodeData
{
    /// Create an arena node payload.
    #[inline]
    #[expect(
        clippy::too_many_arguments,
        reason = "the arena node constructor threads the eight validated payload fields positionally; a parameter struct would only relocate the same arity"
    )]
    pub(crate) const fn new(
        kind: NodeKind,
        material: Material,
        payload: MoldPayload,
        range: TextRange,
        parent: Option<NodeId>,
        child_start: ChildStart,
        child_len: ChildCount,
        hash: StableHash,
    ) -> Self
    {
        let parent = match parent {
            | Some(id) => id.raw().0,
            | None => NO_PARENT,
        };
        Self {
            hash,
            range,
            payload,
            child_start,
            child_len,
            parent,
            kind,
            material,
        }
    }

    /// Return the node kind.
    #[inline]
    pub(crate) const fn kind(self) -> NodeKind
    {
        self.kind
    }

    /// Return the material class.
    #[inline]
    pub(crate) const fn material(self) -> Material
    {
        self.material
    }

    /// Return the material-governed mold payload.
    #[inline]
    pub(crate) const fn payload(self) -> MoldPayload
    {
        self.payload
    }

    /// Return the covered source range.
    #[inline]
    pub(crate) const fn range(self) -> TextRange
    {
        self.range
    }

    /// Return the stable subtree hash.
    #[inline]
    pub(crate) const fn hash(self) -> StableHash
    {
        self.hash
    }

    /// Return the parent identity, if this node is not the root.
    #[inline]
    pub(crate) const fn parent(self) -> Option<NodeId>
    {
        if self.parent == NO_PARENT {
            None
        }
        else {
            Some(NodeId::from_raw(NodeSlot(self.parent)))
        }
    }

    /// Set the parent identity after the builder has allocated this node.
    #[inline]
    pub(crate) const fn set_parent(
        &mut self,
        parent: NodeId,
    )
    {
        self.parent = parent.raw().0;
    }

    /// Return the first flattened child slot.
    #[inline]
    pub(crate) const fn child_start(self) -> ChildStart
    {
        self.child_start
    }

    /// Return the flattened child count.
    #[inline]
    pub(crate) const fn child_len(self) -> ChildCount
    {
        self.child_len
    }
}

/// Flat arena concrete syntax tree.
#[derive(Clone, Debug)]
pub struct Cst
{
    /// Source text shared by all node ranges.
    source: SourceText,
    /// Dense node arena.
    nodes: Vec<NodeData>,
    /// Flattened child-id arena.
    children: Vec<NodeId>,
    /// Root node identity.
    root: NodeId,
    /// Fingerprint of the grammar that produced the [`MoldId`]s in this arena.
    grammar_fingerprint: GrammarFingerprint,
}

impl Cst
{
    /// Create a CST from validated arena parts.
    ///
    /// # Contract
    /// - requires: `nodes` and `children` use dense [`NodeId`] coordinates, and
    ///   `grammar_fingerprint` identifies the grammar whose mold table the
    ///   arena's [`MoldId`]s index.
    /// - ensures: returns a CST when `root`, every parent id, every child id,
    ///   every child span, and every text range resolves inside the supplied
    ///   arenas and source; the grammar fingerprint is recorded verbatim.
    /// - provides: the single checked constructor used by builder-like modules.
    /// - fails: returns a typed [`BuildError`] for the first malformed arena
    ///   edge.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::NodeCountOverflow`] when the node count cannot be
    /// represented by [`NodeId`], [`BuildError::NodeOutOfBounds`] when `root`,
    /// a parent id, or a child id is outside the node arena,
    /// [`BuildError::ChildRangeOutOfBounds`] when a node child span escapes the
    /// flattened child arena, or [`BuildError::InvalidTextBoundary`] when a
    /// node range is not a UTF-8 slice boundary in `source`.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — malformed root ids, parent ids, child ids, child
    ///   spans, and text boundaries each flip a distinct validation branch
    ///   while well-formed one-node and multi-node arenas witness the accepted
    ///   path.
    /// - witness: `builder::tests::builder_rejects_malformed_arena_parts`
    #[inline]
    pub(crate) fn from_parts(
        source: SourceText,
        nodes: Vec<NodeData>,
        children: Vec<NodeId>,
        root: NodeId,
        grammar_fingerprint: GrammarFingerprint,
    ) -> Result<Self, BuildError>
    {
        let node_len = nodes.len();
        let _node_ceiling = u32::try_from(node_len)
            .map_err(|_error| BuildError::NodeCountOverflow { len: node_len })?;
        let cst = Self {
            source,
            nodes,
            children,
            root,
            grammar_fingerprint,
        };
        let _root = cst.node_data(root)?;
        for node_id in cst.node_ids()? {
            let data = *cst.node_data(node_id)?;
            if let Some(parent) = data.parent() {
                let _parent = cst.node_data(parent)?;
            }
            let _children = cst.children_for_data(node_id, data)?;
            let _text = cst.text_for_data(node_id, data)?;
        }
        for child in &cst.children {
            let _data = cst.node_data(*child)?;
        }
        Ok(cst)
    }

    /// Return the source text backing this CST.
    #[inline]
    #[must_use]
    pub fn source(&self) -> &SourceText
    {
        &self.source
    }

    /// Return the root node identity.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> NodeId
    {
        self.root
    }

    /// Return the fingerprint of the grammar that produced this arena's molds.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the exact `grammar_fingerprint` supplied at
    ///   construction.
    /// - provides: the scope that gives this arena's [`MoldId`]s meaning; a
    ///   consumer must resolve mold references against the matching grammar
    ///   revision.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — a stored field returned unchanged; construction
    ///   witnesses observe round-trip preservation.
    /// - witness: `tests::builder_records_grammar_fingerprint`
    #[inline]
    #[must_use]
    pub const fn grammar_fingerprint(&self) -> GrammarFingerprint
    {
        self.grammar_fingerprint
    }

    /// Return a read-only view of `id`.
    ///
    /// # Contract
    /// - requires: `id` is expected to come from this CST's dense node arena.
    /// - ensures: returns a view tied to `self` when `id` is valid.
    /// - provides: the public checked node lookup surface.
    /// - fails: returns [`BuildError::NodeOutOfBounds`] when `id` is outside
    ///   the node arena.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::NodeOutOfBounds`] when `id` does not name a node
    /// in this CST.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the dense arena boundary is killed by a valid first
    ///   node, a valid last node, and the first id past the end.
    /// - witness: `model::tests::node_lookup_checks_bounds`
    #[inline]
    pub fn node(
        &self,
        id: NodeId,
    ) -> Result<NodeView<'_>, BuildError>
    {
        let data = self.node_data(id)?;
        Ok(NodeView {
            cst: self,
            id,
            data,
        })
    }

    /// Return the children of `id`.
    ///
    /// # Contract
    /// - requires: `id` is expected to name a node in this CST.
    /// - ensures: returns the child-id slice for that node when the arena is
    ///   valid.
    /// - provides: read-only access to the flattened child arena without
    ///   exposing mutable storage.
    /// - fails: returns a typed [`BuildError`] if `id` or its stored child span
    ///   is invalid.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::NodeOutOfBounds`] when `id` is invalid, or
    /// [`BuildError::ChildRangeOutOfBounds`] when the stored child span escapes
    /// the child arena.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — tests distinguish leaf nodes, non-empty child spans,
    ///   bad node ids, and spans that overflow or exceed the child arena.
    /// - witness: `model::tests::children_lookup_checks_stored_span`
    #[inline]
    pub fn children(
        &self,
        id: NodeId,
    ) -> Result<&[NodeId], BuildError>
    {
        let data = *self.node_data(id)?;
        self.children_for_data(id, data)
    }

    /// Return the source text covered by `id`.
    ///
    /// # Contract
    /// - requires: `id` is expected to name a node whose range lies on UTF-8
    ///   boundaries in this CST's source.
    /// - ensures: returns the exact borrowed source slice covered by `id`.
    /// - provides: zero-copy token and subtree text access.
    /// - fails: returns a typed [`BuildError`] if the node or text boundaries
    ///   are invalid.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::NodeOutOfBounds`] when `id` is invalid, or
    /// [`BuildError::InvalidTextBoundary`] when the stored range is not a valid
    /// borrowed slice of the source text.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — ASCII, multi-byte UTF-8, out-of-source, and interior
    ///   codepoint-boundary ranges distinguish the lookup decisions.
    /// - witness: `model::tests::text_lookup_is_zero_copy_and_boundary_checked`
    #[inline]
    pub fn text(
        &self,
        id: NodeId,
    ) -> Result<SourceSlice<'_>, BuildError>
    {
        let data = *self.node_data(id)?;
        self.text_for_data(id, data)
    }

    /// Return the stable subtree hash for `id`.
    ///
    /// # Contract
    /// - requires: `id` is expected to name a node in this CST.
    /// - ensures: returns the stored framed FNV-1a hash for `id` when valid.
    /// - provides: the cheap equality key used by diffing.
    /// - fails: returns [`BuildError::NodeOutOfBounds`] when `id` is outside
    ///   the node arena.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::NodeOutOfBounds`] when `id` does not name a node
    /// in this CST.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — valid and first-out-of-range ids kill the only
    ///   branch; hash content itself is produced by the builder hash tests.
    /// - witness: `model::tests::hash_lookup_checks_bounds`
    #[inline]
    pub fn hash(
        &self,
        id: NodeId,
    ) -> Result<StableHash, BuildError>
    {
        let data = *self.node_data(id)?;
        Ok(data.hash())
    }

    /// Return the number of nodes in the arena.
    #[inline]
    #[must_use]
    pub fn len(&self) -> NodeCount
    {
        NodeCount(self.nodes.len())
    }

    /// Return whether the node arena is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> CstEmptiness
    {
        CstEmptiness(self.nodes.is_empty())
    }

    /// Return the crate-private node payload for `id`.
    ///
    /// # Contract
    /// - requires: `id` is expected to name a node in this CST.
    /// - ensures: returns the immutable arena payload when `id` is valid.
    /// - provides: the shared checked lookup primitive for sibling modules.
    /// - fails: returns [`BuildError::NodeOutOfBounds`] when `id` is outside
    ///   the node arena.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::NodeOutOfBounds`] when `id` does not name a node
    /// in this CST.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the dense arena boundary is tested once here and
    ///   reused by all public accessors.
    /// - witness: `model::tests::node_lookup_checks_bounds`
    #[inline]
    pub(crate) fn node_data(
        &self,
        id: NodeId,
    ) -> Result<&NodeData, BuildError>
    {
        let node_index = id.index()?;
        let index = usize::from(node_index);
        self.nodes.get(index).ok_or(BuildError::NodeOutOfBounds {
            id,
            len: self.nodes.len(),
        })
    }

    /// Return all dense node ids in arena order.
    ///
    /// # Contract
    /// - requires: the node arena length fits in [`NodeId`].
    /// - ensures: returns one id per node in dense arena order.
    /// - provides: an iteration aid for validation and sibling modules.
    /// - fails: returns [`BuildError::NodeCountOverflow`] before allocating ids
    ///   if the count cannot be represented as compact slots.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::NodeCountOverflow`] when the arena length exceeds
    /// the compact identity width.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — empty, ordinary, and overflow-sized arenas cover the
    ///   only conversion boundary.
    /// - witness: `builder::tests::rejects_node_count_overflow`
    #[inline]
    pub(crate) fn node_ids(&self) -> Result<Vec<NodeId>, BuildError>
    {
        let slots = NodeCount(self.nodes.len()).slots()?;
        Ok(slots.map(NodeId::from_raw).collect())
    }

    /// Return children for an already-loaded node payload.
    ///
    /// # Contract
    /// - requires: `data` is the payload stored for `id` in this CST.
    /// - ensures: returns exactly `data.child_len()` ids starting at
    ///   `data.child_start()` when the span resolves.
    /// - provides: a single checked child-span decoder for all callers.
    /// - fails: returns [`BuildError::ChildRangeOutOfBounds`] when the encoded
    ///   span overflows or escapes the child arena.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::ChildRangeOutOfBounds`] when the decoded child
    /// span is not contained in the flattened child arena.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — ordinary spans, empty spans, overflowing starts plus
    ///   lengths, and starts past the arena end distinguish the span decoder.
    /// - witness: `model::tests::children_lookup_checks_stored_span`
    #[inline]
    pub(crate) fn children_for_data(
        &self,
        id: NodeId,
        data: NodeData,
    ) -> Result<&[NodeId], BuildError>
    {
        let start = usize::try_from(u32::from(data.child_start())).map_err(|_error| {
            BuildError::ChildRangeOutOfBounds {
                node: id,
                start: usize::MAX,
                len: usize::MAX,
                children_len: self.children.len(),
            }
        })?;
        let len = usize::try_from(u32::from(data.child_len())).map_err(|_error| {
            BuildError::ChildRangeOutOfBounds {
                node: id,
                start,
                len: usize::MAX,
                children_len: self.children.len(),
            }
        })?;
        let Some(end) = start.checked_add(len)
        else {
            return Err(BuildError::ChildRangeOutOfBounds {
                node: id,
                start,
                len,
                children_len: self.children.len(),
            });
        };
        self.children
            .get(start .. end)
            .ok_or(BuildError::ChildRangeOutOfBounds {
                node: id,
                start,
                len,
                children_len: self.children.len(),
            })
    }

    /// Return text for an already-loaded node payload.
    ///
    /// # Contract
    /// - requires: `data` is the payload stored for `id` in this CST.
    /// - ensures: returns the exact borrowed source text for `data.range()`
    ///   when it resolves on UTF-8 boundaries.
    /// - provides: the shared zero-copy text decoder for all accessors.
    /// - fails: returns [`BuildError::InvalidTextBoundary`] when the range
    ///   cannot be borrowed from `source`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::InvalidTextBoundary`] when the range is out of
    /// source bounds or splits a UTF-8 codepoint.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — source bounds and UTF-8 codepoint boundaries are
    ///   killed independently by text accessor tests.
    /// - witness: `model::tests::text_lookup_is_zero_copy_and_boundary_checked`
    #[inline]
    pub(crate) fn text_for_data(
        &self,
        id: NodeId,
        data: NodeData,
    ) -> Result<SourceSlice<'_>, BuildError>
    {
        let range = data.range();
        let start_index = range.start_index()?;
        let end_index = range.end_index()?;
        let start = usize::from(start_index);
        let end = usize::from(end_index);
        let text =
            self.source
                .as_ref()
                .get(start .. end)
                .ok_or(BuildError::InvalidTextBoundary {
                    node: id,
                    start,
                    end,
                })?;
        Ok(SourceSlice::from(text))
    }
}

/// Read-only node view tied to a [`Cst`].
#[derive(Clone, Copy, Debug)]
pub struct NodeView<'cst>
{
    /// Tree that owns this node.
    cst: &'cst Cst,
    /// Node identity.
    id: NodeId,
    /// Cached node payload.
    data: &'cst NodeData,
}

impl<'cst> NodeView<'cst>
{
    /// Return this view's node identity.
    #[inline]
    #[must_use]
    pub const fn id(self) -> NodeId
    {
        self.id
    }

    /// Return this node's syntax kind.
    #[inline]
    #[must_use]
    pub const fn kind(self) -> NodeKind
    {
        self.data.kind()
    }

    /// Return this node's material class.
    #[inline]
    #[must_use]
    pub const fn material(self) -> Material
    {
        self.data.material()
    }

    /// Return this node's material-governed mold payload.
    #[inline]
    #[must_use]
    pub const fn payload(self) -> MoldPayload
    {
        self.data.payload()
    }

    /// Return this node's source range.
    #[inline]
    #[must_use]
    pub const fn range(self) -> TextRange
    {
        self.data.range()
    }

    /// Return this node's stable subtree hash.
    #[inline]
    #[must_use]
    pub const fn hash(self) -> StableHash
    {
        self.data.hash()
    }

    /// Return this node's parent identity, if it has one.
    #[inline]
    #[must_use]
    pub const fn parent(self) -> Option<NodeId>
    {
        self.data.parent()
    }

    /// Return this node's children.
    ///
    /// # Contract
    /// - requires: this view was created by [`Cst::node`].
    /// - ensures: returns the child slice for this node when the stored child
    ///   span is valid.
    /// - provides: view-local child access without exposing mutable storage.
    /// - fails: returns [`BuildError::ChildRangeOutOfBounds`] if the stored
    ///   span is malformed.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::ChildRangeOutOfBounds`] when the stored child span
    /// is invalid.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — this delegates to `Cst::children_for_data`; view
    ///   tests only need to witness that the cached id and data are used
    ///   consistently.
    /// - witness: `model::tests::node_view_delegates_checked_accessors`
    #[inline]
    pub fn children(self) -> Result<&'cst [NodeId], BuildError>
    {
        self.cst.children_for_data(self.id, *self.data)
    }

    /// Return this node's covered source text.
    ///
    /// # Contract
    /// - requires: this view was created by [`Cst::node`].
    /// - ensures: returns the exact borrowed text covered by this node when the
    ///   stored range resolves.
    /// - provides: view-local zero-copy text access.
    /// - fails: returns [`BuildError::InvalidTextBoundary`] if the range cannot
    ///   be borrowed from the source.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`BuildError::InvalidTextBoundary`] when the stored range is out
    /// of source bounds or splits a UTF-8 codepoint.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — this delegates to `Cst::text_for_data`; view tests
    ///   only need to witness that the cached id and data are used
    ///   consistently.
    /// - witness: `model::tests::node_view_delegates_checked_accessors`
    #[inline]
    pub fn text(self) -> Result<SourceSlice<'cst>, BuildError>
    {
        self.cst.text_for_data(self.id, *self.data)
    }
}
