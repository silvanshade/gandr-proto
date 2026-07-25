//! Public value types for Prolly-Bao roots, records, stores, and proofs.

use alloc::boxed::Box;
use core::fmt;

use gandr_storage_chunker::ChunkerParams;

use crate::error::ProllyBaoError;

/// Length in bytes of every opaque BLAKE3 node identity.
pub const NODE_HASH_LEN: usize = blake3::OUT_LEN;

/// Opaque BLAKE3 identity for an encoded Prolly-Bao node.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeHash(
    /// Raw BLAKE3 output bytes.
    [u8; NODE_HASH_LEN],
);

impl NodeHash
{
    /// Creates a node hash from raw BLAKE3 output bytes.
    #[inline]
    #[must_use]
    pub const fn from_bytes(bytes: [u8; NODE_HASH_LEN]) -> Self
    {
        return Self(bytes);
    }

    /// Returns the hash as a fixed-size byte array.
    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; NODE_HASH_LEN]
    {
        return &self.0;
    }

    /// Returns the hash as a byte slice.
    #[inline]
    #[must_use]
    pub const fn as_slice(&self) -> &[u8]
    {
        return &self.0;
    }

    /// Consumes the hash and returns the raw bytes.
    #[inline]
    #[must_use]
    pub const fn into_bytes(self) -> [u8; NODE_HASH_LEN]
    {
        return self.0;
    }
}

impl fmt::Debug for NodeHash
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        return fmt::Display::fmt(self, f);
    }
}

impl fmt::Display for NodeHash
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }

        return Ok(());
    }
}

impl From<[u8; NODE_HASH_LEN]> for NodeHash
{
    #[inline]
    fn from(bytes: [u8; NODE_HASH_LEN]) -> Self
    {
        return Self::from_bytes(bytes);
    }
}

impl AsRef<[u8]> for NodeHash
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.as_slice();
    }
}

impl AsRef<[u8; NODE_HASH_LEN]> for NodeHash
{
    #[inline]
    fn as_ref(&self) -> &[u8; NODE_HASH_LEN]
    {
        return self.as_bytes();
    }
}

/// Prolly-Bao tree shape represented by a root.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum TreeKind
{
    /// Ordered-record Merkle search tree.
    MerkleSearch,
}

/// Encoding version for canonical node, root, and proof material.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum EncodingVersion
{
    /// First Prolly-Bao encoding version.
    V1,
}

impl EncodingVersion
{
    /// Current encoding version for newly built roots.
    pub const CURRENT: Self = Self::V1;

    /// Returns the stable integer representation for committed material.
    #[inline]
    #[must_use]
    pub const fn as_u16(self) -> u16
    {
        match self {
            | Self::V1 => return 1_u16,
        }
    }
}

/// Cryptographic hash algorithm committed by a tree root.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum HashAlgorithm
{
    /// BLAKE3 with full 32-byte output, exposed only through [`NodeHash`].
    Blake3,
}

impl HashAlgorithm
{
    /// Current node identity algorithm for newly built roots.
    pub const CURRENT: Self = Self::Blake3;
}

/// Separator-key convention committed by a tree root.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum SeparatorConvention
{
    /// Internal-node separators carry the first key reachable in each child.
    FirstKey,
}

impl SeparatorConvention
{
    /// Current separator convention for newly built roots.
    pub const CURRENT: Self = Self::FirstKey;
}

/// Consensus-sensitive parameters committed by a root and by proofs.
#[derive(Clone)]
#[non_exhaustive]
pub struct TreeParams
{
    /// Tree shape marker.
    kind: TreeKind,
    /// Canonical encoding version.
    encoding_version: EncodingVersion,
    /// Cryptographic node identity algorithm.
    hash_algorithm: HashAlgorithm,
    /// Internal separator convention.
    separator_convention: SeparatorConvention,
    /// Chunker parameters used for record-safe boundary detection.
    chunker_params: ChunkerParams,
    /// Stable chunker parameter commitment bytes copied into root material.
    chunker_parameter_bytes: Box<[u8]>,
}

impl TreeParams
{
    /// Creates tree parameters from explicit consensus-sensitive choices.
    #[inline]
    #[must_use]
    pub fn new(
        kind: TreeKind,
        encoding_version: EncodingVersion,
        hash_algorithm: HashAlgorithm,
        separator_convention: SeparatorConvention,
        chunker_params: ChunkerParams,
    ) -> Self
    {
        let chunker_parameter_bytes = Box::<[u8]>::from(chunker_params.commitment_bytes().as_ref());

        return Self {
            kind,
            encoding_version,
            hash_algorithm,
            separator_convention,
            chunker_params,
            chunker_parameter_bytes,
        };
    }

    /// Creates the current default parameters using the chunker default
    /// profile.
    #[inline]
    #[must_use]
    pub fn current() -> Self
    {
        return Self::new(
            TreeKind::MerkleSearch,
            EncodingVersion::CURRENT,
            HashAlgorithm::CURRENT,
            SeparatorConvention::CURRENT,
            ChunkerParams::default_fastcdc(),
        );
    }

    /// Returns the committed tree shape marker.
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> TreeKind
    {
        return self.kind;
    }

    /// Returns the committed encoding version.
    #[inline]
    #[must_use]
    pub const fn encoding_version(&self) -> EncodingVersion
    {
        return self.encoding_version;
    }

    /// Returns the committed cryptographic hash algorithm marker.
    #[inline]
    #[must_use]
    pub const fn hash_algorithm(&self) -> HashAlgorithm
    {
        return self.hash_algorithm;
    }

    /// Returns the committed separator convention.
    #[inline]
    #[must_use]
    pub const fn separator_convention(&self) -> SeparatorConvention
    {
        return self.separator_convention;
    }

    /// Returns the committed chunker parameters.
    #[inline]
    #[must_use]
    pub const fn chunker_params(&self) -> &ChunkerParams
    {
        return &self.chunker_params;
    }

    /// Returns the stable chunker parameter bytes committed in root material.
    #[inline]
    #[must_use]
    pub fn chunker_parameter_bytes(&self) -> &[u8]
    {
        return self.chunker_parameter_bytes.as_ref();
    }
}

impl Default for TreeParams
{
    #[inline]
    fn default() -> Self
    {
        return Self::current();
    }
}

impl fmt::Debug for TreeParams
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        return f
            .debug_struct("TreeParams")
            .field("kind", &self.kind)
            .field("encoding_version", &self.encoding_version)
            .field("hash_algorithm", &self.hash_algorithm)
            .field("separator_convention", &self.separator_convention)
            .field("chunker_parameter_bytes", &self.chunker_parameter_bytes)
            .finish_non_exhaustive();
    }
}

impl PartialEq for TreeParams
{
    #[inline]
    fn eq(
        &self,
        other: &Self,
    ) -> bool
    {
        return self.kind == other.kind
            && self.encoding_version == other.encoding_version
            && self.hash_algorithm == other.hash_algorithm
            && self.separator_convention == other.separator_convention
            && self.chunker_parameter_bytes == other.chunker_parameter_bytes;
    }
}

impl Eq for TreeParams
{
}

/// Content-addressed root of a Prolly-Bao tree.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct TreeRoot
{
    /// Opaque BLAKE3 hash for the root node or empty-root manifest.
    hash: NodeHash,
    /// Parameters committed by the root.
    params: TreeParams,
    /// Number of canonical records represented by the tree.
    record_count: u64,
}

impl TreeRoot
{
    /// Creates a tree root from an opaque root hash, parameters, and record
    /// count.
    #[inline]
    #[must_use]
    pub const fn new(
        hash: NodeHash,
        params: TreeParams,
        record_count: u64,
    ) -> Self
    {
        return Self {
            hash,
            params,
            record_count,
        };
    }

    /// Returns the opaque root hash.
    #[inline]
    #[must_use]
    pub const fn hash(&self) -> NodeHash
    {
        return self.hash;
    }

    /// Returns the parameters committed by this root.
    #[inline]
    #[must_use]
    pub const fn params(&self) -> &TreeParams
    {
        return &self.params;
    }

    /// Returns the number of canonical records represented by this root.
    #[inline]
    #[must_use]
    pub const fn record_count(&self) -> u64
    {
        return self.record_count;
    }
}

/// Borrowed canonical key-value record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct RecordRef<'record>
{
    /// Canonical record key bytes.
    key: &'record [u8],
    /// Canonical record value bytes.
    value: &'record [u8],
}

impl<'record> RecordRef<'record>
{
    /// Creates a borrowed canonical record reference.
    #[inline]
    #[must_use]
    pub const fn new(
        key: &'record [u8],
        value: &'record [u8],
    ) -> Self
    {
        return Self { key, value };
    }

    /// Returns the canonical key bytes.
    #[inline]
    #[must_use]
    pub const fn key(&self) -> &'record [u8]
    {
        return self.key;
    }

    /// Returns the canonical value bytes.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &'record [u8]
    {
        return self.value;
    }
}

/// Owned canonical key-value record.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct Record
{
    /// Canonical record key bytes.
    key: Box<[u8]>,
    /// Canonical record value bytes.
    value: Box<[u8]>,
}

impl Record
{
    /// Creates an owned canonical record.
    #[inline]
    #[must_use]
    pub fn new<K, V>(
        key: K,
        value: V,
    ) -> Self
    where
        K: Into<Box<[u8]>>,
        V: Into<Box<[u8]>>,
    {
        return Self {
            key: key.into(),
            value: value.into(),
        };
    }

    /// Returns the canonical key bytes.
    #[inline]
    #[must_use]
    pub fn key(&self) -> &[u8]
    {
        return self.key.as_ref();
    }

    /// Returns the canonical value bytes.
    #[inline]
    #[must_use]
    pub fn value(&self) -> &[u8]
    {
        return self.value.as_ref();
    }

    /// Returns a borrowed view of this owned record.
    #[inline]
    #[must_use]
    pub fn as_record_ref(&self) -> RecordRef<'_>
    {
        return RecordRef::new(self.key(), self.value());
    }

    /// Splits this record into owned key and value byte buffers.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (Box<[u8]>, Box<[u8]>)
    {
        return (self.key, self.value);
    }
}

impl<'record> From<RecordRef<'record>> for Record
{
    #[inline]
    fn from(record: RecordRef<'record>) -> Self
    {
        return Self::new(record.key, record.value);
    }
}

/// Borrowed key bound for ordered-record range traversal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum KeyBound<'key>
{
    /// No bound on this side of the range.
    Unbounded,
    /// Bound includes the named key.
    Included(&'key [u8]),
    /// Bound excludes the named key.
    Excluded(&'key [u8]),
}

impl<'key> KeyBound<'key>
{
    /// Creates an inclusive key bound.
    #[inline]
    #[must_use]
    pub const fn included(key: &'key [u8]) -> Self
    {
        return Self::Included(key);
    }

    /// Creates an exclusive key bound.
    #[inline]
    #[must_use]
    pub const fn excluded(key: &'key [u8]) -> Self
    {
        return Self::Excluded(key);
    }

    /// Returns `true` when this bound is unbounded.
    #[inline]
    #[must_use]
    pub const fn is_unbounded(self) -> bool
    {
        match self {
            | Self::Unbounded => return true,
            | Self::Included(_) | Self::Excluded(_) => return false,
        }
    }

    /// Returns the bound key when the bound is inclusive or exclusive.
    #[inline]
    #[must_use]
    pub const fn key(self) -> Option<&'key [u8]>
    {
        match self {
            | Self::Unbounded => return None,
            | Self::Included(key) | Self::Excluded(key) => return Some(key),
        }
    }
}

/// Borrowed key range for ordered-record traversal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct KeyRangeRef<'key>
{
    /// Lower range bound.
    start: KeyBound<'key>,
    /// Upper range bound.
    end: KeyBound<'key>,
}

impl<'key> KeyRangeRef<'key>
{
    /// Creates a borrowed key range after rejecting reversed bounded ranges.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::RangeBound`] when both sides are bounded and
    /// the start key sorts after the end key.
    #[inline]
    pub fn new(
        start: KeyBound<'key>,
        end: KeyBound<'key>,
    ) -> Result<Self, ProllyBaoError>
    {
        if bounds_are_reversed(start, end) {
            return Err(ProllyBaoError::RangeBound {
                context: "start key sorts after end key",
            });
        }

        return Ok(Self { start, end });
    }

    /// Creates an unbounded key range.
    #[inline]
    #[must_use]
    pub const fn all() -> Self
    {
        return Self {
            start: KeyBound::Unbounded,
            end: KeyBound::Unbounded,
        };
    }

    /// Returns the lower range bound.
    #[inline]
    #[must_use]
    pub const fn start(&self) -> KeyBound<'key>
    {
        return self.start;
    }

    /// Returns the upper range bound.
    #[inline]
    #[must_use]
    pub const fn end(&self) -> KeyBound<'key>
    {
        return self.end;
    }
}

/// Returns `true` when bounded range keys are reversed.
#[inline]
fn bounds_are_reversed(
    start: KeyBound<'_>,
    end: KeyBound<'_>,
) -> bool
{
    match (start.key(), end.key()) {
        | (Some(start_key), Some(end_key)) => return start_key > end_key,
        | _ => return false,
    }
}

/// Borrowed encoded node bytes loaded from or offered to a block store.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct StoredNodeRef<'node>
{
    /// Expected opaque hash for the encoded node bytes.
    hash: NodeHash,
    /// Encoded node bytes.
    bytes: &'node [u8],
}

impl<'node> StoredNodeRef<'node>
{
    /// Creates a borrowed store-facing node reference.
    #[inline]
    #[must_use]
    pub const fn new(
        hash: NodeHash,
        bytes: &'node [u8],
    ) -> Self
    {
        return Self { hash, bytes };
    }

    /// Returns the expected node hash.
    #[inline]
    #[must_use]
    pub const fn node_hash(&self) -> NodeHash
    {
        return self.hash;
    }

    /// Returns the encoded node bytes.
    #[inline]
    #[must_use]
    pub const fn bytes(&self) -> &'node [u8]
    {
        return self.bytes;
    }
}

/// Kind of ordered-record proof represented by an envelope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ProofKind
{
    /// Proof for a present key.
    Membership,
    /// Proof for an absent key.
    NonMembership,
    /// Proof for an ordered key range.
    Range,
}

/// Proof metadata that commits root identity, encoding, and chunker parameters.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProofEnvelope
{
    /// Root against which the proof is interpreted.
    root: TreeRoot,
    /// Proof shape marker.
    kind: ProofKind,
    /// Encoding version committed by the proof.
    encoding_version: EncodingVersion,
    /// Chunker parameter bytes committed by the proof.
    chunker_parameter_bytes: Box<[u8]>,
}

impl ProofEnvelope
{
    /// Creates proof metadata from a root and proof kind.
    #[inline]
    #[must_use]
    pub fn new(
        root: TreeRoot,
        kind: ProofKind,
    ) -> Self
    {
        let encoding_version = root.params().encoding_version();
        let chunker_parameter_bytes = Box::<[u8]>::from(root.params().chunker_parameter_bytes());

        return Self {
            root,
            kind,
            encoding_version,
            chunker_parameter_bytes,
        };
    }

    /// Returns the root against which the proof is interpreted.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> &TreeRoot
    {
        return &self.root;
    }

    /// Returns the proof shape marker.
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> ProofKind
    {
        return self.kind;
    }

    /// Returns the encoding version committed by the proof.
    #[inline]
    #[must_use]
    pub const fn encoding_version(&self) -> EncodingVersion
    {
        return self.encoding_version;
    }

    /// Returns the chunker parameter bytes committed by the proof.
    #[inline]
    #[must_use]
    pub fn chunker_parameter_bytes(&self) -> &[u8]
    {
        return self.chunker_parameter_bytes.as_ref();
    }
}
