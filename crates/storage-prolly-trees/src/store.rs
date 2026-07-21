//! Deterministic encoded-node storage boundary.
//!
//! ## current
//!
//! - [`BlockStore`] stores canonical encoded nodes keyed by [`NodeHash`].
//! - [`InMemoryBlockStore`] uses [`alloc::collections::BTreeMap`] so iteration
//!   and replacement behavior are deterministic for local callers and tests.
//! - [`PackedSegmentStore`] keeps verified encoded nodes in one append-only
//!   byte segment with a deterministic [`BTreeMap`] index of byte ranges.
//! - Stored bytes are checked with [`verify_stored_node`] before they enter or
//!   leave the store.
//!
//! ## designed direction
//!
//! - Persistent stores can implement [`BlockStore`] once backend selection and
//!   traversal policy are reviewed outside this crate slice.
//!
//! ## open decision
//!
//! - Persistent backend choice, transport mapping, and traversal profiles
//!   remain outside this module.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::error::ProllyBaoError;
use crate::proof::verify_stored_node;
use crate::types::NodeHash;
use crate::types::StoredNodeRef;

/// Narrow content-addressed boundary for encoded Prolly-Bao nodes.
///
/// # Contract
///
/// This boundary carries three ratified storage-tier constraints
/// (`docs/research/massive-term-design.md` §6.1;
/// `docs/research/storage-rkyv-design.md` §5, §10 Q3/Q5):
///
/// - **Verified-blob guarantee.** [`load`](BlockStore::load) returns only blobs
///   whose [`NodeHash`] identity has been verified. An implementor MUST run (or
///   have provably already run)
///   [`verify_stored_node`](crate::verify_stored_node) — recomputing
///   `BLAKE3(encoded node bytes)` and rejecting any mismatch — before handing
///   bytes out, so a caller cannot obtain unverified bytes by this path. Both
///   bundled stores verify on every `insert` and every `load`.
/// - **Alignment (storage-rkyv Q3, owner-ratified).** The blob-return contract
///   guarantees slices aligned to the archived root's alignment requirement, so
///   a future zero-copy `rkyv` reader can `access` them without copying. A
///   packed-segment implementor pads blob starts to that alignment (≤ 16 bytes
///   is the practical bound under `rkyv` 0.8 `AlignedVec` conventions; the
///   exact bound is golden-pinned when `storage-rkyv` lands). An implementor
///   that cannot pad copies the blob into an `AlignedVec` on read — a
///   documented fallback that forfeits zero-copy, never the default. This
///   skeleton does not yet pad (see [`PackedSegmentStore`]); the obligation
///   lands with the `storage-rkyv` reader.
/// - **Rehydration budget (storage-rkyv Q5).** A store-tier rehydration bound
///   (a blob-count / depth cap mirroring the export layer's
///   `MAX_EXPANDED_TERM_WORK`) is a standing design constraint so a
///   malformed-but-integral state cannot fan out unboundedly while
///   hash-following. It is documented here as an obligation on hash-following
///   consumers; this skeleton does not implement it beyond documentation.
///
/// - requires: implementors uphold the three constraints above.
/// - ensures: [`load`](BlockStore::load) yields hash-verified blobs whose
///   identity equals the requested [`NodeHash`].
/// - provides: a content-addressed node boundary the tree and the future
///   `storage-rkyv` reader build on.
/// - fails: `insert`/`load` surface typed [`ProllyBaoError`] variants (see each
///   method's `# Errors`).
/// - panics: none.
#[expect(
    clippy::module_name_repetitions,
    reason = "public API intentionally uses BlockStore storage-boundary terminology"
)]
pub trait BlockStore
{
    /// Inserts encoded node bytes under their claimed [`NodeHash`].
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::HashMismatch`] when the bytes do not hash to
    /// `node.node_hash()`, or decoder errors when the bytes are not canonical
    /// encoded node material.
    fn insert(
        &mut self,
        node: StoredNodeRef<'_>,
    ) -> Result<(), ProllyBaoError>;

    /// Loads encoded node bytes by [`NodeHash`].
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::UnknownNodeHash`] when `hash` is absent, or a
    /// verification error when stored bytes no longer hash to `hash` or no
    /// longer decode as canonical node material.
    fn load(
        &self,
        hash: NodeHash,
    ) -> Result<StoredNodeRef<'_>, ProllyBaoError>;
}

/// Deterministic in-memory [`BlockStore`] for local callers and tests.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::module_name_repetitions,
    reason = "public API intentionally names the in-memory BlockStore implementation"
)]
#[non_exhaustive]
pub struct InMemoryBlockStore
{
    /// Encoded nodes keyed by their opaque BLAKE3 identity.
    nodes: BTreeMap<NodeHash, Box<[u8]>>,
}

impl InMemoryBlockStore
{
    /// Creates an empty deterministic in-memory store.
    #[inline]
    #[must_use]
    pub const fn new() -> Self
    {
        return Self {
            nodes: BTreeMap::new(),
        };
    }
}

impl Default for InMemoryBlockStore
{
    #[inline]
    fn default() -> Self
    {
        return Self::new();
    }
}

impl BlockStore for InMemoryBlockStore
{
    #[inline]
    fn insert(
        &mut self,
        node: StoredNodeRef<'_>,
    ) -> Result<(), ProllyBaoError>
    {
        verify_stored_node(node)?;

        let _previous = self
            .nodes
            .insert(node.node_hash(), Box::<[u8]>::from(node.bytes()));

        return Ok(());
    }

    #[inline]
    fn load(
        &self,
        hash: NodeHash,
    ) -> Result<StoredNodeRef<'_>, ProllyBaoError>
    {
        let bytes = self
            .nodes
            .get(&hash)
            .ok_or(ProllyBaoError::UnknownNodeHash { hash })?;
        let node = StoredNodeRef::new(hash, bytes.as_ref());

        verify_stored_node(node)?;

        return Ok(node);
    }
}

/// Byte range metadata for a node inside a [`PackedSegmentStore`] segment.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct NodeSegmentEntry
{
    /// Byte offset where the encoded node starts inside the packed segment.
    offset: usize,
    /// Byte length of the encoded node inside the packed segment.
    length: usize,
}

impl NodeSegmentEntry
{
    /// Creates byte range metadata for a packed node.
    #[inline]
    #[must_use]
    pub const fn new(
        offset: usize,
        length: usize,
    ) -> Self
    {
        return Self { offset, length };
    }

    /// Returns the byte offset where the encoded node starts.
    #[inline]
    #[must_use]
    pub const fn offset(&self) -> usize
    {
        return self.offset;
    }

    /// Returns the byte length of the encoded node.
    #[inline]
    #[must_use]
    pub const fn length(&self) -> usize
    {
        return self.length;
    }

    /// Returns the exclusive end offset for this entry.
    #[inline]
    fn checked_end(&self) -> Result<usize, ProllyBaoError>
    {
        return self
            .offset
            .checked_add(self.length)
            .ok_or(ProllyBaoError::MalformedNodeBytes {
                context: "packed segment entry length overflow",
            });
    }
}

/// In-memory packed-segment [`BlockStore`] prototype for adapter review.
///
/// # Contract
///
/// - intension: [`load`](BlockStore::load) returns a borrowed slice at the
///   node's stored byte offset. This prototype does **not** yet pad blob starts
///   to an archived-root alignment (the [`BlockStore`] Q3 alignment
///   obligation), so returned slices are byte-aligned only; the padding — or
///   the copy-to-`AlignedVec` fallback — lands with the zero-copy
///   `storage-rkyv` reader. Node identity and verification are unaffected
///   because `NodeHash` is `BLAKE3(encoded node bytes)`, independent of segment
///   placement.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::module_name_repetitions,
    reason = "public API intentionally names the packed-segment BlockStore implementation"
)]
#[non_exhaustive]
pub struct PackedSegmentStore
{
    /// Append-only byte segment containing encoded nodes.
    segment: Vec<u8>,
    /// Deterministic node-hash index into `segment`.
    index: BTreeMap<NodeHash, NodeSegmentEntry>,
}

impl PackedSegmentStore
{
    /// Creates an empty packed-segment store.
    #[inline]
    #[must_use]
    pub const fn new() -> Self
    {
        return Self {
            segment: Vec::new(),
            index: BTreeMap::new(),
        };
    }

    /// Creates a packed store from raw segment bytes and a node range index.
    ///
    /// This constructor verifies every indexed range before returning, and
    /// rejects ranges that are out of bounds, overlapping, malformed, or whose
    /// bytes do not match their indexed [`NodeHash`].
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::MalformedNodeBytes`] for invalid segment
    /// ranges or malformed encoded nodes, [`ProllyBaoError::HashMismatch`] for
    /// mismatched bytes, and decoder errors from [`verify_stored_node`] for
    /// unsupported node encodings.
    #[inline]
    pub fn from_raw_parts(
        segment: Vec<u8>,
        index: BTreeMap<NodeHash, NodeSegmentEntry>,
    ) -> Result<Self, ProllyBaoError>
    {
        verify_packed_segment_index(segment.as_slice(), &index)?;

        return Ok(Self { segment, index });
    }

    /// Consumes this store into raw segment bytes and its deterministic index.
    #[inline]
    #[must_use]
    pub fn into_raw_parts(self) -> (Vec<u8>, BTreeMap<NodeHash, NodeSegmentEntry>)
    {
        return (self.segment, self.index);
    }
}

impl Default for PackedSegmentStore
{
    #[inline]
    fn default() -> Self
    {
        return Self::new();
    }
}

impl BlockStore for PackedSegmentStore
{
    #[inline]
    fn insert(
        &mut self,
        node: StoredNodeRef<'_>,
    ) -> Result<(), ProllyBaoError>
    {
        verify_stored_node(node)?;

        let offset = self.segment.len();
        let length = node.bytes().len();
        self.segment.extend_from_slice(node.bytes());
        let _previous = self
            .index
            .insert(node.node_hash(), NodeSegmentEntry::new(offset, length));

        return Ok(());
    }

    #[inline]
    fn load(
        &self,
        hash: NodeHash,
    ) -> Result<StoredNodeRef<'_>, ProllyBaoError>
    {
        let entry = self
            .index
            .get(&hash)
            .ok_or(ProllyBaoError::UnknownNodeHash { hash })?;
        let bytes = packed_entry_bytes(self.segment.as_slice(), *entry)?;
        let node = StoredNodeRef::new(hash, bytes);

        verify_stored_node(node)?;

        return Ok(node);
    }
}

/// Verifies that every indexed packed node range is usable and disjoint.
fn verify_packed_segment_index(
    segment: &[u8],
    index: &BTreeMap<NodeHash, NodeSegmentEntry>,
) -> Result<(), ProllyBaoError>
{
    let mut occupied_ranges = BTreeMap::new();

    for entry in index.values() {
        let range_start = entry.offset();
        let range_end = entry.checked_end()?;
        ensure_packed_entry_is_disjoint(&occupied_ranges, range_start, range_end)?;
        let _previous_end = occupied_ranges.insert(range_start, range_end);
        let _bytes = packed_entry_bytes(segment, *entry)?;
    }

    for (hash, entry) in index {
        let bytes = packed_entry_bytes(segment, *entry)?;
        verify_stored_node(StoredNodeRef::new(*hash, bytes))?;
    }

    return Ok(());
}

/// Returns encoded node bytes for one packed entry.
#[inline]
fn packed_entry_bytes(
    segment: &[u8],
    entry: NodeSegmentEntry,
) -> Result<&[u8], ProllyBaoError>
{
    let range_end = entry.checked_end()?;

    return segment
        .get(entry.offset() .. range_end)
        .ok_or(ProllyBaoError::MalformedNodeBytes {
            context: "packed segment entry is outside segment",
        });
}

/// Rejects a packed range that overlaps already-indexed ranges.
fn ensure_packed_entry_is_disjoint(
    occupied_ranges: &BTreeMap<usize, usize>,
    range_start: usize,
    range_end: usize,
) -> Result<(), ProllyBaoError>
{
    if let Some(previous_range) = occupied_ranges.range(..= range_start).next_back() {
        let previous_end = *previous_range.1;

        if previous_end > range_start {
            return Err(ProllyBaoError::MalformedNodeBytes {
                context: "packed segment entries overlap",
            });
        }
    }

    if let Some(next_range) = occupied_ranges.range(range_start ..).next() {
        let next_start = *next_range.0;

        if next_start < range_end {
            return Err(ProllyBaoError::MalformedNodeBytes {
                context: "packed segment entries overlap",
            });
        }
    }

    return Ok(());
}
