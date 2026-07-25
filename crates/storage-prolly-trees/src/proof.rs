//! Portable Prolly-Bao proof construction and verification.
//!
//! `current`: this module provides a small proof-oriented tree builder and
//! store-independent membership, non-membership, and range proofs. Verification
//! recomputes BLAKE3 node identity from carried node bytes and checks root
//! context before trusting any decoded record.
//!
//! `current`: compact node selection is implemented only for the one-level
//! internal-root tree shape produced by this proof tree: membership material
//! carries the root node plus selected leaf, non-membership material carries
//! the root node plus selected leaf and optional required successor leaf, and
//! range material carries the root node plus contiguous selected leaves.
//! Witness verification remains fail-closed by checking root binding before
//! enforcing root-first node order and delegating to the same store-independent
//! proof verifiers.
//!
//! `designed direction`: generalized multi-level compact proof selection is
//! outside the current contract; later tree/store modules may extend selection
//! while preserving the verifier contract.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cmp::Ordering;

use gandr_storage_chunker::CanonicalRecords;
use gandr_storage_chunker::chunk_record_slices;

use crate::error::ProllyBaoError;
use crate::types::EncodingVersion;
use crate::types::HashAlgorithm;
use crate::types::KeyBound;
use crate::types::KeyRangeRef;
use crate::types::NODE_HASH_LEN;
use crate::types::NodeHash;
#[cfg(feature = "proofs")]
use crate::types::ProofEnvelope;
#[cfg(feature = "proofs")]
use crate::types::ProofKind;
use crate::types::Record;
use crate::types::RecordRef;
use crate::types::SeparatorConvention;
use crate::types::StoredNodeRef;
use crate::types::TreeKind;
use crate::types::TreeParams;
use crate::types::TreeRoot;

/// Fixed node-domain marker included in every encoded node byte string.
const NODE_MAGIC: &[u8] = b"prolly-bao:node:v1";
/// Fixed root-domain marker included in root-manifest hash material.
const ROOT_MAGIC: &[u8] = b"prolly-bao:root:v1";
/// Fixed record-domain marker used only for chunker input framing.
const RECORD_MAGIC: &[u8] = b"prolly-bao:record:v1";
/// Encoded leaf-node kind tag.
const NODE_KIND_LEAF: u8 = 0x00_u8;
/// Encoded internal-node kind tag.
const NODE_KIND_INTERNAL: u8 = 0x01_u8;
/// Encoded Merkle-search tree kind tag.
const TREE_KIND_MERKLE_SEARCH: u8 = 0x01_u8;
/// Encoded BLAKE3 hash algorithm tag.
const HASH_ALGORITHM_BLAKE3: u8 = 0x01_u8;
/// Encoded first-key separator convention tag.
const SEPARATOR_FIRST_KEY: u8 = 0x01_u8;
/// Fixed transcript-domain marker included in native witness bytes.
#[cfg(feature = "proofs")]
const WITNESS_MAGIC: &[u8] = b"prolly-bao:witness:v1";
/// Native witness transcript version 1.
#[cfg(feature = "proofs")]
const WITNESS_VERSION: u16 = 1_u16;
/// Fixed snapshot-domain marker included in canonical snapshot byte streams.
const SNAPSHOT_MAGIC: &[u8] = b"prolly-bao:snapshot:v1";
/// Canonical Prolly-Bao snapshot byte-stream version 1.
const SNAPSHOT_VERSION: u16 = 1_u16;
/// Encoded membership witness kind tag.
#[cfg(feature = "proofs")]
const WITNESS_KIND_MEMBERSHIP: u8 = 0x01_u8;
/// Encoded non-membership witness kind tag.
#[cfg(feature = "proofs")]
const WITNESS_KIND_NON_MEMBERSHIP: u8 = 0x02_u8;
/// Encoded range witness kind tag.
#[cfg(feature = "proofs")]
const WITNESS_KIND_RANGE: u8 = 0x03_u8;
/// Encoded unbounded range-bound tag.
#[cfg(feature = "proofs")]
const WITNESS_BOUND_UNBOUNDED: u8 = 0x00_u8;
/// Encoded inclusive range-bound tag.
#[cfg(feature = "proofs")]
const WITNESS_BOUND_INCLUDED: u8 = 0x01_u8;
/// Encoded exclusive range-bound tag.
#[cfg(feature = "proofs")]
const WITNESS_BOUND_EXCLUDED: u8 = 0x02_u8;
/// Encoded optional-record absence tag.
#[cfg(feature = "proofs")]
const WITNESS_OPTION_NONE: u8 = 0x00_u8;
/// Encoded optional-record presence tag.
#[cfg(feature = "proofs")]
const WITNESS_OPTION_RECORD: u8 = 0x01_u8;
/// Fixed terminal-section marker included in native witness end summaries.
#[cfg(feature = "proofs")]
const WITNESS_END_SUMMARY_MAGIC: &[u8] = b"prolly-bao:witness-end-summary:v1";
/// Hash domain marker for committed witness chunker parameter bytes.
#[cfg(feature = "proofs")]
const WITNESS_CHUNKER_SUMMARY_MAGIC: &[u8] = b"prolly-bao:witness-chunker-summary:v1";
/// Hash domain marker for committed witness body material.
#[cfg(feature = "proofs")]
const WITNESS_BODY_SUMMARY_MAGIC: &[u8] = b"prolly-bao:witness-body-summary:v1";
/// Hash domain marker for committed witness proof-node material.
#[cfg(feature = "proofs")]
const WITNESS_NODES_SUMMARY_MAGIC: &[u8] = b"prolly-bao:witness-nodes-summary:v1";
/// Hash domain marker for committed witness end-summary fields.
#[cfg(feature = "proofs")]
const WITNESS_END_SUMMARY_BINDING_MAGIC: &[u8] = b"prolly-bao:witness-end-summary-binding:v1";

/// Summary node kind parsed from canonical encoded Prolly-Bao node bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum EncodedNodeKind
{
    /// Leaf node carrying ordered records.
    Leaf,
    /// Internal node carrying child references.
    Internal,
}

/// Borrowed inspection summary for canonical encoded Prolly-Bao node bytes.
///
/// `current`: this reports review-oriented node-layout metadata without
/// changing node bytes or the `BLAKE3(encoded_node_bytes)` identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct EncodedNodeLayout
{
    /// Parsed node kind.
    kind: EncodedNodeKind,
    /// Exact encoded byte length inspected by the parser.
    encoded_len: usize,
    /// Number of records carried by a leaf node.
    record_count: Option<u64>,
    /// Number of child references carried by an internal node.
    child_count: Option<u64>,
    /// Whether this encoded node represents an empty logical node.
    is_empty: bool,
}

impl EncodedNodeLayout
{
    /// Creates a leaf layout summary.
    #[inline]
    #[must_use]
    const fn leaf(
        encoded_len: usize,
        record_count: u64,
    ) -> Self
    {
        return Self {
            kind: EncodedNodeKind::Leaf,
            encoded_len,
            record_count: Some(record_count),
            child_count: None,
            is_empty: record_count == 0_u64,
        };
    }

    /// Creates an internal layout summary.
    #[inline]
    #[must_use]
    const fn internal(
        encoded_len: usize,
        child_count: u64,
    ) -> Self
    {
        return Self {
            kind: EncodedNodeKind::Internal,
            encoded_len,
            record_count: None,
            child_count: Some(child_count),
            is_empty: false,
        };
    }

    /// Returns the parsed node kind.
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> EncodedNodeKind
    {
        return self.kind;
    }

    /// Returns the exact encoded byte length inspected by the parser.
    #[inline]
    #[must_use]
    pub const fn encoded_len(&self) -> usize
    {
        return self.encoded_len;
    }

    /// Returns the leaf record count, or `None` for internal nodes.
    #[inline]
    #[must_use]
    pub const fn record_count(&self) -> Option<u64>
    {
        return self.record_count;
    }

    /// Returns the internal child count, or `None` for leaf nodes.
    #[inline]
    #[must_use]
    pub const fn child_count(&self) -> Option<u64>
    {
        return self.child_count;
    }

    /// Returns whether the encoded node represents an empty logical node.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool
    {
        return self.is_empty;
    }
}

/// Native Prolly-Bao witness transcript kind.
///
/// `current`: these variants describe ordered-record query-response
/// transcripts under an agreed [`TreeRoot`]. They are not Bao byte-stream proof
/// kinds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
#[cfg(feature = "proofs")]
pub enum WitnessKind
{
    /// Transcript for a present key and its authenticated value.
    Membership,
    /// Transcript for an absent key and authenticated adjacent-key evidence.
    NonMembership,
    /// Transcript for a complete ordered key range.
    Range,
}

#[cfg(feature = "proofs")]
impl WitnessKind
{
    /// Returns the deterministic binary tag for this witness kind.
    #[inline]
    const fn tag(self) -> u8
    {
        match self {
            | Self::Membership => return WITNESS_KIND_MEMBERSHIP,
            | Self::NonMembership => return WITNESS_KIND_NON_MEMBERSHIP,
            | Self::Range => return WITNESS_KIND_RANGE,
        }
    }

    /// Returns the corresponding existing proof kind.
    #[inline]
    const fn proof_kind(self) -> ProofKind
    {
        match self {
            | Self::Membership => return ProofKind::Membership,
            | Self::NonMembership => return ProofKind::NonMembership,
            | Self::Range => return ProofKind::Range,
        }
    }

    /// Decodes a deterministic binary witness kind tag.
    #[inline]
    const fn from_tag(tag: u8) -> Result<Self, ProllyBaoError>
    {
        match tag {
            | WITNESS_KIND_MEMBERSHIP => return Ok(Self::Membership),
            | WITNESS_KIND_NON_MEMBERSHIP => return Ok(Self::NonMembership),
            | WITNESS_KIND_RANGE => return Ok(Self::Range),
            | _ => {
                return Err(ProllyBaoError::MalformedWitnessBytes {
                    context: "unknown witness kind",
                });
            },
        }
    }
}

/// Native Prolly-Bao witness transcript body.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(feature = "proofs")]
enum WitnessBody
{
    /// Membership query and returned value.
    Membership
    {
        /// Queried key.
        key: Box<[u8]>,
        /// Returned authenticated value.
        value: Box<[u8]>,
    },
    /// Non-membership query and returned adjacent-key evidence.
    NonMembership
    {
        /// Queried absent key.
        key: Box<[u8]>,
        /// Returned authenticated adjacent-key evidence.
        evidence: NonMembershipEvidence,
    },
    /// Range query and returned records.
    Range
    {
        /// Queried range.
        range: OwnedKeyRange,
        /// Returned authenticated records.
        records: Box<[Record]>,
    },
}

/// Required terminal summary for a native Prolly-Bao witness transcript.
///
/// `current`: this summary is a deterministic Prolly-Bao-native binding over
/// witness query-response material and proof nodes. The digest fields are not
/// Bao hashes and do not claim Bao wire-format compatibility.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[cfg(feature = "proofs")]
pub struct WitnessEndSummary
{
    /// Explicit native witness transcript version.
    version: u16,
    /// Transcript shape marker.
    kind: WitnessKind,
    /// Agreed Prolly-Bao root hash named by this transcript.
    root_hash: NodeHash,
    /// Number of records committed by the agreed root.
    root_record_count: u64,
    /// Digest of chunker parameter bytes copied from the root context.
    chunker_parameter_digest: NodeHash,
    /// Root node hash named by the root manifest.
    root_node_hash: NodeHash,
    /// Digest of kind-specific query and returned material.
    body_digest: NodeHash,
    /// Number of encoded proof nodes committed by this summary.
    proof_node_count: u64,
    /// Digest of encoded proof-node hashes and byte strings.
    proof_nodes_digest: NodeHash,
    /// Digest binding all summary fields together.
    binding_digest: NodeHash,
}

#[cfg(feature = "proofs")]
impl WitnessEndSummary
{
    /// Returns the explicit witness transcript version bound by this summary.
    #[inline]
    #[must_use]
    pub const fn version(&self) -> u16
    {
        return self.version;
    }

    /// Returns the native Prolly-Bao witness kind bound by this summary.
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> WitnessKind
    {
        return self.kind;
    }

    /// Returns the agreed Prolly-Bao root hash bound by this summary.
    #[inline]
    #[must_use]
    pub const fn root_hash(&self) -> NodeHash
    {
        return self.root_hash;
    }

    /// Returns the agreed Prolly-Bao root record count bound by this summary.
    #[inline]
    #[must_use]
    pub const fn root_record_count(&self) -> u64
    {
        return self.root_record_count;
    }

    /// Returns the digest of committed chunker parameter bytes.
    #[inline]
    #[must_use]
    pub const fn chunker_parameter_digest(&self) -> NodeHash
    {
        return self.chunker_parameter_digest;
    }

    /// Returns the root node hash bound by this summary.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.root_node_hash;
    }

    /// Returns the digest of kind-specific query and returned material.
    #[inline]
    #[must_use]
    pub const fn body_digest(&self) -> NodeHash
    {
        return self.body_digest;
    }

    /// Returns the number of encoded proof nodes bound by this summary.
    #[inline]
    #[must_use]
    pub const fn proof_node_count(&self) -> u64
    {
        return self.proof_node_count;
    }

    /// Returns the digest of encoded proof-node hashes and byte strings.
    #[inline]
    #[must_use]
    pub const fn proof_nodes_digest(&self) -> NodeHash
    {
        return self.proof_nodes_digest;
    }

    /// Returns the digest binding all summary fields together.
    #[inline]
    #[must_use]
    pub const fn binding_digest(&self) -> NodeHash
    {
        return self.binding_digest;
    }

    /// Decodes the required terminal witness end-summary section.
    fn decode(cursor: &mut WitnessCursor<'_>) -> Result<Self, ProllyBaoError>
    {
        let magic = cursor.take(
            WITNESS_END_SUMMARY_MAGIC.len(),
            "witness end summary magic is truncated",
        )?;

        if magic != WITNESS_END_SUMMARY_MAGIC {
            return Err(ProllyBaoError::MalformedWitnessBytes {
                context: "witness end summary magic mismatch",
            });
        }

        let version = cursor.read_u16()?;
        let kind = WitnessKind::from_tag(cursor.read_u8()?)?;
        let root_hash = NodeHash::from_bytes(
            cursor.take_array::<NODE_HASH_LEN>("witness end summary root hash is truncated")?,
        );
        let root_record_count = cursor.read_u64()?;
        let chunker_parameter_digest = NodeHash::from_bytes(cursor.take_array::<NODE_HASH_LEN>(
            "witness end summary chunker parameter digest is truncated",
        )?);
        let root_node_hash = NodeHash::from_bytes(
            cursor
                .take_array::<NODE_HASH_LEN>("witness end summary root node hash is truncated")?,
        );
        let body_digest = NodeHash::from_bytes(
            cursor.take_array::<NODE_HASH_LEN>("witness end summary body digest is truncated")?,
        );
        let proof_node_count = cursor.read_u64()?;
        let proof_nodes_digest =
            NodeHash::from_bytes(cursor.take_array::<NODE_HASH_LEN>(
                "witness end summary proof nodes digest is truncated",
            )?);
        let binding_digest = NodeHash::from_bytes(
            cursor
                .take_array::<NODE_HASH_LEN>("witness end summary binding digest is truncated")?,
        );

        return Ok(Self {
            version,
            kind,
            root_hash,
            root_record_count,
            chunker_parameter_digest,
            root_node_hash,
            body_digest,
            proof_node_count,
            proof_nodes_digest,
            binding_digest,
        });
    }

    /// Encodes the terminal witness end-summary section.
    fn encode(
        &self,
        out: &mut Vec<u8>,
    )
    {
        out.extend_from_slice(WITNESS_END_SUMMARY_MAGIC);
        push_u16(out, self.version);
        out.push(self.kind.tag());
        out.extend_from_slice(self.root_hash.as_slice());
        push_u64(out, self.root_record_count);
        out.extend_from_slice(self.chunker_parameter_digest.as_slice());
        out.extend_from_slice(self.root_node_hash.as_slice());
        out.extend_from_slice(self.body_digest.as_slice());
        push_u64(out, self.proof_node_count);
        out.extend_from_slice(self.proof_nodes_digest.as_slice());
        out.extend_from_slice(self.binding_digest.as_slice());
    }
}

/// Versioned native Prolly-Bao ordered-record witness transcript.
///
/// `current`: a witness transcript is a deterministic Prolly-Bao-native
/// query-response byte format for membership, non-membership, and range
/// verification under an agreed [`TreeRoot`] and [`TreeParams`]. It is not a
/// Bao byte-stream proof and does not claim Bao wire-format compatibility.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[cfg(feature = "proofs")]
pub struct WitnessTranscript
{
    /// Explicit native witness transcript version.
    version: u16,
    /// Transcript shape marker.
    kind: WitnessKind,
    /// Agreed Prolly-Bao root hash named by this transcript.
    root_hash: NodeHash,
    /// Number of records committed by the agreed root.
    root_record_count: u64,
    /// Chunker parameter commitment bytes copied from the proof root context.
    chunker_parameter_bytes: Box<[u8]>,
    /// Root node hash named by the root manifest.
    root_node_hash: NodeHash,
    /// Kind-specific query and returned material.
    body: WitnessBody,
    /// Encoded proof nodes carried by this transcript.
    ///
    /// `current`: one-level internal-root transcripts preserve the proof's
    /// compact node material and verifier-required root-first order.
    nodes: Box<[ProofNode]>,
    /// Required terminal summary binding this transcript.
    end_summary: WitnessEndSummary,
}

#[cfg(feature = "proofs")]
impl WitnessTranscript
{
    /// Native witness transcript version 1.
    pub const VERSION: u16 = WITNESS_VERSION;

    /// Creates a native Prolly-Bao witness transcript from a membership proof.
    ///
    /// `current`: the resulting transcript remains a Prolly-Bao ordered-record
    /// transcript, not a Bao byte-stream proof.
    ///
    /// # Errors
    ///
    /// Returns an error when the proof envelope is not internally compatible
    /// with a membership witness transcript.
    #[inline]
    pub fn from_membership_proof(proof: MembershipProof) -> Result<Self, ProllyBaoError>
    {
        let root_node_hash = proof.root_node_hash;
        ensure_proof_can_form_witness(proof.envelope(), root_node_hash, ProofKind::Membership)?;
        let (root_hash, root_record_count, chunker_parameter_bytes) =
            witness_root_parts(proof.envelope());
        let MembershipProof {
            key, value, nodes, ..
        } = proof;

        let body = WitnessBody::Membership { key, value };
        let end_summary = compute_witness_end_summary(&WitnessSummaryMaterial {
            version: Self::VERSION,
            kind: WitnessKind::Membership,
            root_hash,
            root_record_count,
            chunker_parameter_bytes: chunker_parameter_bytes.as_ref(),
            root_node_hash,
            body: &body,
            nodes: nodes.as_ref(),
        })?;

        return Ok(Self {
            version: Self::VERSION,
            kind: WitnessKind::Membership,
            root_hash,
            root_record_count,
            chunker_parameter_bytes,
            root_node_hash,
            body,
            nodes,
            end_summary,
        });
    }

    /// Creates a native Prolly-Bao witness transcript from a non-membership
    /// proof.
    ///
    /// `current`: the resulting transcript remains a Prolly-Bao ordered-record
    /// transcript, not a Bao byte-stream proof.
    ///
    /// # Errors
    ///
    /// Returns an error when the proof envelope is not internally compatible
    /// with a non-membership witness transcript.
    #[inline]
    pub fn from_non_membership_proof(proof: NonMembershipProof) -> Result<Self, ProllyBaoError>
    {
        let root_node_hash = proof.root_node_hash;
        ensure_proof_can_form_witness(proof.envelope(), root_node_hash, ProofKind::NonMembership)?;
        let (root_hash, root_record_count, chunker_parameter_bytes) =
            witness_root_parts(proof.envelope());
        let NonMembershipProof {
            key,
            evidence,
            nodes,
            ..
        } = proof;

        let body = WitnessBody::NonMembership { key, evidence };
        let end_summary = compute_witness_end_summary(&WitnessSummaryMaterial {
            version: Self::VERSION,
            kind: WitnessKind::NonMembership,
            root_hash,
            root_record_count,
            chunker_parameter_bytes: chunker_parameter_bytes.as_ref(),
            root_node_hash,
            body: &body,
            nodes: nodes.as_ref(),
        })?;

        return Ok(Self {
            version: Self::VERSION,
            kind: WitnessKind::NonMembership,
            root_hash,
            root_record_count,
            chunker_parameter_bytes,
            root_node_hash,
            body,
            nodes,
            end_summary,
        });
    }

    /// Creates a native Prolly-Bao witness transcript from a range proof.
    ///
    /// `current`: the resulting transcript remains a Prolly-Bao ordered-record
    /// transcript, not a Bao byte-stream proof.
    ///
    /// # Errors
    ///
    /// Returns an error when the proof envelope is not internally compatible
    /// with a range witness transcript.
    #[inline]
    pub fn from_range_proof(proof: RangeProof) -> Result<Self, ProllyBaoError>
    {
        let root_node_hash = proof.root_node_hash;
        ensure_proof_can_form_witness(proof.envelope(), root_node_hash, ProofKind::Range)?;
        let (root_hash, root_record_count, chunker_parameter_bytes) =
            witness_root_parts(proof.envelope());
        let RangeProof {
            range,
            records,
            nodes,
            ..
        } = proof;

        let body = WitnessBody::Range { range, records };
        let end_summary = compute_witness_end_summary(&WitnessSummaryMaterial {
            version: Self::VERSION,
            kind: WitnessKind::Range,
            root_hash,
            root_record_count,
            chunker_parameter_bytes: chunker_parameter_bytes.as_ref(),
            root_node_hash,
            body: &body,
            nodes: nodes.as_ref(),
        })?;

        return Ok(Self {
            version: Self::VERSION,
            kind: WitnessKind::Range,
            root_hash,
            root_record_count,
            chunker_parameter_bytes,
            root_node_hash,
            body,
            nodes,
            end_summary,
        });
    }

    /// Decodes native Prolly-Bao witness transcript bytes.
    ///
    /// `current`: the decoded bytes are interpreted only as a Prolly-Bao
    /// ordered-record transcript, not as a Bao byte-stream proof.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::MalformedWitnessBytes`] for malformed or
    /// truncated transcript bytes,
    /// [`ProllyBaoError::UnsupportedWitnessVersion`] for unsupported
    /// witness versions, or range/proof errors for invalid decoded query
    /// material.
    #[inline]
    pub fn decode(bytes: &[u8]) -> Result<Self, ProllyBaoError>
    {
        let mut cursor = WitnessCursor::new(bytes);
        let magic = cursor.take(WITNESS_MAGIC.len(), "witness magic is truncated")?;

        if magic != WITNESS_MAGIC {
            return Err(ProllyBaoError::MalformedWitnessBytes {
                context: "witness magic mismatch",
            });
        }

        let version = cursor.read_u16()?;

        if version != Self::VERSION {
            return Err(ProllyBaoError::UnsupportedWitnessVersion { version });
        }

        let kind = WitnessKind::from_tag(cursor.read_u8()?)?;
        let root_hash = NodeHash::from_bytes(
            cursor.take_array::<NODE_HASH_LEN>("witness root hash is truncated")?,
        );
        let root_record_count = cursor.read_u64()?;
        let chunker_parameter_bytes =
            decode_witness_bytes(&mut cursor, "witness chunker parameter bytes are truncated")?;
        let root_node_hash = NodeHash::from_bytes(
            cursor.take_array::<NODE_HASH_LEN>("witness root node hash is truncated")?,
        );
        let body = decode_witness_body(&mut cursor, kind)?;
        let nodes = decode_witness_nodes(&mut cursor)?;
        let end_summary = WitnessEndSummary::decode(&mut cursor)?;
        let expected_end_summary = compute_witness_end_summary(&WitnessSummaryMaterial {
            version,
            kind,
            root_hash,
            root_record_count,
            chunker_parameter_bytes: chunker_parameter_bytes.as_ref(),
            root_node_hash,
            body: &body,
            nodes: nodes.as_ref(),
        })?;

        if end_summary != expected_end_summary {
            return Err(ProllyBaoError::MalformedWitnessBytes {
                context: "witness end summary mismatch",
            });
        }

        if !cursor.is_empty() {
            return Err(ProllyBaoError::MalformedWitnessBytes {
                context: "trailing witness bytes",
            });
        }

        return Ok(Self {
            version,
            kind,
            root_hash,
            root_record_count,
            chunker_parameter_bytes,
            root_node_hash,
            body,
            nodes,
            end_summary,
        });
    }

    /// Appends deterministic native Prolly-Bao witness bytes to `out`.
    ///
    /// `current`: the emitted bytes are a Prolly-Bao ordered-record transcript,
    /// not a Bao byte-stream proof. Existing contents of `out` are preserved
    /// and the transcript is appended.
    ///
    /// # Errors
    ///
    /// Returns an error when a transcript length cannot be represented in the
    /// deterministic binary format.
    #[inline]
    pub fn encode(
        &self,
        out: &mut Vec<u8>,
    ) -> Result<(), ProllyBaoError>
    {
        out.reserve(self.encoded_len()?);
        self.encode_unreserved(out)?;

        return Ok(());
    }

    /// Encodes this transcript into owned deterministic bytes.
    ///
    /// `current`: the emitted bytes are a Prolly-Bao ordered-record transcript,
    /// not a Bao byte-stream proof.
    ///
    /// # Errors
    ///
    /// Returns an error when a transcript length cannot be represented in the
    /// deterministic binary format.
    #[inline]
    pub fn to_bytes(&self) -> Result<Box<[u8]>, ProllyBaoError>
    {
        let capacity = self.encoded_len()?;
        let mut bytes = Vec::<u8>::with_capacity(capacity);
        self.encode_unreserved(&mut bytes)?;

        return Ok(bytes.into_boxed_slice());
    }

    /// Returns the explicit witness transcript version.
    #[inline]
    #[must_use]
    pub const fn version(&self) -> u16
    {
        return self.version;
    }

    /// Returns the native Prolly-Bao witness kind.
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> WitnessKind
    {
        return self.kind;
    }

    /// Returns the agreed Prolly-Bao root hash named by this transcript.
    #[inline]
    #[must_use]
    pub const fn root_hash(&self) -> NodeHash
    {
        return self.root_hash;
    }

    /// Returns the agreed Prolly-Bao root record count.
    #[inline]
    #[must_use]
    pub const fn root_record_count(&self) -> u64
    {
        return self.root_record_count;
    }

    /// Returns the committed chunker parameter bytes carried by this
    /// transcript.
    #[inline]
    #[must_use]
    pub fn chunker_parameter_bytes(&self) -> &[u8]
    {
        return self.chunker_parameter_bytes.as_ref();
    }

    /// Returns the root node hash named by the root manifest.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.root_node_hash;
    }

    /// Returns encoded proof nodes carried by this transcript.
    ///
    /// `current`: one-level internal-root witnesses carry root-first compact
    /// material: membership root plus selected leaf, non-membership root plus
    /// selected leaf and optional required successor leaf, and range root plus
    /// contiguous selected leaves.
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> &[ProofNode]
    {
        return self.nodes.as_ref();
    }

    /// Returns the decoded terminal witness end summary.
    #[inline]
    #[must_use]
    pub const fn end_summary(&self) -> &WitnessEndSummary
    {
        return &self.end_summary;
    }

    /// Verifies a membership witness against explicit root, parameter, key, and
    /// value expectations.
    ///
    /// `current`: verification is native Prolly-Bao ordered-record verification
    /// and does not require Bao wrapping. It fails closed by checking root
    /// binding before requiring carried node material to be root-first.
    ///
    /// # Errors
    ///
    /// Returns an error when witness version, kind, root binding, chunker
    /// parameter commitment, query material, node bytes, path shape, or
    /// expected value do not match authenticated data.
    #[inline]
    pub fn verify_membership(
        &self,
        expected_root: &TreeRoot,
        expected_params: &TreeParams,
        expected_key: &[u8],
        expected_value: &[u8],
    ) -> Result<(), ProllyBaoError>
    {
        self.verify_binding(expected_root, expected_params, WitnessKind::Membership)?;
        let (key, value) = self.membership_body()?;

        if key != expected_key {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "membership witness key does not match verifier query",
            });
        }

        if value != expected_value {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "membership witness value does not match verifier expectation",
            });
        }

        verify_membership_witness_node_order(self.root_node_hash, key, self.nodes.as_ref())?;

        let envelope =
            ProofEnvelope::new(expected_root.clone(), WitnessKind::Membership.proof_kind());

        let input = VerifierInput {
            envelope: &envelope,
            root_node_hash: self.root_node_hash,
            nodes: self.nodes.as_ref(),
            expected_root,
            expected_params,
        };

        return verify_membership_material(&input, key, value, expected_key, expected_value);
    }

    /// Verifies a non-membership witness against explicit root, parameter, and
    /// absent-key expectations.
    ///
    /// `current`: verification is native Prolly-Bao ordered-record verification
    /// and does not require Bao wrapping. It fails closed by checking root
    /// binding before requiring carried node material to be root-first.
    ///
    /// # Errors
    ///
    /// Returns an error when witness version, kind, root binding, chunker
    /// parameter commitment, query material, node bytes, or adjacent-key
    /// evidence do not match authenticated data.
    #[inline]
    pub fn verify_non_membership(
        &self,
        expected_root: &TreeRoot,
        expected_params: &TreeParams,
        expected_key: &[u8],
    ) -> Result<NonMembershipEvidence, ProllyBaoError>
    {
        self.verify_binding(expected_root, expected_params, WitnessKind::NonMembership)?;
        let (key, evidence) = self.non_membership_body()?;

        if key != expected_key {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "non-membership witness key does not match verifier query",
            });
        }

        verify_non_membership_witness_node_order(
            self.root_node_hash,
            expected_key,
            self.nodes.as_ref(),
        )?;

        let envelope = ProofEnvelope::new(
            expected_root.clone(),
            WitnessKind::NonMembership.proof_kind(),
        );

        let input = VerifierInput {
            envelope: &envelope,
            root_node_hash: self.root_node_hash,
            nodes: self.nodes.as_ref(),
            expected_root,
            expected_params,
        };

        return verify_non_membership_material(&input, key, evidence, expected_key);
    }

    /// Verifies a range witness against explicit root, parameter, and range
    /// expectations.
    ///
    /// `current`: verification is native Prolly-Bao ordered-record verification
    /// and does not require Bao wrapping. It fails closed by checking root
    /// binding before requiring carried node material to be root-first.
    ///
    /// # Errors
    ///
    /// Returns an error when witness version, kind, root binding, chunker
    /// parameter commitment, query range, node bytes, or returned records do
    /// not match authenticated data.
    #[inline]
    pub fn verify_range(
        &self,
        expected_root: &TreeRoot,
        expected_params: &TreeParams,
        expected_range: KeyRangeRef<'_>,
    ) -> Result<Box<[Record]>, ProllyBaoError>
    {
        self.verify_binding(expected_root, expected_params, WitnessKind::Range)?;
        let (range, records) = self.range_body()?;
        let expected_owned_range = OwnedKeyRange::from_ref(expected_range);

        if range != &expected_owned_range {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "range witness bounds do not match verifier query",
            });
        }

        verify_range_witness_node_order(self.root_node_hash, expected_range, self.nodes.as_ref())?;

        let envelope = ProofEnvelope::new(expected_root.clone(), WitnessKind::Range.proof_kind());

        let input = VerifierInput {
            envelope: &envelope,
            root_node_hash: self.root_node_hash,
            nodes: self.nodes.as_ref(),
            expected_root,
            expected_params,
        };

        return verify_range_material(&input, range, records, expected_range);
    }

    /// Computes exact deterministic encoded length.
    fn encoded_len(&self) -> Result<usize, ProllyBaoError>
    {
        let mut len = WITNESS_MAGIC.len();
        checked_add_to_len(&mut len, 2_usize, "witness encoded length overflow")?;
        checked_add_to_len(&mut len, 1_usize, "witness encoded length overflow")?;
        checked_add_to_len(&mut len, NODE_HASH_LEN, "witness encoded length overflow")?;
        checked_add_to_len(&mut len, 8_usize, "witness encoded length overflow")?;
        checked_add_to_len(
            &mut len,
            len_prefixed_bytes_encoded_len(self.chunker_parameter_bytes.as_ref())?,
            "witness encoded length overflow",
        )?;
        checked_add_to_len(&mut len, NODE_HASH_LEN, "witness encoded length overflow")?;
        checked_add_to_len(
            &mut len,
            witness_body_encoded_len(&self.body)?,
            "witness encoded length overflow",
        )?;
        checked_add_to_len(&mut len, 8_usize, "witness encoded length overflow")?;

        for node in self.nodes.as_ref() {
            checked_add_to_len(&mut len, NODE_HASH_LEN, "witness encoded length overflow")?;
            checked_add_to_len(
                &mut len,
                len_prefixed_bytes_encoded_len(node.bytes())?,
                "witness encoded length overflow",
            )?;
        }
        checked_add_to_len(
            &mut len,
            witness_end_summary_encoded_len(),
            "witness encoded length overflow",
        )?;

        return Ok(len);
    }

    /// Encodes without reserving capacity.
    fn encode_unreserved(
        &self,
        out: &mut Vec<u8>,
    ) -> Result<(), ProllyBaoError>
    {
        out.extend_from_slice(WITNESS_MAGIC);
        push_u16(out, self.version);
        out.push(self.kind.tag());
        out.extend_from_slice(self.root_hash.as_slice());
        push_u64(out, self.root_record_count);
        push_len_prefixed_bytes(out, self.chunker_parameter_bytes.as_ref())?;
        out.extend_from_slice(self.root_node_hash.as_slice());
        encode_witness_body(&self.body, out)?;
        push_u64(
            out,
            u64_from_usize(self.nodes.len(), "witness node count does not fit u64")?,
        );

        for node in self.nodes.as_ref() {
            out.extend_from_slice(node.hash().as_slice());
            push_len_prefixed_bytes(out, node.bytes())?;
        }

        self.end_summary.encode(out);

        return Ok(());
    }

    /// Verifies root, parameter, version, and kind binding.
    fn verify_binding(
        &self,
        expected_root: &TreeRoot,
        expected_params: &TreeParams,
        expected_kind: WitnessKind,
    ) -> Result<(), ProllyBaoError>
    {
        if self.version != Self::VERSION {
            return Err(ProllyBaoError::UnsupportedWitnessVersion {
                version: self.version,
            });
        }

        if self.kind != expected_kind {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "witness kind does not match verifier request",
            });
        }

        if self.root_hash != expected_root.hash() {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "witness root hash does not match verifier root",
            });
        }

        if self.root_record_count != expected_root.record_count() {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "witness root record count does not match verifier root",
            });
        }

        if expected_root.params() != expected_params {
            return Err(ProllyBaoError::IncompatibleTreeParameters {
                context: "expected root parameters do not match verifier parameters",
            });
        }

        if self.chunker_parameter_bytes.as_ref() != expected_params.chunker_parameter_bytes() {
            return Err(ProllyBaoError::IncompatibleTreeParameters {
                context: "witness chunker parameter commitment mismatch",
            });
        }

        return Ok(());
    }

    /// Returns the membership body.
    fn membership_body(&self) -> Result<(&[u8], &[u8]), ProllyBaoError>
    {
        match self.body {
            | WitnessBody::Membership { ref key, ref value } => {
                return Ok((key.as_ref(), value.as_ref()));
            },
            | WitnessBody::NonMembership { .. } | WitnessBody::Range { .. } => {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "witness body does not match membership kind",
                });
            },
        }
    }

    /// Returns the non-membership body.
    fn non_membership_body(&self) -> Result<(&[u8], &NonMembershipEvidence), ProllyBaoError>
    {
        match self.body {
            | WitnessBody::NonMembership {
                ref key,
                ref evidence,
            } => return Ok((key.as_ref(), evidence)),
            | WitnessBody::Membership { .. } | WitnessBody::Range { .. } => {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "witness body does not match non-membership kind",
                });
            },
        }
    }

    /// Returns the range body.
    fn range_body(&self) -> Result<(&OwnedKeyRange, &[Record]), ProllyBaoError>
    {
        match self.body {
            | WitnessBody::Range {
                ref range,
                ref records,
            } => return Ok((range, records.as_ref())),
            | WitnessBody::Membership { .. } | WitnessBody::NonMembership { .. } => {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "witness body does not match range kind",
                });
            },
        }
    }
}

/// Encoded node bytes carried by a portable proof.
///
/// `current`: for the implemented one-level internal-root tree shape, compact
/// proof and witness material carries only query-selected nodes: membership
/// root plus selected leaf, non-membership root plus selected leaf and optional
/// required successor leaf, or range root plus contiguous selected leaves.
/// Multi-level compact selection is not promised by this type.
#[expect(
    clippy::module_name_repetitions,
    reason = "ProofNode is the explicit public proof node vocabulary"
)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProofNode
{
    /// Claimed BLAKE3 identity for `bytes`.
    hash: NodeHash,
    /// Encoded node bytes.
    bytes: Box<[u8]>,
}

impl ProofNode
{
    /// Creates a proof node from a claimed hash and encoded bytes.
    #[inline]
    #[must_use]
    pub fn new<B>(
        hash: NodeHash,
        bytes: B,
    ) -> Self
    where
        B: Into<Box<[u8]>>,
    {
        return Self {
            hash,
            bytes: bytes.into(),
        };
    }

    /// Returns the claimed BLAKE3 node hash.
    #[inline]
    #[must_use]
    pub const fn hash(&self) -> NodeHash
    {
        return self.hash;
    }

    /// Returns encoded node bytes.
    #[inline]
    #[must_use]
    pub fn bytes(&self) -> &[u8]
    {
        return self.bytes.as_ref();
    }

    /// Splits this proof node into its claimed hash and owned encoded bytes.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (NodeHash, Box<[u8]>)
    {
        return (self.hash, self.bytes);
    }
}

/// Owned key bound stored inside range proofs.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[cfg(feature = "proofs")]
pub enum OwnedKeyBound
{
    /// No bound on this side of the range.
    Unbounded,
    /// Bound includes the named key.
    Included(Box<[u8]>),
    /// Bound excludes the named key.
    Excluded(Box<[u8]>),
}

#[cfg(feature = "proofs")]
impl OwnedKeyBound
{
    /// Copies a borrowed bound into owned proof material.
    #[inline]
    #[must_use]
    pub fn from_ref(bound: KeyBound<'_>) -> Self
    {
        match bound {
            | KeyBound::Unbounded => return Self::Unbounded,
            | KeyBound::Included(key) => return Self::Included(Box::<[u8]>::from(key)),
            | KeyBound::Excluded(key) => return Self::Excluded(Box::<[u8]>::from(key)),
        }
    }

    /// Returns a borrowed view of this owned key bound.
    #[inline]
    #[must_use]
    pub fn as_ref(&self) -> KeyBound<'_>
    {
        match *self {
            | Self::Unbounded => return KeyBound::Unbounded,
            | Self::Included(ref key) => return KeyBound::Included(key.as_ref()),
            | Self::Excluded(ref key) => return KeyBound::Excluded(key.as_ref()),
        }
    }
}

/// Owned key range stored inside range proofs.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[cfg(feature = "proofs")]
pub struct OwnedKeyRange
{
    /// Lower range bound.
    start: OwnedKeyBound,
    /// Upper range bound.
    end: OwnedKeyBound,
}

#[cfg(feature = "proofs")]
impl OwnedKeyRange
{
    /// Copies a borrowed range into owned proof material.
    #[inline]
    #[must_use]
    pub fn from_ref(range: KeyRangeRef<'_>) -> Self
    {
        return Self {
            start: OwnedKeyBound::from_ref(range.start()),
            end: OwnedKeyBound::from_ref(range.end()),
        };
    }

    /// Creates a borrowed range view.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::RangeBound`] when the stored bounds are
    /// reversed.
    #[inline]
    pub fn as_range_ref(&self) -> Result<KeyRangeRef<'_>, ProllyBaoError>
    {
        return KeyRangeRef::new(self.start.as_ref(), self.end.as_ref());
    }

    /// Returns the lower bound.
    #[inline]
    #[must_use]
    pub const fn start(&self) -> &OwnedKeyBound
    {
        return &self.start;
    }

    /// Returns the upper bound.
    #[inline]
    #[must_use]
    pub const fn end(&self) -> &OwnedKeyBound
    {
        return &self.end;
    }
}

/// Store-independent proof that a key maps to a specific value.
#[expect(
    clippy::module_name_repetitions,
    reason = "MembershipProof is the explicit public proof vocabulary"
)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[cfg(feature = "proofs")]
pub struct MembershipProof
{
    /// Proof metadata and committed root context.
    envelope: ProofEnvelope,
    /// Root node hash named by the root manifest.
    root_node_hash: NodeHash,
    /// Queried key bound by this proof.
    key: Box<[u8]>,
    /// Value bound to `key` by this proof.
    value: Box<[u8]>,
    /// Encoded proof nodes; for one-level internal roots, root plus selected
    /// leaf.
    nodes: Box<[ProofNode]>,
}

#[cfg(feature = "proofs")]
impl MembershipProof
{
    /// Creates membership proof material.
    #[inline]
    #[must_use]
    pub fn new<K, V, N>(
        envelope: ProofEnvelope,
        root_node_hash: NodeHash,
        key: K,
        value: V,
        nodes: N,
    ) -> Self
    where
        K: Into<Box<[u8]>>,
        V: Into<Box<[u8]>>,
        N: Into<Box<[ProofNode]>>,
    {
        return Self {
            envelope,
            root_node_hash,
            key: key.into(),
            value: value.into(),
            nodes: nodes.into(),
        };
    }

    /// Returns proof metadata.
    #[inline]
    #[must_use]
    pub const fn envelope(&self) -> &ProofEnvelope
    {
        return &self.envelope;
    }

    /// Returns the root node hash named by the root manifest.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.root_node_hash;
    }

    /// Returns the queried key carried by this proof.
    #[inline]
    #[must_use]
    pub fn key(&self) -> &[u8]
    {
        return self.key.as_ref();
    }

    /// Returns the value carried by this proof.
    #[inline]
    #[must_use]
    pub fn value(&self) -> &[u8]
    {
        return self.value.as_ref();
    }

    /// Returns encoded proof nodes.
    ///
    /// `current`: one-level internal-root membership proofs carry the root node
    /// followed by the selected leaf node.
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> &[ProofNode]
    {
        return self.nodes.as_ref();
    }

    /// Verifies this proof against explicit root, parameter, key, and value
    /// expectations.
    ///
    /// # Errors
    ///
    /// Returns an error when root context, chunker parameters, node bytes, path
    /// shape, queried key, or expected value do not match authenticated data.
    #[inline]
    pub fn verify(
        &self,
        expected_root: &TreeRoot,
        expected_params: &TreeParams,
        expected_key: &[u8],
        expected_value: &[u8],
    ) -> Result<(), ProllyBaoError>
    {
        let input = VerifierInput {
            envelope: &self.envelope,
            root_node_hash: self.root_node_hash,
            nodes: self.nodes.as_ref(),
            expected_root,
            expected_params,
        };

        return verify_membership_material(
            &input,
            self.key.as_ref(),
            self.value.as_ref(),
            expected_key,
            expected_value,
        );
    }
}

/// Authenticated predecessor/successor evidence for an absent key.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[cfg(feature = "proofs")]
pub struct NonMembershipEvidence
{
    /// Greatest authenticated record whose key sorts before the absent key.
    predecessor: Option<Record>,
    /// Smallest authenticated record whose key sorts after the absent key.
    successor: Option<Record>,
}

#[cfg(feature = "proofs")]
impl NonMembershipEvidence
{
    /// Creates adjacent-key absence evidence.
    #[inline]
    #[must_use]
    pub const fn new(
        predecessor: Option<Record>,
        successor: Option<Record>,
    ) -> Self
    {
        return Self {
            predecessor,
            successor,
        };
    }

    /// Returns the predecessor record, when one exists.
    #[inline]
    #[must_use]
    pub const fn predecessor(&self) -> Option<&Record>
    {
        return self.predecessor.as_ref();
    }

    /// Returns the successor record, when one exists.
    #[inline]
    #[must_use]
    pub const fn successor(&self) -> Option<&Record>
    {
        return self.successor.as_ref();
    }
}

/// Store-independent proof that a key is absent from a root.
#[expect(
    clippy::module_name_repetitions,
    reason = "NonMembershipProof is the explicit public proof vocabulary"
)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[cfg(feature = "proofs")]
pub struct NonMembershipProof
{
    /// Proof metadata and committed root context.
    envelope: ProofEnvelope,
    /// Root node hash named by the root manifest.
    root_node_hash: NodeHash,
    /// Queried absent key.
    key: Box<[u8]>,
    /// Adjacent-key evidence committed by the proof.
    evidence: NonMembershipEvidence,
    /// Encoded proof nodes; for one-level internal roots, root plus selected
    /// leaf and an optional required successor leaf.
    nodes: Box<[ProofNode]>,
}

#[cfg(feature = "proofs")]
impl NonMembershipProof
{
    /// Creates non-membership proof material.
    #[inline]
    #[must_use]
    pub fn new<K, N>(
        envelope: ProofEnvelope,
        root_node_hash: NodeHash,
        key: K,
        evidence: NonMembershipEvidence,
        nodes: N,
    ) -> Self
    where
        K: Into<Box<[u8]>>,
        N: Into<Box<[ProofNode]>>,
    {
        return Self {
            envelope,
            root_node_hash,
            key: key.into(),
            evidence,
            nodes: nodes.into(),
        };
    }

    /// Returns proof metadata.
    #[inline]
    #[must_use]
    pub const fn envelope(&self) -> &ProofEnvelope
    {
        return &self.envelope;
    }

    /// Returns the root node hash named by the root manifest.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.root_node_hash;
    }

    /// Returns the absent key carried by this proof.
    #[inline]
    #[must_use]
    pub fn key(&self) -> &[u8]
    {
        return self.key.as_ref();
    }

    /// Returns the adjacent-key evidence carried by this proof.
    #[inline]
    #[must_use]
    pub const fn evidence(&self) -> &NonMembershipEvidence
    {
        return &self.evidence;
    }

    /// Returns encoded proof nodes.
    ///
    /// `current`: one-level internal-root non-membership proofs carry the root
    /// node followed by the selected leaf node and, only when required, the
    /// adjacent successor leaf node.
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> &[ProofNode]
    {
        return self.nodes.as_ref();
    }

    /// Verifies absence for `expected_key`.
    ///
    /// # Errors
    ///
    /// Returns an error when root context, chunker parameters, node bytes,
    /// adjacent-key evidence, or queried key do not match authenticated data.
    #[inline]
    pub fn verify(
        &self,
        expected_root: &TreeRoot,
        expected_params: &TreeParams,
        expected_key: &[u8],
    ) -> Result<NonMembershipEvidence, ProllyBaoError>
    {
        let input = VerifierInput {
            envelope: &self.envelope,
            root_node_hash: self.root_node_hash,
            nodes: self.nodes.as_ref(),
            expected_root,
            expected_params,
        };

        return verify_non_membership_material(
            &input,
            self.key.as_ref(),
            &self.evidence,
            expected_key,
        );
    }
}

/// Store-independent proof for a complete ordered key range.
#[expect(
    clippy::module_name_repetitions,
    reason = "RangeProof is the explicit public proof vocabulary"
)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[cfg(feature = "proofs")]
pub struct RangeProof
{
    /// Proof metadata and committed root context.
    envelope: ProofEnvelope,
    /// Root node hash named by the root manifest.
    root_node_hash: NodeHash,
    /// Queried range bound by the proof.
    range: OwnedKeyRange,
    /// Returned records bound to `range`.
    records: Box<[Record]>,
    /// Encoded proof nodes; for one-level internal roots, root plus contiguous
    /// selected leaves.
    nodes: Box<[ProofNode]>,
}

#[cfg(feature = "proofs")]
impl RangeProof
{
    /// Creates range proof material.
    #[inline]
    #[must_use]
    pub fn new<N, R>(
        envelope: ProofEnvelope,
        root_node_hash: NodeHash,
        range: OwnedKeyRange,
        records: R,
        nodes: N,
    ) -> Self
    where
        N: Into<Box<[ProofNode]>>,
        R: Into<Box<[Record]>>,
    {
        return Self {
            envelope,
            root_node_hash,
            range,
            records: records.into(),
            nodes: nodes.into(),
        };
    }

    /// Returns proof metadata.
    #[inline]
    #[must_use]
    pub const fn envelope(&self) -> &ProofEnvelope
    {
        return &self.envelope;
    }

    /// Returns the root node hash named by the root manifest.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.root_node_hash;
    }

    /// Returns the queried range stored in this proof.
    #[inline]
    #[must_use]
    pub const fn range(&self) -> &OwnedKeyRange
    {
        return &self.range;
    }

    /// Returns records carried by this proof.
    #[inline]
    #[must_use]
    pub fn records(&self) -> &[Record]
    {
        return self.records.as_ref();
    }

    /// Returns encoded proof nodes.
    ///
    /// `current`: one-level internal-root range proofs carry the root node
    /// followed by contiguous selected leaf nodes in root separator order.
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> &[ProofNode]
    {
        return self.nodes.as_ref();
    }

    /// Verifies this proof for the range stored inside the proof.
    ///
    /// # Errors
    ///
    /// Returns an error when root context, chunker parameters, node bytes,
    /// range completeness, or returned records do not match authenticated
    /// data.
    #[inline]
    pub fn verify(
        &self,
        expected_root: &TreeRoot,
        expected_params: &TreeParams,
    ) -> Result<Box<[Record]>, ProllyBaoError>
    {
        return self.verify_for_range(expected_root, expected_params, self.range.as_range_ref()?);
    }

    /// Verifies this proof for an explicit borrowed range.
    ///
    /// # Errors
    ///
    /// Returns an error when `expected_range` does not match the stored proof
    /// range or when authenticated records are incomplete for the range.
    #[inline]
    pub fn verify_for_range(
        &self,
        expected_root: &TreeRoot,
        expected_params: &TreeParams,
        expected_range: KeyRangeRef<'_>,
    ) -> Result<Box<[Record]>, ProllyBaoError>
    {
        let input = VerifierInput {
            envelope: &self.envelope,
            root_node_hash: self.root_node_hash,
            nodes: self.nodes.as_ref(),
            expected_root,
            expected_params,
        };

        return verify_range_material(&input, &self.range, self.records.as_ref(), expected_range);
    }
}

/// Proof-oriented deterministic tree used by the first portable proof slice.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PortableProofTree
{
    /// Content-addressed root manifest.
    root: TreeRoot,
    /// Root node hash named by the root manifest.
    root_node_hash: NodeHash,
    /// Encoded nodes in deterministic root-first order.
    nodes: Box<[ProofNode]>,
    /// Leaf hashes in canonical order for structural-sharing evidence.
    leaf_hashes: Box<[NodeHash]>,
    /// Owned records represented by this tree.
    records: Box<[Record]>,
}

impl PortableProofTree
{
    /// Builds a proof-oriented tree from strictly sorted borrowed records.
    ///
    /// # Errors
    ///
    /// Returns an error when input records are unsorted, duplicate keyed,
    /// chunker parameters reject record-safe spans, or encoded node/root
    /// material cannot be represented.
    #[inline]
    pub fn build(
        records: &[RecordRef<'_>],
        params: TreeParams,
    ) -> Result<Self, ProllyBaoError>
    {
        ensure_supported_params(&params)?;
        validate_sorted_records(records)?;

        let owned_records = own_records(records);
        let canonical_records = canonical_record_bytes(records)?;
        let record_slices = borrowed_slices(canonical_records.as_ref());
        let chunk_spans = chunk_record_slices(
            CanonicalRecords::from(record_slices.as_ref()),
            params.chunker_params(),
        )?;

        let mut leaves = Vec::<LeafBuild>::new();

        if records.is_empty() {
            leaves.push(build_leaf(records)?);
        }
        else {
            for span in chunk_spans {
                let start = usize_from_u64(
                    u64::from(span.records.start),
                    "chunk start record index does not fit usize",
                )?;
                let end = usize_from_u64(
                    u64::from(span.records.end),
                    "chunk end record index does not fit usize",
                )?;
                let chunk_records =
                    records
                        .get(start .. end)
                        .ok_or(ProllyBaoError::InvalidProofShape {
                            context: "chunk span is outside input records",
                        })?;
                leaves.push(build_leaf(chunk_records)?);
            }
        }

        if leaves.is_empty() {
            leaves.push(build_leaf(records)?);
        }

        let root_build = build_root_node(leaves.as_slice())?;
        let record_count = u64_from_usize(records.len(), "record count does not fit u64")?;
        let root_hash = hash_root_manifest(&params, record_count, root_build.hash)?;
        let root = TreeRoot::new(root_hash, params, record_count);

        let nodes = collect_nodes(root_build, leaves.as_slice());
        let leaf_hashes = collect_leaf_hashes(leaves.as_slice());

        return Ok(Self {
            root,
            root_node_hash: nodes
                .first()
                .ok_or(ProllyBaoError::InvalidProofShape {
                    context: "tree has no root node",
                })?
                .hash(),
            nodes,
            leaf_hashes,
            records: owned_records,
        });
    }

    /// Returns this tree's root manifest.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> &TreeRoot
    {
        return &self.root;
    }

    /// Returns the root node hash named by the root manifest.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.root_node_hash;
    }

    /// Returns encoded nodes in root-first deterministic order.
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> &[ProofNode]
    {
        return self.nodes.as_ref();
    }

    /// Returns canonical leaf hashes in order.
    #[inline]
    #[must_use]
    pub fn leaf_hashes(&self) -> &[NodeHash]
    {
        return self.leaf_hashes.as_ref();
    }

    /// Appends canonical Prolly-Bao snapshot bytes to `out`.
    ///
    /// `current`: these bytes materialize the complete ordered-record snapshot
    /// under this tree's [`TreeRoot`] and [`TreeParams`]. They are
    /// deterministic Prolly-Bao snapshot bytes, not Bao proofs; Bao
    /// verification belongs in a separate test/dev adapter over the emitted
    /// byte stream.
    ///
    /// Existing contents of `out` are preserved and the snapshot is appended.
    /// The version-1 format is:
    ///
    /// 1. `prolly-bao:snapshot:v1` domain bytes;
    /// 2. two-byte big-endian snapshot version `1`;
    /// 3. root hash, root record count, and length-prefixed chunker parameter
    ///    bytes;
    /// 4. root-node hash;
    /// 5. exact record count;
    /// 6. exact key/value length-prefixed records in decoded key order.
    ///
    /// Empty trees, empty keys, empty values, and arbitrary high-byte key/value
    /// bytes are represented by the same length-prefixed record framing.
    ///
    /// # Errors
    ///
    /// Returns an error when a length or count cannot be represented in the
    /// deterministic binary format or when this tree's stored records do not
    /// match its root record count.
    #[inline]
    pub fn encode_snapshot_bytes(
        &self,
        out: &mut Vec<u8>,
    ) -> Result<(), ProllyBaoError>
    {
        out.reserve(self.snapshot_encoded_len()?);
        self.encode_snapshot_bytes_unreserved(out)?;

        return Ok(());
    }

    /// Encodes this tree into owned canonical Prolly-Bao snapshot bytes.
    ///
    /// `current`: the returned bytes are deterministic snapshot material for
    /// adapter evidence. They do not turn native Prolly-Bao witnesses into Bao
    /// proofs.
    ///
    /// # Errors
    ///
    /// Returns an error when a length or count cannot be represented in the
    /// deterministic binary format or when this tree's stored records do not
    /// match its root record count.
    #[inline]
    pub fn to_snapshot_bytes(&self) -> Result<Box<[u8]>, ProllyBaoError>
    {
        let capacity = self.snapshot_encoded_len()?;
        let mut bytes = Vec::<u8>::with_capacity(capacity);
        self.encode_snapshot_bytes_unreserved(&mut bytes)?;

        return Ok(bytes.into_boxed_slice());
    }

    /// Computes exact deterministic snapshot encoded length.
    fn snapshot_encoded_len(&self) -> Result<usize, ProllyBaoError>
    {
        let _record_count = self.snapshot_record_count()?;
        let mut len = SNAPSHOT_MAGIC.len();
        checked_add_to_len(&mut len, 2_usize, "snapshot encoded length overflow")?;
        checked_add_to_len(&mut len, NODE_HASH_LEN, "snapshot encoded length overflow")?;
        checked_add_to_len(&mut len, 8_usize, "snapshot encoded length overflow")?;
        checked_add_to_len(
            &mut len,
            snapshot_len_prefixed_bytes_encoded_len(self.root.params().chunker_parameter_bytes())?,
            "snapshot encoded length overflow",
        )?;
        checked_add_to_len(&mut len, NODE_HASH_LEN, "snapshot encoded length overflow")?;
        checked_add_to_len(&mut len, 8_usize, "snapshot encoded length overflow")?;

        for record in self.records.as_ref() {
            checked_add_to_len(
                &mut len,
                snapshot_record_encoded_len(record)?,
                "snapshot encoded length overflow",
            )?;
        }

        return Ok(len);
    }

    /// Encodes snapshot bytes without reserving capacity.
    fn encode_snapshot_bytes_unreserved(
        &self,
        out: &mut Vec<u8>,
    ) -> Result<(), ProllyBaoError>
    {
        let record_count = self.snapshot_record_count()?;

        out.extend_from_slice(SNAPSHOT_MAGIC);
        push_u16(out, SNAPSHOT_VERSION);
        out.extend_from_slice(self.root.hash().as_slice());
        push_u64(out, self.root.record_count());
        push_snapshot_len_prefixed_bytes(out, self.root.params().chunker_parameter_bytes())?;
        out.extend_from_slice(self.root_node_hash.as_slice());
        push_u64(out, record_count);

        for record in self.records.as_ref() {
            push_snapshot_len_prefixed_bytes(out, record.key())?;
            push_snapshot_len_prefixed_bytes(out, record.value())?;
        }

        return Ok(());
    }

    /// Returns the exact record count represented by stored snapshot records.
    fn snapshot_record_count(&self) -> Result<u64, ProllyBaoError>
    {
        let record_count =
            u64_from_usize(self.records.len(), "snapshot record count does not fit u64")?;

        if record_count != self.root.record_count() {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "snapshot records do not match root record count",
            });
        }

        return Ok(record_count);
    }

    /// Looks up a key in this deterministic proof tree.
    #[inline]
    #[must_use]
    pub fn lookup(
        &self,
        key: &[u8],
    ) -> Option<&[u8]>
    {
        match find_record(self.records.as_ref(), key) {
            | Some(record) => return Some(record.value()),
            | None => return None,
        }
    }

    /// Returns owned records inside `range`.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::RangeBound`] when the borrowed range is
    /// invalid.
    #[inline]
    pub fn range(
        &self,
        range: KeyRangeRef<'_>,
    ) -> Result<Box<[Record]>, ProllyBaoError>
    {
        let checked_range = KeyRangeRef::new(range.start(), range.end())?;

        return Ok(select_range_records(self.records.as_ref(), checked_range));
    }

    /// Builds a membership proof for `key`.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::InvalidProofShape`] when `key` is absent.
    #[inline]
    #[cfg(feature = "proofs")]
    pub fn prove_membership(
        &self,
        key: &[u8],
    ) -> Result<MembershipProof, ProllyBaoError>
    {
        let record =
            find_record(self.records.as_ref(), key).ok_or(ProllyBaoError::InvalidProofShape {
                context: "membership key is absent",
            })?;
        let nodes = self.path_nodes_for_key(key)?;

        return Ok(MembershipProof::new(
            ProofEnvelope::new(self.root.clone(), ProofKind::Membership),
            self.root_node_hash,
            record.key(),
            record.value(),
            nodes,
        ));
    }

    /// Builds a non-membership proof for `key`.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::InvalidProofShape`] when `key` is present.
    #[inline]
    #[cfg(feature = "proofs")]
    pub fn prove_non_membership(
        &self,
        key: &[u8],
    ) -> Result<NonMembershipProof, ProllyBaoError>
    {
        if find_record(self.records.as_ref(), key).is_some() {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "non-membership key is present",
            });
        }

        let nodes = self.non_membership_nodes_for_key(key)?;

        return Ok(NonMembershipProof::new(
            ProofEnvelope::new(self.root.clone(), ProofKind::NonMembership),
            self.root_node_hash,
            key,
            adjacent_evidence(self.records.as_ref(), key),
            nodes,
        ));
    }

    /// Builds a range proof for `range`.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::RangeBound`] when `range` is invalid.
    #[inline]
    #[cfg(feature = "proofs")]
    pub fn prove_range(
        &self,
        range: KeyRangeRef<'_>,
    ) -> Result<RangeProof, ProllyBaoError>
    {
        let checked_range = KeyRangeRef::new(range.start(), range.end())?;
        let records = select_range_records(self.records.as_ref(), checked_range);

        let nodes = self.range_nodes_for_query(checked_range)?;

        return Ok(RangeProof::new(
            ProofEnvelope::new(self.root.clone(), ProofKind::Range),
            self.root_node_hash,
            OwnedKeyRange::from_ref(checked_range),
            records,
            nodes,
        ));
    }

    /// Returns root/leaf path nodes for `key`.
    #[cfg(feature = "proofs")]
    fn path_nodes_for_key(
        &self,
        key: &[u8],
    ) -> Result<Box<[ProofNode]>, ProllyBaoError>
    {
        let root_node = find_proof_node(self.nodes.as_ref(), self.root_node_hash).ok_or(
            ProllyBaoError::InvalidProofShape {
                context: "root node hash is absent from tree nodes",
            },
        )?;
        let root = decode_encoded_node(root_node.clone())?;

        match root.kind {
            | DecodedNodeKind::Leaf(_) => {
                let nodes = vec![root.proof_node];
                return Ok(nodes.into_boxed_slice());
            },
            | DecodedNodeKind::Internal(ref internal) => {
                let selected_child = select_child_for_key(internal.children.as_ref(), key).ok_or(
                    ProllyBaoError::InvalidProofShape {
                        context: "internal root has no child for key",
                    },
                )?;
                let selected_node = find_proof_node(self.nodes.as_ref(), selected_child.hash)
                    .ok_or(ProllyBaoError::InvalidProofShape {
                        context: "selected child node is absent from tree nodes",
                    })?;
                let selected_leaf_node = decode_encoded_node(selected_node.clone())?;
                let selected_leaf = match selected_leaf_node.kind {
                    | DecodedNodeKind::Leaf(ref leaf) => leaf,
                    | DecodedNodeKind::Internal(_) => {
                        return Err(ProllyBaoError::InvalidProofShape {
                            context: "selected child is not a leaf",
                        });
                    },
                };
                verify_child_leaf(selected_child, selected_leaf)?;
                let nodes = vec![
                    root.proof_node.clone(),
                    selected_leaf_node.proof_node.clone(),
                ];
                return Ok(nodes.into_boxed_slice());
            },
        }
    }

    /// Returns compact root/leaf nodes for a non-membership proof.
    #[cfg(feature = "proofs")]
    fn non_membership_nodes_for_key(
        &self,
        key: &[u8],
    ) -> Result<Box<[ProofNode]>, ProllyBaoError>
    {
        let root_node = find_proof_node(self.nodes.as_ref(), self.root_node_hash).ok_or(
            ProllyBaoError::InvalidProofShape {
                context: "root node hash is absent from tree nodes",
            },
        )?;
        let root = decode_encoded_node(root_node.clone())?;

        match root.kind {
            | DecodedNodeKind::Leaf(_) => {
                let nodes = vec![root.proof_node];
                return Ok(nodes.into_boxed_slice());
            },
            | DecodedNodeKind::Internal(ref internal) => {
                let selected_index = select_child_index_for_key(internal.children.as_ref(), key)
                    .ok_or(ProllyBaoError::InvalidProofShape {
                        context: "internal root has no child for key",
                    })?;
                let selected_child = internal.children.get(selected_index).ok_or(
                    ProllyBaoError::InvalidProofShape {
                        context: "selected child index is out of bounds",
                    },
                )?;
                let selected_node = find_proof_node(self.nodes.as_ref(), selected_child.hash)
                    .ok_or(ProllyBaoError::InvalidProofShape {
                        context: "selected child node is absent from tree nodes",
                    })?;
                let (selected_start, selected_end) =
                    child_record_span(internal.children.as_ref(), selected_index)?;
                let selected_records = self.records.get(selected_start .. selected_end).ok_or(
                    ProllyBaoError::InvalidProofShape {
                        context: "selected child record span is outside tree records",
                    },
                )?;
                let mut nodes = Vec::<ProofNode>::with_capacity(0x03_usize);
                nodes.push(root.proof_node.clone());
                nodes.push(selected_node.clone());

                if compact_non_membership_needs_next_records(selected_records, key)?
                    && let Some(next_child) = selected_index
                        .checked_add(1_usize)
                        .and_then(|next_index| internal.children.get(next_index))
                {
                    let next_node = find_proof_node(self.nodes.as_ref(), next_child.hash).ok_or(
                        ProllyBaoError::InvalidProofShape {
                            context: "successor child node is absent from tree nodes",
                        },
                    )?;
                    nodes.push(next_node.clone());
                }

                return Ok(nodes.into_boxed_slice());
            },
        }
    }

    /// Returns compact root/leaf nodes for a current one-level range proof.
    #[cfg(feature = "proofs")]
    fn range_nodes_for_query(
        &self,
        range: KeyRangeRef<'_>,
    ) -> Result<Box<[ProofNode]>, ProllyBaoError>
    {
        let root_node = find_proof_node(self.nodes.as_ref(), self.root_node_hash).ok_or(
            ProllyBaoError::InvalidProofShape {
                context: "root node hash is absent from tree nodes",
            },
        )?;
        let root = decode_encoded_node(root_node.clone())?;

        match root.kind {
            | DecodedNodeKind::Leaf(_) => {
                let nodes = vec![root.proof_node];
                return Ok(nodes.into_boxed_slice());
            },
            | DecodedNodeKind::Internal(ref internal) => {
                let start_index = start_child_index_for_range(internal.children.as_ref(), range)
                    .ok_or(ProllyBaoError::InvalidProofShape {
                        context: "range proof start child is absent",
                    })?;
                let end_index = end_child_index_for_range(internal.children.as_ref(), range)
                    .ok_or(ProllyBaoError::InvalidProofShape {
                        context: "range proof end child is absent",
                    })?;
                if start_index > end_index {
                    return Err(ProllyBaoError::InvalidProofShape {
                        context: "range proof child span is inverted",
                    });
                }

                let mut nodes = Vec::<ProofNode>::with_capacity(
                    end_index
                        .saturating_sub(start_index)
                        .saturating_add(0x02_usize),
                );
                nodes.push(root.proof_node.clone());

                let selected_children = internal.children.get(start_index ..= end_index).ok_or(
                    ProllyBaoError::InvalidProofShape {
                        context: "range proof child span is outside root children",
                    },
                )?;
                for child in selected_children {
                    let child_node = find_proof_node(self.nodes.as_ref(), child.hash).ok_or(
                        ProllyBaoError::InvalidProofShape {
                            context: "range proof child node is absent from tree nodes",
                        },
                    )?;
                    nodes.push(child_node.clone());
                }

                return Ok(nodes.into_boxed_slice());
            },
        }
    }
}

/// Verifies canonical Prolly-Bao snapshot bytes against explicit root context.
///
/// `current`: this parser verifies deterministic Prolly-Bao snapshot bytes and
/// rebuilds the represented ordered-record tree under `expected_params`. It is
/// not a Bao verifier and does not make native Prolly-Bao witnesses into Bao
/// proofs; adapters may feed the same canonical bytes to Bao separately as
/// byte-stream evidence.
///
/// The accepted version-1 stream carries the Prolly-Bao snapshot domain,
/// version, root hash, root record count, chunker parameter bytes, root-node
/// hash, exact record count, and exact key/value length-prefixed records in
/// decoded key order.
///
/// # Errors
///
/// Returns [`ProllyBaoError::MalformedSnapshotBytes`] for malformed framing,
/// truncation, count mismatches, magic mismatches, or trailing bytes;
/// [`ProllyBaoError::UnsupportedSnapshotVersion`] for unsupported snapshot
/// versions; [`ProllyBaoError::UnsortedInput`] /
/// [`ProllyBaoError::DuplicateKeys`] for non-canonical record order; and root,
/// parameter, or hash errors when the snapshot bytes do not rebuild to
/// `expected_root` under `expected_params`.
#[inline]
pub fn verify_snapshot_bytes(
    bytes: &[u8],
    expected_root: &TreeRoot,
    expected_params: &TreeParams,
) -> Result<PortableProofTree, ProllyBaoError>
{
    let mut cursor = SnapshotCursor::new(bytes);
    let magic = cursor.take(SNAPSHOT_MAGIC.len(), "snapshot magic is truncated")?;

    if magic != SNAPSHOT_MAGIC {
        return Err(ProllyBaoError::MalformedSnapshotBytes {
            context: "snapshot magic mismatch",
        });
    }

    let version = cursor.read_u16()?;

    if version != SNAPSHOT_VERSION {
        return Err(ProllyBaoError::UnsupportedSnapshotVersion { version });
    }

    let root_hash = NodeHash::from_bytes(
        cursor.take_array::<NODE_HASH_LEN>("snapshot root hash is truncated")?,
    );
    let root_record_count = cursor.read_u64()?;
    let chunker_parameter_bytes = decode_snapshot_bytes(
        &mut cursor,
        "snapshot chunker parameter bytes are truncated",
    )?;
    let root_node_hash = NodeHash::from_bytes(
        cursor.take_array::<NODE_HASH_LEN>("snapshot root node hash is truncated")?,
    );
    let record_count = cursor.read_u64()?;

    if record_count != root_record_count {
        return Err(ProllyBaoError::MalformedSnapshotBytes {
            context: "snapshot record count does not match root record count",
        });
    }

    verify_snapshot_binding(
        root_hash,
        root_record_count,
        chunker_parameter_bytes,
        expected_root,
        expected_params,
    )?;

    let records = decode_snapshot_records(&mut cursor, record_count)?;

    if !cursor.is_empty() {
        return Err(ProllyBaoError::MalformedSnapshotBytes {
            context: "trailing snapshot bytes",
        });
    }

    validate_sorted_records(records.as_slice())?;

    let parsed_record_count = u64_from_usize(
        records.len(),
        "snapshot parsed record count does not fit u64",
    )?;

    if parsed_record_count != record_count {
        return Err(ProllyBaoError::MalformedSnapshotBytes {
            context: "snapshot parsed record count mismatch",
        });
    }

    let tree = PortableProofTree::build(records.as_slice(), expected_params.clone())?;

    if tree.root_node_hash() != root_node_hash {
        return Err(ProllyBaoError::HashMismatch {
            expected: root_node_hash,
            actual: tree.root_node_hash(),
        });
    }

    if tree.root().hash() != expected_root.hash() {
        return Err(ProllyBaoError::HashMismatch {
            expected: expected_root.hash(),
            actual: tree.root().hash(),
        });
    }

    if tree.root().record_count() != expected_root.record_count() {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "snapshot rebuilt record count does not match verifier root",
        });
    }

    if tree.root().params() != expected_params {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "snapshot rebuilt parameters do not match verifier parameters",
        });
    }

    return Ok(tree);
}
/// Returns the BLAKE3 identity for encoded node bytes.
#[inline]
#[must_use]
pub fn hash_encoded_node(bytes: &[u8]) -> NodeHash
{
    return NodeHash::from_bytes(*blake3::hash(bytes).as_bytes());
}

/// Inspects canonical encoded Prolly-Bao node bytes without changing identity.
///
/// `current`: this parser borrows the input, validates the node framing, and
/// reports only layout metadata needed for node-layout review. It does not
/// allocate decoded records and leaves `BLAKE3(encoded_node_bytes)` as the node
/// identity.
///
/// # Errors
///
/// Returns [`ProllyBaoError::MalformedNodeBytes`] for malformed framing,
/// unknown node kinds, truncated sections, invalid ordering, or trailing bytes;
/// [`ProllyBaoError::UnsupportedEncodingVersion`] for unsupported encoding
/// versions; and [`ProllyBaoError::DuplicateKeys`] for duplicate leaf keys.
#[inline]
pub fn inspect_encoded_node(bytes: &[u8]) -> Result<EncodedNodeLayout, ProllyBaoError>
{
    let encoded_len = bytes.len();
    let mut cursor = Cursor::new(bytes);
    let kind = read_node_header(&mut cursor)?;
    let layout = match kind {
        | NODE_KIND_LEAF => {
            let record_count = inspect_leaf_payload(&mut cursor)?;
            EncodedNodeLayout::leaf(encoded_len, record_count)
        },
        | NODE_KIND_INTERNAL => {
            let child_count = inspect_internal_payload(&mut cursor)?;
            EncodedNodeLayout::internal(encoded_len, child_count)
        },
        | _ => {
            return Err(ProllyBaoError::MalformedNodeBytes {
                context: "unknown node kind",
            });
        },
    };

    if !cursor.is_empty() {
        return Err(ProllyBaoError::MalformedNodeBytes {
            context: "trailing node bytes",
        });
    }

    return Ok(layout);
}

/// Verifies a borrowed stored node by checking its hash and decoding its bytes.
///
/// # Errors
///
/// Returns [`ProllyBaoError::HashMismatch`] when bytes do not hash to
/// `node.node_hash()`, or [`ProllyBaoError::MalformedNodeBytes`] /
/// [`ProllyBaoError::UnsupportedEncodingVersion`] when the bytes do not decode.
#[inline]
pub fn verify_stored_node(node: StoredNodeRef<'_>) -> Result<(), ProllyBaoError>
{
    let actual = hash_encoded_node(node.bytes());
    let expected = node.node_hash();

    if actual != expected {
        return Err(ProllyBaoError::HashMismatch { expected, actual });
    }

    let _decoded = decode_encoded_node(ProofNode::new(expected, node.bytes()))?;

    return Ok(());
}

/// Leaf node assembled during tree construction.
#[derive(Clone, Debug, Eq, PartialEq)]
struct LeafBuild
{
    /// Leaf hash.
    hash: NodeHash,
    /// Encoded leaf bytes.
    bytes: Box<[u8]>,
    /// First key in this leaf, if any.
    first_key: Option<Box<[u8]>>,
    /// Number of records in this leaf.
    record_count: u64,
}

/// Root node assembled during tree construction.
#[derive(Clone, Debug, Eq, PartialEq)]
struct RootBuild
{
    /// Root node hash.
    hash: NodeHash,
    /// Encoded root node bytes.
    bytes: Box<[u8]>,
}

/// Decoded proof tree used by full-reachability verifiers.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(feature = "proofs")]
struct VerifiedTree
{
    /// Authenticated ordered records.
    records: Box<[Record]>,
}

/// Fully decoded proof node.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodedNode
{
    /// Re-usable proof node.
    proof_node: ProofNode,
    /// Decoded node kind and payload.
    kind: DecodedNodeKind,
}

impl DecodedNode
{
    /// Returns decoded node hash.
    #[inline]
    #[must_use]
    #[cfg(feature = "proofs")]
    const fn hash(&self) -> NodeHash
    {
        return self.proof_node.hash();
    }
}

/// Decoded node payload.
#[derive(Clone, Debug, Eq, PartialEq)]
enum DecodedNodeKind
{
    /// Leaf records.
    Leaf(DecodedLeaf),
    /// Internal child references.
    Internal(DecodedInternal),
}

/// Decoded leaf records.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodedLeaf
{
    /// Strictly sorted records.
    records: Box<[Record]>,
}

impl DecodedLeaf
{
    /// Returns the first key in this leaf, when present.
    #[inline]
    #[must_use]
    #[cfg(feature = "proofs")]
    fn first_key(&self) -> Option<&[u8]>
    {
        match self.records.first() {
            | Some(record) => return Some(record.key()),
            | None => return None,
        }
    }

    /// Returns this leaf record count.
    #[inline]
    #[cfg(feature = "proofs")]
    fn record_count(&self) -> Result<u64, ProllyBaoError>
    {
        return u64_from_usize(self.records.len(), "leaf record count does not fit u64");
    }
}

/// Decoded internal node.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodedInternal
{
    /// Total records reachable under this node.
    record_count: u64,
    /// Child separators and hashes.
    children: Box<[DecodedChild]>,
}

/// Decoded child reference.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodedChild
{
    /// First key reachable from the child.
    first_key: Box<[u8]>,
    /// Child node hash.
    hash: NodeHash,
    /// Number of records reachable from the child.
    record_count: u64,
}

/// Borrowed byte decoder for canonical node bytes.
struct Cursor<'bytes>
{
    /// Remaining unread bytes.
    remaining: &'bytes [u8],
}

impl<'bytes> Cursor<'bytes>
{
    /// Creates a cursor over `bytes`.
    #[inline]
    #[must_use]
    const fn new(bytes: &'bytes [u8]) -> Self
    {
        return Self { remaining: bytes };
    }

    /// Takes exactly `len` bytes.
    const fn take(
        &mut self,
        len: usize,
        context: &'static str,
    ) -> Result<&'bytes [u8], ProllyBaoError>
    {
        if self.remaining.len() < len {
            return Err(ProllyBaoError::MalformedNodeBytes { context });
        }

        let (head, tail) = self.remaining.split_at(len);
        self.remaining = tail;

        return Ok(head);
    }

    /// Reads a fixed-size byte array.
    fn take_array<const LEN: usize>(
        &mut self,
        context: &'static str,
    ) -> Result<[u8; LEN], ProllyBaoError>
    {
        let bytes = self.take(LEN, context)?;
        let mut array = [0_u8; LEN];
        array.copy_from_slice(bytes);

        return Ok(array);
    }

    /// Reads one byte.
    fn read_u8(&mut self) -> Result<u8, ProllyBaoError>
    {
        let byte = self.take_array::<1>("u8 field is truncated")?;

        return Ok(u8::from_be_bytes(byte));
    }

    /// Reads one big-endian `u16`.
    fn read_u16(&mut self) -> Result<u16, ProllyBaoError>
    {
        let bytes = self.take_array::<2>("u16 field is truncated")?;

        return Ok(u16::from_be_bytes(bytes));
    }

    /// Reads one big-endian `u64`.
    fn read_u64(&mut self) -> Result<u64, ProllyBaoError>
    {
        let bytes = self.take_array::<8>("u64 field is truncated")?;

        return Ok(u64::from_be_bytes(bytes));
    }

    /// Returns true when all bytes have been consumed.
    #[inline]
    #[must_use]
    const fn is_empty(&self) -> bool
    {
        return self.remaining.is_empty();
    }
}

/// Validates tree parameters currently supported by proof encoding.
fn ensure_supported_params(params: &TreeParams) -> Result<(), ProllyBaoError>
{
    if params.kind() != TreeKind::MerkleSearch {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "unsupported tree kind",
        });
    }

    if params.encoding_version() != EncodingVersion::CURRENT {
        return Err(ProllyBaoError::UnsupportedEncodingVersion {
            version: params.encoding_version().as_u16(),
        });
    }

    if params.hash_algorithm() != HashAlgorithm::CURRENT {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "unsupported hash algorithm",
        });
    }

    if params.separator_convention() != SeparatorConvention::CURRENT {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "unsupported separator convention",
        });
    }

    if params.chunker_parameter_bytes() != params.chunker_params().commitment_bytes().as_ref() {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "chunker commitment copy does not match chunker parameters",
        });
    }

    return Ok(());
}

/// Validates proof metadata against explicit verifier context.
#[cfg(feature = "proofs")]
fn verify_envelope(
    envelope: &ProofEnvelope,
    expected_root: &TreeRoot,
    expected_params: &TreeParams,
    expected_kind: ProofKind,
) -> Result<(), ProllyBaoError>
{
    ensure_supported_params(expected_params)?;

    if envelope.kind() != expected_kind {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "proof kind does not match verifier request",
        });
    }

    if envelope.root() != expected_root {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "proof root does not match verifier root",
        });
    }

    if expected_root.params() != expected_params {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "expected root parameters do not match verifier parameters",
        });
    }

    if envelope.encoding_version() != expected_params.encoding_version() {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "proof encoding version mismatch",
        });
    }

    if envelope.chunker_parameter_bytes() != expected_params.chunker_parameter_bytes() {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "proof chunker parameter commitment mismatch",
        });
    }

    return Ok(());
}

/// Shared verifier context for proof wrappers and witness delegation.
#[cfg(feature = "proofs")]
struct VerifierInput<'proof>
{
    /// Proof metadata and committed root context.
    envelope: &'proof ProofEnvelope,
    /// Root node hash named by the root manifest.
    root_node_hash: NodeHash,
    /// Encoded proof nodes.
    nodes: &'proof [ProofNode],
    /// Verifier's expected root.
    expected_root: &'proof TreeRoot,
    /// Verifier's expected tree parameters.
    expected_params: &'proof TreeParams,
}

/// Verifies membership proof material without requiring an owned proof wrapper.
#[cfg(feature = "proofs")]
fn verify_membership_material(
    input: &VerifierInput<'_>,
    key: &[u8],
    value: &[u8],
    expected_key: &[u8],
    expected_value: &[u8],
) -> Result<(), ProllyBaoError>
{
    verify_envelope(
        input.envelope,
        input.expected_root,
        input.expected_params,
        ProofKind::Membership,
    )?;

    if key != expected_key {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "membership key does not match verifier query",
        });
    }

    if value != expected_value {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "membership value does not match verifier expectation",
        });
    }

    verify_membership_path(
        input.expected_root,
        input.root_node_hash,
        key,
        value,
        input.nodes,
    )?;

    return Ok(());
}

/// Verifies non-membership proof material without requiring an owned proof
/// wrapper.
#[cfg(feature = "proofs")]
fn verify_non_membership_material(
    input: &VerifierInput<'_>,
    key: &[u8],
    evidence: &NonMembershipEvidence,
    expected_key: &[u8],
) -> Result<NonMembershipEvidence, ProllyBaoError>
{
    verify_envelope(
        input.envelope,
        input.expected_root,
        input.expected_params,
        ProofKind::NonMembership,
    )?;

    if key != expected_key {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "non-membership key does not match verifier query",
        });
    }

    let expected_evidence =
        verify_non_membership_nodes(input.expected_root, input.root_node_hash, input.nodes, key)?;

    if evidence != &expected_evidence {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "non-membership adjacent evidence mismatch",
        });
    }

    return Ok(expected_evidence);
}

/// Verifies compact or full-tree non-membership nodes and derives adjacent
/// evidence.
#[cfg(feature = "proofs")]
fn verify_non_membership_nodes(
    expected_root: &TreeRoot,
    root_node_hash: NodeHash,
    nodes: &[ProofNode],
    key: &[u8],
) -> Result<NonMembershipEvidence, ProllyBaoError>
{
    let decoded = decode_proof_nodes(nodes)?;
    verify_root_manifest(expected_root, root_node_hash)?;
    let root = find_decoded_node(decoded.as_slice(), root_node_hash).ok_or(
        ProllyBaoError::InvalidProofShape {
            context: "non-membership proof root node is absent",
        },
    )?;

    match root.kind {
        | DecodedNodeKind::Leaf(ref leaf) => {
            if decoded.len() != 1_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "leaf non-membership proof carries extra nodes",
                });
            }
            verify_leaf_record_count(leaf, expected_root.record_count())?;
            return compact_non_membership_evidence(leaf, None, key);
        },
        | DecodedNodeKind::Internal(ref internal) => {
            let expected_full_len = checked_add_usize(
                internal.children.len(),
                1_usize,
                "non-membership proof node count overflow",
            )?;
            if decoded.len() == expected_full_len {
                let verified_tree =
                    verify_full_internal_tree(expected_root, internal, decoded.as_slice())?;
                if find_record(verified_tree.records.as_ref(), key).is_some() {
                    return Err(ProllyBaoError::InvalidProofShape {
                        context: "non-membership proof key is present",
                    });
                }
                return Ok(adjacent_evidence(verified_tree.records.as_ref(), key));
            }

            return verify_compact_non_membership_internal(
                expected_root,
                internal,
                decoded.as_slice(),
                key,
            );
        },
    }
}

/// Verifies the compact non-membership proof shape for the current one-level
/// internal root.
#[cfg(feature = "proofs")]
fn verify_compact_non_membership_internal(
    expected_root: &TreeRoot,
    internal: &DecodedInternal,
    decoded: &[DecodedNode],
    key: &[u8],
) -> Result<NonMembershipEvidence, ProllyBaoError>
{
    if internal.record_count != expected_root.record_count() {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "non-membership internal record count mismatch",
        });
    }

    if decoded.len() < 2_usize || decoded.len() > 3_usize {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "compact non-membership proof must carry root, selected leaf, and optional successor leaf",
        });
    }

    let selected_index = select_child_index_for_key(internal.children.as_ref(), key).ok_or(
        ProllyBaoError::InvalidProofShape {
            context: "non-membership key has no selected child",
        },
    )?;
    let selected_child =
        internal
            .children
            .get(selected_index)
            .ok_or(ProllyBaoError::InvalidProofShape {
                context: "selected child index is out of bounds",
            })?;
    let selected_node = decoded
        .get(1_usize)
        .ok_or(ProllyBaoError::InvalidProofShape {
            context: "selected child node is absent",
        })?;

    if selected_node.hash() != selected_child.hash {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "selected child node is not in compact selected-child position",
        });
    }

    let selected_leaf = match selected_node.kind {
        | DecodedNodeKind::Leaf(ref leaf) => leaf,
        | DecodedNodeKind::Internal(_) => {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "selected child is not a leaf",
            });
        },
    };
    verify_child_leaf(selected_child, selected_leaf)?;

    let needs_next_leaf = compact_non_membership_needs_next_leaf(selected_leaf, key)?;
    let next_child = selected_index
        .checked_add(1_usize)
        .and_then(|next_index| internal.children.get(next_index));
    if needs_next_leaf && let Some(next_child) = next_child {
        let next_node = decoded
            .get(2_usize)
            .ok_or(ProllyBaoError::InvalidProofShape {
                context: "compact non-membership proof successor node is absent",
            })?;

        if decoded.len() != 3_usize {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "compact non-membership proof carries an unexpected number of nodes",
            });
        }

        if next_node.hash() != next_child.hash {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "compact non-membership proof successor node is not in adjacent-child position",
            });
        }

        let next_leaf = match next_node.kind {
            | DecodedNodeKind::Leaf(ref leaf) => leaf,
            | DecodedNodeKind::Internal(_) => {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "compact non-membership successor is not a leaf",
                });
            },
        };
        verify_child_leaf(next_child, next_leaf)?;

        return compact_non_membership_evidence(selected_leaf, Some(next_leaf), key);
    }

    if decoded.len() != 2_usize {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "compact non-membership proof carries an unnecessary successor leaf",
        });
    }

    return compact_non_membership_evidence(selected_leaf, None, key);
}

/// Verifies range proof material without requiring an owned proof wrapper.
#[cfg(feature = "proofs")]
fn verify_range_material(
    input: &VerifierInput<'_>,
    range: &OwnedKeyRange,
    records: &[Record],
    expected_range: KeyRangeRef<'_>,
) -> Result<Box<[Record]>, ProllyBaoError>
{
    verify_envelope(
        input.envelope,
        input.expected_root,
        input.expected_params,
        ProofKind::Range,
    )?;

    let expected_owned_range = OwnedKeyRange::from_ref(expected_range);

    if range != &expected_owned_range {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "range proof bounds do not match verifier query",
        });
    }

    let expected_records = verify_compact_range_nodes(
        input.expected_root,
        input.root_node_hash,
        input.nodes,
        expected_range,
    )?;

    if records != expected_records.as_ref() {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "range proof records are incomplete or inconsistent",
        });
    }

    return Ok(expected_records);
}

/// Verifies compact or full-tree range nodes and derives the authenticated
/// records for `expected_range`.
#[cfg(feature = "proofs")]
fn verify_compact_range_nodes(
    expected_root: &TreeRoot,
    root_node_hash: NodeHash,
    nodes: &[ProofNode],
    expected_range: KeyRangeRef<'_>,
) -> Result<Box<[Record]>, ProllyBaoError>
{
    let decoded = decode_proof_nodes(nodes)?;
    verify_root_manifest(expected_root, root_node_hash)?;
    let root = find_decoded_node(decoded.as_slice(), root_node_hash).ok_or(
        ProllyBaoError::InvalidProofShape {
            context: "range proof root node is absent",
        },
    )?;

    match root.kind {
        | DecodedNodeKind::Leaf(ref leaf) => {
            if decoded.len() != 1_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "leaf range proof carries extra nodes",
                });
            }
            verify_leaf_record_count(leaf, expected_root.record_count())?;
            return Ok(select_range_records(leaf.records.as_ref(), expected_range));
        },
        | DecodedNodeKind::Internal(ref internal) => {
            if internal.record_count != expected_root.record_count() {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "range proof internal record count mismatch",
                });
            }

            let start_index =
                start_child_index_for_range(internal.children.as_ref(), expected_range).ok_or(
                    ProllyBaoError::InvalidProofShape {
                        context: "range proof start child is absent",
                    },
                )?;
            let end_index = end_child_index_for_range(internal.children.as_ref(), expected_range)
                .ok_or(ProllyBaoError::InvalidProofShape {
                context: "range proof end child is absent",
            })?;
            if start_index > end_index {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "range proof child span is inverted",
                });
            }

            let expected_len = end_index
                .saturating_sub(start_index)
                .saturating_add(0x02_usize);
            if decoded.len() != expected_len {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "range proof carries unreachable or missing compact children",
                });
            }

            let mut records = Vec::<Record>::new();
            let mut previous_key: Option<Box<[u8]>> = None;
            let compact_nodes =
                decoded
                    .get(1_usize ..)
                    .ok_or(ProllyBaoError::InvalidProofShape {
                        context: "range proof compact child nodes are absent",
                    })?;
            let selected_children = internal.children.get(start_index ..= end_index).ok_or(
                ProllyBaoError::InvalidProofShape {
                    context: "range proof child span is outside root children",
                },
            )?;

            for (node, child) in compact_nodes.iter().zip(selected_children.iter()) {
                if node.hash() != child.hash {
                    return Err(ProllyBaoError::InvalidProofShape {
                        context: "range proof child nodes are not in compact separator order",
                    });
                }

                let leaf = match node.kind {
                    | DecodedNodeKind::Leaf(ref leaf) => leaf,
                    | DecodedNodeKind::Internal(_) => {
                        return Err(ProllyBaoError::InvalidProofShape {
                            context: "range proof compact child is not a leaf",
                        });
                    },
                };
                verify_child_leaf(child, leaf)?;

                for record in leaf.records.as_ref() {
                    if let Some(previous) = previous_key.as_ref()
                        && previous.as_ref() >= record.key()
                    {
                        return Err(ProllyBaoError::InvalidProofShape {
                            context: "range proof compact records are not globally sorted",
                        });
                    }

                    previous_key = Some(Box::<[u8]>::from(record.key()));
                    records.push(record.clone());
                }
            }

            return Ok(select_range_records(records.as_slice(), expected_range));
        },
    }
}

/// Verifies deterministic root-first node order for a membership witness.
#[cfg(feature = "proofs")]
fn verify_membership_witness_node_order(
    root_node_hash: NodeHash,
    key: &[u8],
    nodes: &[ProofNode],
) -> Result<(), ProllyBaoError>
{
    let decoded = decode_proof_nodes(nodes)?;
    let root = decoded.first().ok_or(ProllyBaoError::InvalidProofShape {
        context: "witness carries no node bytes",
    })?;

    if root.hash() != root_node_hash {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "witness root node is not first",
        });
    }

    match root.kind {
        | DecodedNodeKind::Leaf(_) => {
            if nodes.len() != 1_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "leaf membership witness carries extra nodes",
                });
            }
        },
        | DecodedNodeKind::Internal(ref internal) => {
            if nodes.len() != 2_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "membership witness must carry root and selected leaf",
                });
            }

            let child = select_child_for_key(internal.children.as_ref(), key).ok_or(
                ProllyBaoError::InvalidProofShape {
                    context: "membership witness key has no selected child",
                },
            )?;
            let child_node = nodes
                .get(1_usize)
                .ok_or(ProllyBaoError::InvalidProofShape {
                    context: "membership witness selected child is absent",
                })?;

            if child_node.hash() != child.hash {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "membership witness child is not in selected-child position",
                });
            }
        },
    }

    return Ok(());
}

/// Verifies deterministic root-first node order for a compact non-membership
/// witness.
#[cfg(feature = "proofs")]
fn verify_non_membership_witness_node_order(
    root_node_hash: NodeHash,
    key: &[u8],
    nodes: &[ProofNode],
) -> Result<(), ProllyBaoError>
{
    let decoded = decode_proof_nodes(nodes)?;
    let root = decoded.first().ok_or(ProllyBaoError::InvalidProofShape {
        context: "witness carries no node bytes",
    })?;

    if root.hash() != root_node_hash {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "witness root node is not first",
        });
    }

    match root.kind {
        | DecodedNodeKind::Leaf(_) => {
            if nodes.len() != 1_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "leaf non-membership witness carries extra nodes",
                });
            }
        },
        | DecodedNodeKind::Internal(ref internal) => {
            if nodes.len() < 2_usize || nodes.len() > 3_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "compact non-membership witness must carry root, selected leaf, and optional successor leaf",
                });
            }

            let selected_index = select_child_index_for_key(internal.children.as_ref(), key)
                .ok_or(ProllyBaoError::InvalidProofShape {
                    context: "non-membership witness key has no selected child",
                })?;
            let selected_child =
                internal
                    .children
                    .get(selected_index)
                    .ok_or(ProllyBaoError::InvalidProofShape {
                        context: "non-membership witness selected child index is out of bounds",
                    })?;
            let selected_node = nodes
                .get(1_usize)
                .ok_or(ProllyBaoError::InvalidProofShape {
                    context: "non-membership witness selected child is absent",
                })?;

            if selected_node.hash() != selected_child.hash {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "non-membership witness child is not in selected-child position",
                });
            }

            let selected_leaf = match decoded
                .get(1_usize)
                .ok_or(ProllyBaoError::InvalidProofShape {
                    context: "non-membership witness decoded selected child is absent",
                })?
                .kind
            {
                | DecodedNodeKind::Leaf(ref leaf) => leaf,
                | DecodedNodeKind::Internal(_) => {
                    return Err(ProllyBaoError::InvalidProofShape {
                        context: "non-membership witness selected child is not a leaf",
                    });
                },
            };

            let needs_next_leaf = compact_non_membership_needs_next_leaf(selected_leaf, key)?;
            let next_child = selected_index
                .checked_add(1_usize)
                .and_then(|next_index| internal.children.get(next_index));
            if needs_next_leaf && let Some(next_child) = next_child {
                if nodes.len() != 3_usize {
                    return Err(ProllyBaoError::InvalidProofShape {
                        context: "non-membership witness successor leaf is absent",
                    });
                }

                let next_node = nodes
                    .get(2_usize)
                    .ok_or(ProllyBaoError::InvalidProofShape {
                        context: "non-membership witness successor child is absent",
                    })?;

                if next_node.hash() != next_child.hash {
                    return Err(ProllyBaoError::InvalidProofShape {
                        context: "non-membership witness successor child is not in adjacent-child position",
                    });
                }
            }
            else if nodes.len() != 2_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "non-membership witness carries an unnecessary successor leaf",
                });
            }
        },
    }

    return Ok(());
}

/// Verifies deterministic root-first node order for a compact range witness.
#[cfg(feature = "proofs")]
fn verify_range_witness_node_order(
    root_node_hash: NodeHash,
    expected_range: KeyRangeRef<'_>,
    nodes: &[ProofNode],
) -> Result<(), ProllyBaoError>
{
    let decoded = decode_proof_nodes(nodes)?;
    let root = decoded.first().ok_or(ProllyBaoError::InvalidProofShape {
        context: "witness carries no node bytes",
    })?;

    if root.hash() != root_node_hash {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "witness root node is not first",
        });
    }

    match root.kind {
        | DecodedNodeKind::Leaf(_) => {
            if nodes.len() != 1_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "leaf range witness carries extra nodes",
                });
            }
        },
        | DecodedNodeKind::Internal(ref internal) => {
            let start_index =
                start_child_index_for_range(internal.children.as_ref(), expected_range).ok_or(
                    ProllyBaoError::InvalidProofShape {
                        context: "range witness start child is absent",
                    },
                )?;
            let end_index = end_child_index_for_range(internal.children.as_ref(), expected_range)
                .ok_or(ProllyBaoError::InvalidProofShape {
                context: "range witness end child is absent",
            })?;
            if start_index > end_index {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "range witness child span is inverted",
                });
            }

            let expected_len = end_index
                .saturating_sub(start_index)
                .saturating_add(0x02_usize);
            if nodes.len() != expected_len {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "range witness node count does not match compact child span",
                });
            }

            let compact_nodes = nodes
                .get(1_usize ..)
                .ok_or(ProllyBaoError::InvalidProofShape {
                    context: "range witness compact child nodes are absent",
                })?;
            let selected_children = internal.children.get(start_index ..= end_index).ok_or(
                ProllyBaoError::InvalidProofShape {
                    context: "range witness child span is outside root children",
                },
            )?;

            for (node, child) in compact_nodes.iter().zip(selected_children.iter()) {
                if node.hash() != child.hash {
                    return Err(ProllyBaoError::InvalidProofShape {
                        context: "range witness child nodes are not in compact separator order",
                    });
                }
            }
        },
    }

    return Ok(());
}

/// Verifies deterministic root-first node order for a full-tree witness.
#[expect(
    dead_code,
    reason = "kept as a fallback witness validator for future full-tree-only experiments"
)]
#[cfg(feature = "proofs")]
fn verify_full_witness_node_order(
    root_node_hash: NodeHash,
    nodes: &[ProofNode],
) -> Result<(), ProllyBaoError>
{
    let decoded = decode_proof_nodes(nodes)?;
    let root = decoded.first().ok_or(ProllyBaoError::InvalidProofShape {
        context: "witness carries no node bytes",
    })?;

    if root.hash() != root_node_hash {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "witness root node is not first",
        });
    }

    match root.kind {
        | DecodedNodeKind::Leaf(_) => {
            if nodes.len() != 1_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "leaf witness carries extra nodes",
                });
            }
        },
        | DecodedNodeKind::Internal(ref internal) => {
            let expected_len = checked_add_usize(
                internal.children.len(),
                1_usize,
                "witness node count overflow",
            )?;

            if nodes.len() != expected_len {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "full witness node count does not match root children",
                });
            }

            let child_nodes = nodes
                .get(1_usize ..)
                .ok_or(ProllyBaoError::InvalidProofShape {
                    context: "full witness node count does not match root children",
                })?;

            for (node, child) in child_nodes.iter().zip(internal.children.as_ref()) {
                if node.hash() != child.hash {
                    return Err(ProllyBaoError::InvalidProofShape {
                        context: "full witness child nodes are not in separator order",
                    });
                }
            }
        },
    }

    return Ok(());
}

/// Validates proof envelope fields needed to form a native witness transcript.
#[cfg(feature = "proofs")]
fn ensure_proof_can_form_witness(
    envelope: &ProofEnvelope,
    root_node_hash: NodeHash,
    expected_kind: ProofKind,
) -> Result<(), ProllyBaoError>
{
    ensure_supported_params(envelope.root().params())?;

    if envelope.kind() != expected_kind {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "proof kind does not match witness constructor",
        });
    }

    if envelope.encoding_version() != envelope.root().params().encoding_version() {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "proof encoding version mismatch",
        });
    }

    if envelope.chunker_parameter_bytes() != envelope.root().params().chunker_parameter_bytes() {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "proof chunker parameter commitment mismatch",
        });
    }

    verify_root_manifest(envelope.root(), root_node_hash)?;

    return Ok(());
}

/// Returns root fields copied into a native witness transcript.
#[cfg(feature = "proofs")]
fn witness_root_parts(envelope: &ProofEnvelope) -> (NodeHash, u64, Box<[u8]>)
{
    return (
        envelope.root().hash(),
        envelope.root().record_count(),
        Box::<[u8]>::from(envelope.chunker_parameter_bytes()),
    );
}

/// Borrowed material needed to compute a witness end summary.
#[cfg(feature = "proofs")]
struct WitnessSummaryMaterial<'witness>
{
    /// Explicit native witness transcript version.
    version: u16,
    /// Transcript shape marker.
    kind: WitnessKind,
    /// Agreed Prolly-Bao root hash named by this transcript.
    root_hash: NodeHash,
    /// Number of records committed by the agreed root.
    root_record_count: u64,
    /// Chunker parameter bytes copied from the root context.
    chunker_parameter_bytes: &'witness [u8],
    /// Root node hash named by the root manifest.
    root_node_hash: NodeHash,
    /// Kind-specific query and returned material.
    body: &'witness WitnessBody,
    /// Encoded proof nodes carried by the transcript.
    nodes: &'witness [ProofNode],
}

/// Already-computed witness end-summary fields except the binding digest.
#[cfg(feature = "proofs")]
struct WitnessSummaryParts
{
    /// Explicit native witness transcript version.
    version: u16,
    /// Transcript shape marker.
    kind: WitnessKind,
    /// Agreed Prolly-Bao root hash named by this transcript.
    root_hash: NodeHash,
    /// Number of records committed by the agreed root.
    root_record_count: u64,
    /// Digest of chunker parameter bytes copied from the root context.
    chunker_parameter_digest: NodeHash,
    /// Root node hash named by the root manifest.
    root_node_hash: NodeHash,
    /// Digest of kind-specific query and returned material.
    body_digest: NodeHash,
    /// Number of encoded proof nodes committed by this summary.
    proof_node_count: u64,
    /// Digest of encoded proof-node hashes and byte strings.
    proof_nodes_digest: NodeHash,
}

/// Computes the required terminal summary for a native witness transcript.
#[cfg(feature = "proofs")]
fn compute_witness_end_summary(
    material: &WitnessSummaryMaterial<'_>
) -> Result<WitnessEndSummary, ProllyBaoError>
{
    let chunker_parameter_digest =
        hash_witness_chunker_parameters(material.chunker_parameter_bytes)?;
    let body_digest = hash_witness_body(material.body)?;
    let proof_node_count =
        u64_from_usize(material.nodes.len(), "witness node count does not fit u64")?;
    let proof_nodes_digest = hash_witness_nodes(proof_node_count, material.nodes)?;
    let parts = WitnessSummaryParts {
        version: material.version,
        kind: material.kind,
        root_hash: material.root_hash,
        root_record_count: material.root_record_count,
        chunker_parameter_digest,
        root_node_hash: material.root_node_hash,
        body_digest,
        proof_node_count,
        proof_nodes_digest,
    };
    let binding_digest = hash_witness_end_summary_binding(&parts);

    return Ok(WitnessEndSummary {
        version: parts.version,
        kind: parts.kind,
        root_hash: parts.root_hash,
        root_record_count: parts.root_record_count,
        chunker_parameter_digest: parts.chunker_parameter_digest,
        root_node_hash: parts.root_node_hash,
        body_digest: parts.body_digest,
        proof_node_count: parts.proof_node_count,
        proof_nodes_digest: parts.proof_nodes_digest,
        binding_digest,
    });
}

/// Returns the encoded length of the fixed-width terminal summary section.
#[cfg(feature = "proofs")]
const fn witness_end_summary_encoded_len() -> usize
{
    return WITNESS_END_SUMMARY_MAGIC
        .len()
        .saturating_add(2_usize)
        .saturating_add(1_usize)
        .saturating_add(NODE_HASH_LEN)
        .saturating_add(8_usize)
        .saturating_add(NODE_HASH_LEN)
        .saturating_add(NODE_HASH_LEN)
        .saturating_add(NODE_HASH_LEN)
        .saturating_add(8_usize)
        .saturating_add(NODE_HASH_LEN)
        .saturating_add(NODE_HASH_LEN);
}

/// Hashes committed chunker parameter bytes for the witness end summary.
#[cfg(feature = "proofs")]
fn hash_witness_chunker_parameters(bytes: &[u8]) -> Result<NodeHash, ProllyBaoError>
{
    let mut hasher = blake3::Hasher::new();
    hasher.update(WITNESS_CHUNKER_SUMMARY_MAGIC);
    update_len_prefixed_bytes_digest(&mut hasher, bytes)?;

    return Ok(finish_witness_digest(&hasher));
}

/// Hashes kind-specific witness query and returned material.
#[cfg(feature = "proofs")]
fn hash_witness_body(body: &WitnessBody) -> Result<NodeHash, ProllyBaoError>
{
    let mut hasher = blake3::Hasher::new();
    hasher.update(WITNESS_BODY_SUMMARY_MAGIC);

    match *body {
        | WitnessBody::Membership { ref key, ref value } => {
            hasher.update([WITNESS_KIND_MEMBERSHIP].as_ref());
            update_len_prefixed_bytes_digest(&mut hasher, key.as_ref())?;
            update_len_prefixed_bytes_digest(&mut hasher, value.as_ref())?;
        },
        | WitnessBody::NonMembership {
            ref key,
            ref evidence,
        } => {
            hasher.update([WITNESS_KIND_NON_MEMBERSHIP].as_ref());
            update_len_prefixed_bytes_digest(&mut hasher, key.as_ref())?;
            update_optional_witness_record_digest(&mut hasher, evidence.predecessor())?;
            update_optional_witness_record_digest(&mut hasher, evidence.successor())?;
        },
        | WitnessBody::Range {
            ref range,
            ref records,
        } => {
            hasher.update([WITNESS_KIND_RANGE].as_ref());
            update_witness_bound_digest(&mut hasher, range.start())?;
            update_witness_bound_digest(&mut hasher, range.end())?;
            update_u64_digest(
                &mut hasher,
                u64_from_usize(records.len(), "witness record count does not fit u64")?,
            );

            for record in records.as_ref() {
                update_witness_record_digest(&mut hasher, record)?;
            }
        },
    }

    return Ok(finish_witness_digest(&hasher));
}

/// Hashes encoded proof-node material for the witness end summary.
#[cfg(feature = "proofs")]
fn hash_witness_nodes(
    proof_node_count: u64,
    nodes: &[ProofNode],
) -> Result<NodeHash, ProllyBaoError>
{
    let mut hasher = blake3::Hasher::new();
    hasher.update(WITNESS_NODES_SUMMARY_MAGIC);
    update_u64_digest(&mut hasher, proof_node_count);

    for node in nodes {
        hasher.update(node.hash().as_slice());
        update_len_prefixed_bytes_digest(&mut hasher, node.bytes())?;
    }

    return Ok(finish_witness_digest(&hasher));
}

/// Hashes all explicit end-summary fields into the terminal binding digest.
#[cfg(feature = "proofs")]
fn hash_witness_end_summary_binding(parts: &WitnessSummaryParts) -> NodeHash
{
    let mut hasher = blake3::Hasher::new();
    hasher.update(WITNESS_END_SUMMARY_BINDING_MAGIC);
    update_u16_digest(&mut hasher, parts.version);
    hasher.update([parts.kind.tag()].as_ref());
    hasher.update(parts.root_hash.as_slice());
    update_u64_digest(&mut hasher, parts.root_record_count);
    hasher.update(parts.chunker_parameter_digest.as_slice());
    hasher.update(parts.root_node_hash.as_slice());
    hasher.update(parts.body_digest.as_slice());
    update_u64_digest(&mut hasher, parts.proof_node_count);
    hasher.update(parts.proof_nodes_digest.as_slice());

    return finish_witness_digest(&hasher);
}

/// Hashes one optional adjacent-evidence record in deterministic witness form.
#[cfg(feature = "proofs")]
fn update_optional_witness_record_digest(
    hasher: &mut blake3::Hasher,
    record: Option<&Record>,
) -> Result<(), ProllyBaoError>
{
    match record {
        | None => {
            hasher.update([WITNESS_OPTION_NONE].as_ref());
        },
        | Some(record) => {
            hasher.update([WITNESS_OPTION_RECORD].as_ref());
            update_witness_record_digest(hasher, record)?;
        },
    }

    return Ok(());
}

/// Hashes one record in deterministic witness form.
#[cfg(feature = "proofs")]
fn update_witness_record_digest(
    hasher: &mut blake3::Hasher,
    record: &Record,
) -> Result<(), ProllyBaoError>
{
    update_len_prefixed_bytes_digest(hasher, record.key())?;
    update_len_prefixed_bytes_digest(hasher, record.value())?;

    return Ok(());
}

/// Hashes one range bound in deterministic witness form.
#[cfg(feature = "proofs")]
fn update_witness_bound_digest(
    hasher: &mut blake3::Hasher,
    bound: &OwnedKeyBound,
) -> Result<(), ProllyBaoError>
{
    match *bound {
        | OwnedKeyBound::Unbounded => {
            hasher.update([WITNESS_BOUND_UNBOUNDED].as_ref());
        },
        | OwnedKeyBound::Included(ref key) => {
            hasher.update([WITNESS_BOUND_INCLUDED].as_ref());
            update_len_prefixed_bytes_digest(hasher, key.as_ref())?;
        },
        | OwnedKeyBound::Excluded(ref key) => {
            hasher.update([WITNESS_BOUND_EXCLUDED].as_ref());
            update_len_prefixed_bytes_digest(hasher, key.as_ref())?;
        },
    }

    return Ok(());
}

/// Hashes one length-prefixed byte field without materializing encoded bytes.
#[cfg(feature = "proofs")]
fn update_len_prefixed_bytes_digest(
    hasher: &mut blake3::Hasher,
    bytes: &[u8],
) -> Result<(), ProllyBaoError>
{
    update_u64_digest(
        hasher,
        u64_from_usize(bytes.len(), "witness byte length does not fit u64")?,
    );
    hasher.update(bytes);

    return Ok(());
}

/// Hashes one big-endian `u16` field.
#[cfg(feature = "proofs")]
fn update_u16_digest(
    hasher: &mut blake3::Hasher,
    value: u16,
)
{
    hasher.update(value.to_be_bytes().as_ref());
}

/// Hashes one big-endian `u64` field.
#[cfg(feature = "proofs")]
fn update_u64_digest(
    hasher: &mut blake3::Hasher,
    value: u64,
)
{
    hasher.update(value.to_be_bytes().as_ref());
}

/// Converts a BLAKE3 digest into the crate's public 32-byte hash value.
#[cfg(feature = "proofs")]
fn finish_witness_digest(hasher: &blake3::Hasher) -> NodeHash
{
    let digest = hasher.finalize();

    return NodeHash::from_bytes(*digest.as_bytes());
}

/// Decodes the kind-specific native witness body.
#[cfg(feature = "proofs")]
fn decode_witness_body(
    cursor: &mut WitnessCursor<'_>,
    kind: WitnessKind,
) -> Result<WitnessBody, ProllyBaoError>
{
    match kind {
        | WitnessKind::Membership => {
            let key = decode_witness_bytes(cursor, "membership witness key is truncated")?;
            let value = decode_witness_bytes(cursor, "membership witness value is truncated")?;

            return Ok(WitnessBody::Membership { key, value });
        },
        | WitnessKind::NonMembership => {
            let key = decode_witness_bytes(cursor, "non-membership witness key is truncated")?;
            let predecessor = decode_optional_witness_record(cursor)?;
            let successor = decode_optional_witness_record(cursor)?;
            let evidence = NonMembershipEvidence::new(predecessor, successor);

            return Ok(WitnessBody::NonMembership { key, evidence });
        },
        | WitnessKind::Range => {
            let start = decode_witness_bound(cursor)?;
            let end = decode_witness_bound(cursor)?;
            let range = OwnedKeyRange { start, end };
            range.as_range_ref()?;
            let count = cursor.read_u64()?;
            let capacity =
                cursor.item_capacity(count, 16_usize, "range witness record table is truncated")?;
            let mut records = Vec::<Record>::with_capacity(capacity);

            for _ in 0_u64 .. count {
                records.push(decode_witness_record(cursor)?);
            }

            return Ok(WitnessBody::Range {
                range,
                records: records.into_boxed_slice(),
            });
        },
    }
}

/// Decodes encoded proof nodes carried by a witness transcript.
#[cfg(feature = "proofs")]
fn decode_witness_nodes(cursor: &mut WitnessCursor<'_>)
-> Result<Box<[ProofNode]>, ProllyBaoError>
{
    let count = cursor.read_u64()?;
    let capacity = cursor.item_capacity(
        count,
        NODE_HASH_LEN + 8_usize,
        "witness proof node table is truncated",
    )?;
    let mut nodes = Vec::<ProofNode>::with_capacity(capacity);

    for _ in 0_u64 .. count {
        let hash = NodeHash::from_bytes(
            cursor.take_array::<NODE_HASH_LEN>("witness proof node hash is truncated")?,
        );
        let bytes = decode_witness_bytes(cursor, "witness proof node bytes are truncated")?;
        nodes.push(ProofNode::new(hash, bytes));
    }

    return Ok(nodes.into_boxed_slice());
}

/// Decodes one optional record in adjacent-key evidence.
#[cfg(feature = "proofs")]
fn decode_optional_witness_record(
    cursor: &mut WitnessCursor<'_>
) -> Result<Option<Record>, ProllyBaoError>
{
    match cursor.read_u8()? {
        | WITNESS_OPTION_NONE => return Ok(None),
        | WITNESS_OPTION_RECORD => return Ok(Some(decode_witness_record(cursor)?)),
        | _ => {
            return Err(ProllyBaoError::MalformedWitnessBytes {
                context: "unknown witness optional-record tag",
            });
        },
    }
}

/// Decodes one owned record from a witness transcript.
#[cfg(feature = "proofs")]
fn decode_witness_record(cursor: &mut WitnessCursor<'_>) -> Result<Record, ProllyBaoError>
{
    let key = decode_witness_bytes(cursor, "witness record key is truncated")?;
    let value = decode_witness_bytes(cursor, "witness record value is truncated")?;

    return Ok(Record::new(key, value));
}

/// Decodes one owned range bound from a witness transcript.
#[cfg(feature = "proofs")]
fn decode_witness_bound(cursor: &mut WitnessCursor<'_>) -> Result<OwnedKeyBound, ProllyBaoError>
{
    match cursor.read_u8()? {
        | WITNESS_BOUND_UNBOUNDED => return Ok(OwnedKeyBound::Unbounded),
        | WITNESS_BOUND_INCLUDED => {
            let key = decode_witness_bytes(cursor, "included witness bound key is truncated")?;

            return Ok(OwnedKeyBound::Included(key));
        },
        | WITNESS_BOUND_EXCLUDED => {
            let key = decode_witness_bytes(cursor, "excluded witness bound key is truncated")?;

            return Ok(OwnedKeyBound::Excluded(key));
        },
        | _ => {
            return Err(ProllyBaoError::MalformedWitnessBytes {
                context: "unknown witness bound tag",
            });
        },
    }
}

/// Decodes one length-prefixed byte string from a witness transcript.
#[cfg(feature = "proofs")]
fn decode_witness_bytes(
    cursor: &mut WitnessCursor<'_>,
    context: &'static str,
) -> Result<Box<[u8]>, ProllyBaoError>
{
    let len = usize_from_witness_u64(cursor.read_u64()?, "witness byte length does not fit usize")?;
    let bytes = cursor.take(len, context)?;

    return Ok(Box::<[u8]>::from(bytes));
}

/// Encodes the kind-specific native witness body.
#[cfg(feature = "proofs")]
fn encode_witness_body(
    body: &WitnessBody,
    out: &mut Vec<u8>,
) -> Result<(), ProllyBaoError>
{
    match *body {
        | WitnessBody::Membership { ref key, ref value } => {
            push_len_prefixed_bytes(out, key.as_ref())?;
            push_len_prefixed_bytes(out, value.as_ref())?;
        },
        | WitnessBody::NonMembership {
            ref key,
            ref evidence,
        } => {
            push_len_prefixed_bytes(out, key.as_ref())?;
            encode_optional_witness_record(evidence.predecessor(), out)?;
            encode_optional_witness_record(evidence.successor(), out)?;
        },
        | WitnessBody::Range {
            ref range,
            ref records,
        } => {
            encode_witness_bound(range.start(), out)?;
            encode_witness_bound(range.end(), out)?;
            push_u64(
                out,
                u64_from_usize(records.len(), "witness record count does not fit u64")?,
            );

            for record in records.as_ref() {
                encode_witness_record(record, out)?;
            }
        },
    }

    return Ok(());
}

/// Encodes one optional adjacent-evidence record.
#[cfg(feature = "proofs")]
fn encode_optional_witness_record(
    record: Option<&Record>,
    out: &mut Vec<u8>,
) -> Result<(), ProllyBaoError>
{
    match record {
        | None => out.push(WITNESS_OPTION_NONE),
        | Some(record) => {
            out.push(WITNESS_OPTION_RECORD);
            encode_witness_record(record, out)?;
        },
    }

    return Ok(());
}

/// Encodes one record in deterministic witness format.
#[cfg(feature = "proofs")]
fn encode_witness_record(
    record: &Record,
    out: &mut Vec<u8>,
) -> Result<(), ProllyBaoError>
{
    push_len_prefixed_bytes(out, record.key())?;
    push_len_prefixed_bytes(out, record.value())?;

    return Ok(());
}

/// Encodes one owned range bound in deterministic witness format.
#[cfg(feature = "proofs")]
fn encode_witness_bound(
    bound: &OwnedKeyBound,
    out: &mut Vec<u8>,
) -> Result<(), ProllyBaoError>
{
    match *bound {
        | OwnedKeyBound::Unbounded => out.push(WITNESS_BOUND_UNBOUNDED),
        | OwnedKeyBound::Included(ref key) => {
            out.push(WITNESS_BOUND_INCLUDED);
            push_len_prefixed_bytes(out, key.as_ref())?;
        },
        | OwnedKeyBound::Excluded(ref key) => {
            out.push(WITNESS_BOUND_EXCLUDED);
            push_len_prefixed_bytes(out, key.as_ref())?;
        },
    }

    return Ok(());
}

/// Appends one length-prefixed byte field.
#[cfg(feature = "proofs")]
fn push_len_prefixed_bytes(
    out: &mut Vec<u8>,
    bytes: &[u8],
) -> Result<(), ProllyBaoError>
{
    push_u64(
        out,
        u64_from_usize(bytes.len(), "witness byte length does not fit u64")?,
    );
    out.extend_from_slice(bytes);

    return Ok(());
}

/// Returns the deterministic encoded length for the kind-specific body.
#[cfg(feature = "proofs")]
fn witness_body_encoded_len(body: &WitnessBody) -> Result<usize, ProllyBaoError>
{
    let mut len = 0_usize;

    match *body {
        | WitnessBody::Membership { ref key, ref value } => {
            checked_add_to_len(
                &mut len,
                len_prefixed_bytes_encoded_len(key.as_ref())?,
                "witness encoded length overflow",
            )?;
            checked_add_to_len(
                &mut len,
                len_prefixed_bytes_encoded_len(value.as_ref())?,
                "witness encoded length overflow",
            )?;
        },
        | WitnessBody::NonMembership {
            ref key,
            ref evidence,
        } => {
            checked_add_to_len(
                &mut len,
                len_prefixed_bytes_encoded_len(key.as_ref())?,
                "witness encoded length overflow",
            )?;
            checked_add_to_len(
                &mut len,
                optional_record_encoded_len(evidence.predecessor())?,
                "witness encoded length overflow",
            )?;
            checked_add_to_len(
                &mut len,
                optional_record_encoded_len(evidence.successor())?,
                "witness encoded length overflow",
            )?;
        },
        | WitnessBody::Range {
            ref range,
            ref records,
        } => {
            checked_add_to_len(
                &mut len,
                bound_encoded_len(range.start())?,
                "witness encoded length overflow",
            )?;
            checked_add_to_len(
                &mut len,
                bound_encoded_len(range.end())?,
                "witness encoded length overflow",
            )?;
            checked_add_to_len(&mut len, 8_usize, "witness encoded length overflow")?;

            for record in records.as_ref() {
                checked_add_to_len(
                    &mut len,
                    record_encoded_len(record)?,
                    "witness encoded length overflow",
                )?;
            }
        },
    }

    return Ok(len);
}

/// Returns the encoded length for one optional record.
#[cfg(feature = "proofs")]
fn optional_record_encoded_len(record: Option<&Record>) -> Result<usize, ProllyBaoError>
{
    let mut len = 1_usize;

    if let Some(record) = record {
        checked_add_to_len(
            &mut len,
            record_encoded_len(record)?,
            "witness encoded length overflow",
        )?;
    }

    return Ok(len);
}

/// Returns the encoded length for one record.
#[cfg(feature = "proofs")]
fn record_encoded_len(record: &Record) -> Result<usize, ProllyBaoError>
{
    let mut len = len_prefixed_bytes_encoded_len(record.key())?;
    checked_add_to_len(
        &mut len,
        len_prefixed_bytes_encoded_len(record.value())?,
        "witness encoded length overflow",
    )?;

    return Ok(len);
}

/// Returns the encoded length for one range bound.
#[cfg(feature = "proofs")]
fn bound_encoded_len(bound: &OwnedKeyBound) -> Result<usize, ProllyBaoError>
{
    let mut len = 1_usize;

    match *bound {
        | OwnedKeyBound::Unbounded => {},
        | OwnedKeyBound::Included(ref key) | OwnedKeyBound::Excluded(ref key) => {
            checked_add_to_len(
                &mut len,
                len_prefixed_bytes_encoded_len(key.as_ref())?,
                "witness encoded length overflow",
            )?;
        },
    }

    return Ok(len);
}

/// Returns the encoded length for one length-prefixed byte field.
#[cfg(feature = "proofs")]
const fn len_prefixed_bytes_encoded_len(bytes: &[u8]) -> Result<usize, ProllyBaoError>
{
    return checked_add_usize(8_usize, bytes.len(), "witness encoded length overflow");
}

/// Adds `add` to an encoded-length accumulator.
fn checked_add_to_len(
    len: &mut usize,
    add: usize,
    context: &'static str,
) -> Result<(), ProllyBaoError>
{
    *len = checked_add_usize(*len, add, context)?;

    return Ok(());
}

/// Converts a witness `u64` length or count to `usize`.
#[cfg(feature = "proofs")]
fn usize_from_witness_u64(
    value: u64,
    context: &'static str,
) -> Result<usize, ProllyBaoError>
{
    match usize::try_from(value) {
        | Ok(converted) => return Ok(converted),
        | Err(_) => return Err(ProllyBaoError::MalformedWitnessBytes { context }),
    }
}

/// Borrowed byte decoder for native witness transcript bytes.
#[cfg(feature = "proofs")]
struct WitnessCursor<'bytes>
{
    /// Remaining unread bytes.
    remaining: &'bytes [u8],
}

#[cfg(feature = "proofs")]
impl<'bytes> WitnessCursor<'bytes>
{
    /// Creates a cursor over witness bytes.
    #[inline]
    #[must_use]
    const fn new(bytes: &'bytes [u8]) -> Self
    {
        return Self { remaining: bytes };
    }

    /// Takes exactly `len` bytes.
    const fn take(
        &mut self,
        len: usize,
        context: &'static str,
    ) -> Result<&'bytes [u8], ProllyBaoError>
    {
        if self.remaining.len() < len {
            return Err(ProllyBaoError::MalformedWitnessBytes { context });
        }

        let (head, tail) = self.remaining.split_at(len);
        self.remaining = tail;

        return Ok(head);
    }

    /// Reads a fixed-size byte array.
    fn take_array<const LEN: usize>(
        &mut self,
        context: &'static str,
    ) -> Result<[u8; LEN], ProllyBaoError>
    {
        let bytes = self.take(LEN, context)?;
        let mut array = [0_u8; LEN];
        array.copy_from_slice(bytes);

        return Ok(array);
    }

    /// Reads one byte.
    fn read_u8(&mut self) -> Result<u8, ProllyBaoError>
    {
        let byte = self.take_array::<1>("witness u8 field is truncated")?;

        return Ok(u8::from_be_bytes(byte));
    }

    /// Reads one big-endian `u16`.
    fn read_u16(&mut self) -> Result<u16, ProllyBaoError>
    {
        let bytes = self.take_array::<2>("witness u16 field is truncated")?;

        return Ok(u16::from_be_bytes(bytes));
    }

    /// Reads one big-endian `u64`.
    fn read_u64(&mut self) -> Result<u64, ProllyBaoError>
    {
        let bytes = self.take_array::<8>("witness u64 field is truncated")?;

        return Ok(u64::from_be_bytes(bytes));
    }

    /// Returns true when all bytes have been consumed.
    #[inline]
    #[must_use]
    const fn is_empty(&self) -> bool
    {
        return self.remaining.is_empty();
    }

    /// Converts an item count to capacity after checking that the remaining
    /// bytes can carry at least `min_bytes_per_item` for every item.
    fn item_capacity(
        &self,
        count: u64,
        min_bytes_per_item: usize,
        context: &'static str,
    ) -> Result<usize, ProllyBaoError>
    {
        let capacity = usize_from_witness_u64(count, context)?;
        let Some(minimum_len) = capacity.checked_mul(min_bytes_per_item)
        else {
            return Err(ProllyBaoError::MalformedWitnessBytes { context });
        };

        if minimum_len > self.remaining.len() {
            return Err(ProllyBaoError::MalformedWitnessBytes { context });
        }

        return Ok(capacity);
    }
}

/// Borrowed byte decoder for canonical snapshot bytes.
struct SnapshotCursor<'bytes>
{
    /// Remaining unread bytes.
    remaining: &'bytes [u8],
}

impl<'bytes> SnapshotCursor<'bytes>
{
    /// Creates a cursor over snapshot bytes.
    #[inline]
    #[must_use]
    const fn new(bytes: &'bytes [u8]) -> Self
    {
        return Self { remaining: bytes };
    }

    /// Takes exactly `len` bytes.
    const fn take(
        &mut self,
        len: usize,
        context: &'static str,
    ) -> Result<&'bytes [u8], ProllyBaoError>
    {
        if self.remaining.len() < len {
            return Err(ProllyBaoError::MalformedSnapshotBytes { context });
        }

        let (head, tail) = self.remaining.split_at(len);
        self.remaining = tail;

        return Ok(head);
    }

    /// Reads a fixed-size byte array.
    fn take_array<const LEN: usize>(
        &mut self,
        context: &'static str,
    ) -> Result<[u8; LEN], ProllyBaoError>
    {
        let bytes = self.take(LEN, context)?;
        let mut array = [0_u8; LEN];
        array.copy_from_slice(bytes);

        return Ok(array);
    }

    /// Reads one big-endian `u16`.
    fn read_u16(&mut self) -> Result<u16, ProllyBaoError>
    {
        let bytes = self.take_array::<2>("snapshot u16 field is truncated")?;

        return Ok(u16::from_be_bytes(bytes));
    }

    /// Reads one big-endian `u64`.
    fn read_u64(&mut self) -> Result<u64, ProllyBaoError>
    {
        let bytes = self.take_array::<8>("snapshot u64 field is truncated")?;

        return Ok(u64::from_be_bytes(bytes));
    }

    /// Returns true when all bytes have been consumed.
    #[inline]
    #[must_use]
    const fn is_empty(&self) -> bool
    {
        return self.remaining.is_empty();
    }

    /// Converts an item count to capacity after checking that the remaining
    /// bytes can carry at least `min_bytes_per_item` for every item.
    fn item_capacity(
        &self,
        count: u64,
        min_bytes_per_item: usize,
        context: &'static str,
    ) -> Result<usize, ProllyBaoError>
    {
        let capacity = usize_from_snapshot_u64(count, context)?;
        let Some(minimum_len) = capacity.checked_mul(min_bytes_per_item)
        else {
            return Err(ProllyBaoError::MalformedSnapshotBytes { context });
        };

        if minimum_len > self.remaining.len() {
            return Err(ProllyBaoError::MalformedSnapshotBytes { context });
        }

        return Ok(capacity);
    }
}

/// Verifies snapshot header fields against explicit verifier context.
fn verify_snapshot_binding(
    root_hash: NodeHash,
    root_record_count: u64,
    chunker_parameter_bytes: &[u8],
    expected_root: &TreeRoot,
    expected_params: &TreeParams,
) -> Result<(), ProllyBaoError>
{
    if expected_root.params() != expected_params {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "expected root parameters do not match verifier parameters",
        });
    }

    if root_hash != expected_root.hash() {
        return Err(ProllyBaoError::HashMismatch {
            expected: expected_root.hash(),
            actual: root_hash,
        });
    }

    if root_record_count != expected_root.record_count() {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "snapshot root record count does not match verifier root",
        });
    }

    if chunker_parameter_bytes != expected_params.chunker_parameter_bytes() {
        return Err(ProllyBaoError::IncompatibleTreeParameters {
            context: "snapshot chunker parameter commitment mismatch",
        });
    }

    return Ok(());
}

/// Decodes borrowed snapshot records.
fn decode_snapshot_records<'bytes>(
    cursor: &mut SnapshotCursor<'bytes>,
    record_count: u64,
) -> Result<Vec<RecordRef<'bytes>>, ProllyBaoError>
{
    let capacity = cursor.item_capacity(
        record_count,
        16_usize,
        "snapshot record framing is truncated",
    )?;
    let mut records = Vec::<RecordRef<'bytes>>::with_capacity(capacity);

    for _ in 0_u64 .. record_count {
        records.push(decode_snapshot_record(cursor)?);
    }

    return Ok(records);
}

/// Decodes one borrowed record from a snapshot stream.
fn decode_snapshot_record<'bytes>(
    cursor: &mut SnapshotCursor<'bytes>
) -> Result<RecordRef<'bytes>, ProllyBaoError>
{
    let key = decode_snapshot_bytes(cursor, "snapshot key bytes are truncated")?;
    let value = decode_snapshot_bytes(cursor, "snapshot value bytes are truncated")?;

    return Ok(RecordRef::new(key, value));
}

/// Decodes one length-prefixed byte string from snapshot bytes.
fn decode_snapshot_bytes<'bytes>(
    cursor: &mut SnapshotCursor<'bytes>,
    context: &'static str,
) -> Result<&'bytes [u8], ProllyBaoError>
{
    let len = cursor.read_u64()?;
    let len = usize_from_snapshot_u64(len, context)?;

    return cursor.take(len, context);
}

/// Appends one snapshot length-prefixed byte field.
fn push_snapshot_len_prefixed_bytes(
    out: &mut Vec<u8>,
    bytes: &[u8],
) -> Result<(), ProllyBaoError>
{
    push_u64(
        out,
        u64_from_usize(bytes.len(), "snapshot byte length does not fit u64")?,
    );
    out.extend_from_slice(bytes);

    return Ok(());
}

/// Returns the encoded length for one snapshot record.
fn snapshot_record_encoded_len(record: &Record) -> Result<usize, ProllyBaoError>
{
    let mut len = snapshot_len_prefixed_bytes_encoded_len(record.key())?;
    checked_add_to_len(
        &mut len,
        snapshot_len_prefixed_bytes_encoded_len(record.value())?,
        "snapshot encoded length overflow",
    )?;

    return Ok(len);
}

/// Returns the encoded length for one snapshot length-prefixed byte field.
const fn snapshot_len_prefixed_bytes_encoded_len(bytes: &[u8]) -> Result<usize, ProllyBaoError>
{
    return checked_add_usize(8_usize, bytes.len(), "snapshot encoded length overflow");
}

/// Converts a snapshot `u64` length or count to `usize`.
fn usize_from_snapshot_u64(
    value: u64,
    context: &'static str,
) -> Result<usize, ProllyBaoError>
{
    match usize::try_from(value) {
        | Ok(converted) => return Ok(converted),
        | Err(_) => return Err(ProllyBaoError::MalformedSnapshotBytes { context }),
    }
}

/// Validates strict sorted order for input records.
fn validate_sorted_records(records: &[RecordRef<'_>]) -> Result<(), ProllyBaoError>
{
    let mut previous_key: Option<&[u8]> = None;
    let mut previous_index = 0_u64;
    let mut current_index = 0_u64;

    for record in records {
        if let Some(key) = previous_key {
            match key.cmp(record.key()) {
                | Ordering::Less => {},
                | Ordering::Equal => {
                    return Err(ProllyBaoError::DuplicateKeys {
                        first_index: previous_index,
                        second_index: current_index,
                    });
                },
                | Ordering::Greater => {
                    return Err(ProllyBaoError::UnsortedInput {
                        previous_index,
                        current_index,
                    });
                },
            }
        }

        previous_key = Some(record.key());
        previous_index = current_index;
        current_index = checked_add_u64(current_index, 1_u64, "record index overflow")?;
    }

    return Ok(());
}

/// Copies borrowed records into owned records.
fn own_records(records: &[RecordRef<'_>]) -> Box<[Record]>
{
    let mut owned = Vec::<Record>::with_capacity(records.len());

    for record in records {
        owned.push(Record::from(*record));
    }

    return owned.into_boxed_slice();
}

/// Builds canonical record byte strings for chunker input.
fn canonical_record_bytes(records: &[RecordRef<'_>]) -> Result<Box<[Box<[u8]>]>, ProllyBaoError>
{
    let mut canonical = Vec::<Box<[u8]>>::with_capacity(records.len());

    for record in records {
        canonical.push(encode_record_for_chunker(*record)?);
    }

    return Ok(canonical.into_boxed_slice());
}

/// Borrows encoded record byte strings for the chunker API.
fn borrowed_slices(records: &[Box<[u8]>]) -> Box<[&[u8]]>
{
    let mut slices = Vec::<&[u8]>::with_capacity(records.len());

    for record in records {
        slices.push(record.as_ref());
    }

    return slices.into_boxed_slice();
}

/// Encodes one canonical record for chunker boundary detection.
fn encode_record_for_chunker(record: RecordRef<'_>) -> Result<Box<[u8]>, ProllyBaoError>
{
    let key_len = u64_from_usize(record.key().len(), "record key length does not fit u64")?;
    let value_len = u64_from_usize(record.value().len(), "record value length does not fit u64")?;
    let capacity = checked_add_usize(
        RECORD_MAGIC.len(),
        checked_add_usize(
            16_usize,
            record.key().len(),
            "record encoding length overflow",
        )?,
        "record encoding length overflow",
    )?;
    let capacity = checked_add_usize(
        capacity,
        record.value().len(),
        "record encoding length overflow",
    )?;
    let mut bytes = Vec::<u8>::with_capacity(capacity);
    bytes.extend_from_slice(RECORD_MAGIC);
    push_u64(&mut bytes, key_len);
    push_u64(&mut bytes, value_len);
    bytes.extend_from_slice(record.key());
    bytes.extend_from_slice(record.value());

    return Ok(bytes.into_boxed_slice());
}

/// Builds one leaf node from borrowed records.
fn build_leaf(records: &[RecordRef<'_>]) -> Result<LeafBuild, ProllyBaoError>
{
    let bytes = encode_leaf_node(records)?;
    let hash = hash_encoded_node(bytes.as_ref());
    let first_key = match records.first() {
        | Some(record) => Some(Box::<[u8]>::from(record.key())),
        | None => None,
    };
    let record_count = u64_from_usize(records.len(), "leaf record count does not fit u64")?;

    return Ok(LeafBuild {
        hash,
        bytes,
        first_key,
        record_count,
    });
}

/// Builds either a leaf root or a one-level internal root.
fn build_root_node(leaves: &[LeafBuild]) -> Result<RootBuild, ProllyBaoError>
{
    let only_leaf = leaves.first().ok_or(ProllyBaoError::InvalidProofShape {
        context: "tree build produced no leaves",
    })?;

    if leaves.len() == 1_usize {
        return Ok(RootBuild {
            hash: only_leaf.hash,
            bytes: only_leaf.bytes.clone(),
        });
    }

    let bytes = encode_internal_node(leaves)?;
    let hash = hash_encoded_node(bytes.as_ref());

    return Ok(RootBuild { hash, bytes });
}

/// Collects root and leaf nodes in deterministic order.
fn collect_nodes(
    root: RootBuild,
    leaves: &[LeafBuild],
) -> Box<[ProofNode]>
{
    let mut nodes = Vec::<ProofNode>::with_capacity(leaves.len().saturating_add(1_usize));
    nodes.push(ProofNode::new(root.hash, root.bytes));

    if leaves.len() != 1_usize {
        for leaf in leaves {
            nodes.push(ProofNode::new(leaf.hash, leaf.bytes.clone()));
        }
    }

    return nodes.into_boxed_slice();
}

/// Collects leaf hashes in canonical order.
fn collect_leaf_hashes(leaves: &[LeafBuild]) -> Box<[NodeHash]>
{
    let mut hashes = Vec::<NodeHash>::with_capacity(leaves.len());

    for leaf in leaves {
        hashes.push(leaf.hash);
    }

    return hashes.into_boxed_slice();
}

/// Encodes a leaf node.
fn encode_leaf_node(records: &[RecordRef<'_>]) -> Result<Box<[u8]>, ProllyBaoError>
{
    let record_count = u64_from_usize(records.len(), "leaf record count does not fit u64")?;
    let mut bytes = Vec::<u8>::new();
    push_node_header(&mut bytes, NODE_KIND_LEAF);
    push_u64(&mut bytes, record_count);

    for record in records {
        let key_len = u64_from_usize(record.key().len(), "leaf key length does not fit u64")?;
        let value_len = u64_from_usize(record.value().len(), "leaf value length does not fit u64")?;
        push_u64(&mut bytes, key_len);
        push_u64(&mut bytes, value_len);
        bytes.extend_from_slice(record.key());
        bytes.extend_from_slice(record.value());
    }

    return Ok(bytes.into_boxed_slice());
}

/// Encodes an internal root node.
fn encode_internal_node(leaves: &[LeafBuild]) -> Result<Box<[u8]>, ProllyBaoError>
{
    let child_count = u64_from_usize(leaves.len(), "internal child count does not fit u64")?;
    let mut total_records = 0_u64;
    let mut bytes = Vec::<u8>::new();
    push_node_header(&mut bytes, NODE_KIND_INTERNAL);

    for leaf in leaves {
        total_records = checked_add_u64(
            total_records,
            leaf.record_count,
            "internal record count overflow",
        )?;
    }

    push_u64(&mut bytes, total_records);
    push_u64(&mut bytes, child_count);

    for leaf in leaves {
        let first_key = leaf
            .first_key
            .as_ref()
            .ok_or(ProllyBaoError::InvalidProofShape {
                context: "non-empty internal child has no first key",
            })?;
        let first_key_len = u64_from_usize(
            first_key.len(),
            "internal separator length does not fit u64",
        )?;
        push_u64(&mut bytes, first_key_len);
        bytes.extend_from_slice(first_key.as_ref());
        bytes.extend_from_slice(leaf.hash.as_slice());
        push_u64(&mut bytes, leaf.record_count);
    }

    return Ok(bytes.into_boxed_slice());
}

/// Appends common node header fields.
fn push_node_header(
    bytes: &mut Vec<u8>,
    kind: u8,
)
{
    bytes.extend_from_slice(NODE_MAGIC);
    push_u16(bytes, EncodingVersion::CURRENT.as_u16());
    bytes.push(kind);
}

/// Decodes and verifies every proof node hash.
#[cfg(feature = "proofs")]
fn decode_proof_nodes(nodes: &[ProofNode]) -> Result<Vec<DecodedNode>, ProllyBaoError>
{
    if nodes.is_empty() {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "proof carries no node bytes",
        });
    }

    let mut decoded = Vec::<DecodedNode>::with_capacity(nodes.len());

    for node in nodes {
        let actual = hash_encoded_node(node.bytes());

        if actual != node.hash() {
            return Err(ProllyBaoError::HashMismatch {
                expected: node.hash(),
                actual,
            });
        }

        if find_decoded_node(decoded.as_slice(), node.hash()).is_some() {
            return Err(ProllyBaoError::InvalidProofShape {
                context: "proof carries duplicate node hash",
            });
        }

        decoded.push(decode_encoded_node(node.clone())?);
    }

    return Ok(decoded);
}

/// Reads and validates the common encoded-node header.
fn read_node_header(cursor: &mut Cursor<'_>) -> Result<u8, ProllyBaoError>
{
    let magic = cursor.take(NODE_MAGIC.len(), "node magic is truncated")?;

    if magic != NODE_MAGIC {
        return Err(ProllyBaoError::MalformedNodeBytes {
            context: "node magic mismatch",
        });
    }

    let version = cursor.read_u16()?;

    if version != EncodingVersion::CURRENT.as_u16() {
        return Err(ProllyBaoError::UnsupportedEncodingVersion { version });
    }

    return cursor.read_u8();
}

/// Decodes one encoded node after its hash has already been checked.
fn decode_encoded_node(node: ProofNode) -> Result<DecodedNode, ProllyBaoError>
{
    let mut cursor = Cursor::new(node.bytes());
    let kind = read_node_header(&mut cursor)?;
    let decoded_kind = match kind {
        | NODE_KIND_LEAF => DecodedNodeKind::Leaf(decode_leaf_payload(&mut cursor)?),
        | NODE_KIND_INTERNAL => DecodedNodeKind::Internal(decode_internal_payload(&mut cursor)?),
        | _ => {
            return Err(ProllyBaoError::MalformedNodeBytes {
                context: "unknown node kind",
            });
        },
    };

    if !cursor.is_empty() {
        return Err(ProllyBaoError::MalformedNodeBytes {
            context: "trailing node bytes",
        });
    }

    return Ok(DecodedNode {
        proof_node: node,
        kind: decoded_kind,
    });
}

/// Inspects a leaf payload without materializing owned records.
fn inspect_leaf_payload(cursor: &mut Cursor<'_>) -> Result<u64, ProllyBaoError>
{
    let count = cursor.read_u64()?;
    let mut previous_key: Option<&[u8]> = None;
    let mut previous_index = 0_u64;
    let mut current_index = 0_u64;

    for _ in 0_u64 .. count {
        let key_len = usize_from_u64(cursor.read_u64()?, "leaf key length does not fit usize")?;
        let value_len = usize_from_u64(cursor.read_u64()?, "leaf value length does not fit usize")?;
        let key = cursor.take(key_len, "leaf key is truncated")?;
        cursor.take(value_len, "leaf value is truncated")?;

        if let Some(previous) = previous_key {
            match previous.cmp(key) {
                | Ordering::Less => {},
                | Ordering::Equal => {
                    return Err(ProllyBaoError::DuplicateKeys {
                        first_index: previous_index,
                        second_index: current_index,
                    });
                },
                | Ordering::Greater => {
                    return Err(ProllyBaoError::MalformedNodeBytes {
                        context: "leaf records are unsorted",
                    });
                },
            }
        }

        previous_key = Some(key);
        previous_index = current_index;
        current_index = checked_add_u64(current_index, 1_u64, "leaf record index overflow")?;
    }

    return Ok(count);
}

/// Inspects an internal payload without materializing owned child references.
fn inspect_internal_payload(cursor: &mut Cursor<'_>) -> Result<u64, ProllyBaoError>
{
    let record_count = cursor.read_u64()?;
    let child_count = cursor.read_u64()?;

    if child_count == 0_u64 {
        return Err(ProllyBaoError::MalformedNodeBytes {
            context: "internal node has no children",
        });
    }

    let mut total_child_records = 0_u64;
    let mut previous_first_key: Option<&[u8]> = None;

    for _ in 0_u64 .. child_count {
        let first_key_len = usize_from_u64(
            cursor.read_u64()?,
            "internal separator length does not fit usize",
        )?;
        let first_key = cursor.take(first_key_len, "internal separator key is truncated")?;
        cursor.take(NODE_HASH_LEN, "internal child hash is truncated")?;
        let child_record_count = cursor.read_u64()?;

        if child_record_count == 0_u64 {
            return Err(ProllyBaoError::MalformedNodeBytes {
                context: "internal child has zero records",
            });
        }

        if let Some(previous) = previous_first_key
            && previous >= first_key
        {
            return Err(ProllyBaoError::MalformedNodeBytes {
                context: "internal separators are not strictly sorted",
            });
        }

        total_child_records = checked_add_u64(
            total_child_records,
            child_record_count,
            "internal child record count overflow",
        )?;
        previous_first_key = Some(first_key);
    }

    if total_child_records != record_count {
        return Err(ProllyBaoError::MalformedNodeBytes {
            context: "internal child record counts do not match record count",
        });
    }

    return Ok(child_count);
}

/// Decodes a leaf payload.
fn decode_leaf_payload(cursor: &mut Cursor<'_>) -> Result<DecodedLeaf, ProllyBaoError>
{
    let count = cursor.read_u64()?;
    let capacity = usize_from_u64(count, "leaf record count does not fit usize")?;
    let mut records = Vec::<Record>::with_capacity(capacity);
    let mut previous_key: Option<Box<[u8]>> = None;
    let mut previous_index = 0_u64;
    let mut current_index = 0_u64;

    for _ in 0_u64 .. count {
        let key_len = usize_from_u64(cursor.read_u64()?, "leaf key length does not fit usize")?;
        let value_len = usize_from_u64(cursor.read_u64()?, "leaf value length does not fit usize")?;
        let key = Box::<[u8]>::from(cursor.take(key_len, "leaf key is truncated")?);
        let value = Box::<[u8]>::from(cursor.take(value_len, "leaf value is truncated")?);

        if let Some(previous) = previous_key.as_ref() {
            match previous.as_ref().cmp(key.as_ref()) {
                | Ordering::Less => {},
                | Ordering::Equal => {
                    return Err(ProllyBaoError::DuplicateKeys {
                        first_index: previous_index,
                        second_index: current_index,
                    });
                },
                | Ordering::Greater => {
                    return Err(ProllyBaoError::MalformedNodeBytes {
                        context: "leaf records are unsorted",
                    });
                },
            }
        }

        previous_key = Some(key.clone());
        previous_index = current_index;
        current_index = checked_add_u64(current_index, 1_u64, "leaf record index overflow")?;
        records.push(Record::new(key, value));
    }

    return Ok(DecodedLeaf {
        records: records.into_boxed_slice(),
    });
}

/// Decodes an internal payload.
fn decode_internal_payload(cursor: &mut Cursor<'_>) -> Result<DecodedInternal, ProllyBaoError>
{
    let record_count = cursor.read_u64()?;
    let child_count = cursor.read_u64()?;

    if child_count == 0_u64 {
        return Err(ProllyBaoError::MalformedNodeBytes {
            context: "internal node has no children",
        });
    }

    let capacity = usize_from_u64(child_count, "internal child count does not fit usize")?;
    let mut children = Vec::<DecodedChild>::with_capacity(capacity);
    let mut previous_first_key: Option<Box<[u8]>> = None;

    for _ in 0_u64 .. child_count {
        let first_key_len = usize_from_u64(
            cursor.read_u64()?,
            "internal separator length does not fit usize",
        )?;
        let first_key =
            Box::<[u8]>::from(cursor.take(first_key_len, "internal separator key is truncated")?);
        let child_hash = NodeHash::from_bytes(
            cursor.take_array::<NODE_HASH_LEN>("internal child hash is truncated")?,
        );
        let child_record_count = cursor.read_u64()?;

        if child_record_count == 0_u64 {
            return Err(ProllyBaoError::MalformedNodeBytes {
                context: "internal child has zero records",
            });
        }

        if let Some(previous) = previous_first_key.as_ref()
            && previous.as_ref() >= first_key.as_ref()
        {
            return Err(ProllyBaoError::MalformedNodeBytes {
                context: "internal separators are not strictly sorted",
            });
        }

        previous_first_key = Some(first_key.clone());
        children.push(DecodedChild {
            first_key,
            hash: child_hash,
            record_count: child_record_count,
        });
    }

    return Ok(DecodedInternal {
        record_count,
        children: children.into_boxed_slice(),
    });
}

/// Selects the first leaf that could contribute records to `range`.
#[cfg(feature = "proofs")]
fn start_child_index_for_range(
    children: &[DecodedChild],
    range: KeyRangeRef<'_>,
) -> Option<usize>
{
    return match range.start() {
        | KeyBound::Unbounded => {
            if children.is_empty() {
                None
            }
            else {
                Some(0_usize)
            }
        },
        | KeyBound::Included(key) | KeyBound::Excluded(key) => {
            select_child_index_for_key(children, key)
        },
    };
}

/// Selects the last leaf that could contribute records to `range`.
#[cfg(feature = "proofs")]
fn end_child_index_for_range(
    children: &[DecodedChild],
    range: KeyRangeRef<'_>,
) -> Option<usize>
{
    return match range.end() {
        | KeyBound::Unbounded => children.len().checked_sub(1_usize),
        | KeyBound::Included(key) | KeyBound::Excluded(key) => {
            select_child_index_for_key(children, key)
        },
    };
}

/// Returns the global record slice boundaries for one child leaf.
#[cfg(feature = "proofs")]
fn child_record_span(
    children: &[DecodedChild],
    child_index: usize,
) -> Result<(usize, usize), ProllyBaoError>
{
    let mut start = 0_usize;

    for child in children.iter().take(child_index) {
        let child_len =
            usize_from_u64(child.record_count, "child record count does not fit usize")?;
        start = checked_add_usize(start, child_len, "child record span overflow")?;
    }

    let selected_child = children
        .get(child_index)
        .ok_or(ProllyBaoError::InvalidProofShape {
            context: "selected child index is out of bounds",
        })?;
    let selected_len = usize_from_u64(
        selected_child.record_count,
        "child record count does not fit usize",
    )?;
    let end = checked_add_usize(start, selected_len, "child record span overflow")?;

    return Ok((start, end));
}

/// Verifies a membership path proof.
#[cfg(feature = "proofs")]
fn verify_membership_path(
    expected_root: &TreeRoot,
    root_node_hash: NodeHash,
    key: &[u8],
    value: &[u8],
    nodes: &[ProofNode],
) -> Result<(), ProllyBaoError>
{
    let decoded = decode_proof_nodes(nodes)?;
    verify_root_manifest(expected_root, root_node_hash)?;
    let root = find_decoded_node(decoded.as_slice(), root_node_hash).ok_or(
        ProllyBaoError::InvalidProofShape {
            context: "membership root node is absent",
        },
    )?;

    match root.kind {
        | DecodedNodeKind::Leaf(ref leaf) => {
            if decoded.len() != 1_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "leaf membership proof carries extra nodes",
                });
            }
            verify_leaf_record_count(leaf, expected_root.record_count())?;
            verify_leaf_contains(leaf, key, value)?;
        },
        | DecodedNodeKind::Internal(ref internal) => {
            if decoded.len() != 2_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "internal membership proof must carry root and one leaf",
                });
            }
            if internal.record_count != expected_root.record_count() {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "internal root record count mismatch",
                });
            }
            let child = select_child_for_key(internal.children.as_ref(), key).ok_or(
                ProllyBaoError::InvalidProofShape {
                    context: "membership key has no selected child",
                },
            )?;
            let child_node = find_decoded_node(decoded.as_slice(), child.hash).ok_or(
                ProllyBaoError::InvalidProofShape {
                    context: "membership child node is absent",
                },
            )?;
            let leaf = match child_node.kind {
                | DecodedNodeKind::Leaf(ref leaf) => leaf,
                | DecodedNodeKind::Internal(_) => {
                    return Err(ProllyBaoError::InvalidProofShape {
                        context: "membership child is not a leaf",
                    });
                },
            };
            verify_child_leaf(child, leaf)?;
            verify_leaf_contains(leaf, key, value)?;
        },
    }

    return Ok(());
}

/// Returns true when a compact non-membership proof needs the adjacent
/// successor leaf based on the in-memory record slice.
#[cfg(feature = "proofs")]
fn compact_non_membership_needs_next_records(
    records: &[Record],
    key: &[u8],
) -> Result<bool, ProllyBaoError>
{
    for record in records {
        match record.key().cmp(key) {
            | Ordering::Less => {},
            | Ordering::Equal => {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "compact non-membership proof key is present",
                });
            },
            | Ordering::Greater => return Ok(false),
        }
    }

    return Ok(true);
}

#[expect(
    dead_code,
    reason = "kept as a fallback verifier for future full-tree-only proof experiments"
)]
/// Verifies a full reachable tree proof.
#[cfg(feature = "proofs")]
fn verify_full_reachable_tree(
    expected_root: &TreeRoot,
    root_node_hash: NodeHash,
    nodes: &[ProofNode],
) -> Result<VerifiedTree, ProllyBaoError>
{
    let decoded = decode_proof_nodes(nodes)?;
    verify_root_manifest(expected_root, root_node_hash)?;
    let root = find_decoded_node(decoded.as_slice(), root_node_hash).ok_or(
        ProllyBaoError::InvalidProofShape {
            context: "full proof root node is absent",
        },
    )?;

    match root.kind {
        | DecodedNodeKind::Leaf(ref leaf) => {
            if decoded.len() != 1_usize {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "full leaf proof carries unreachable nodes",
                });
            }
            verify_leaf_record_count(leaf, expected_root.record_count())?;
            return Ok(VerifiedTree {
                records: leaf.records.clone(),
            });
        },
        | DecodedNodeKind::Internal(ref internal) => {
            return verify_full_internal_tree(expected_root, internal, decoded.as_slice());
        },
    }
}

/// Verifies a full internal-root tree proof.
#[cfg(feature = "proofs")]
fn verify_full_internal_tree(
    expected_root: &TreeRoot,
    internal: &DecodedInternal,
    decoded: &[DecodedNode],
) -> Result<VerifiedTree, ProllyBaoError>
{
    if internal.record_count != expected_root.record_count() {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "full proof internal record count mismatch",
        });
    }

    let expected_node_count = checked_add_usize(
        internal.children.len(),
        1_usize,
        "full proof node count overflow",
    )?;

    if decoded.len() != expected_node_count {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "full proof carries unreachable or missing nodes",
        });
    }

    let mut records = Vec::<Record>::new();
    let mut total_records = 0_u64;
    let mut previous_key: Option<Box<[u8]>> = None;

    for child in internal.children.as_ref() {
        let child_node =
            find_decoded_node(decoded, child.hash).ok_or(ProllyBaoError::InvalidProofShape {
                context: "full proof child node is absent",
            })?;
        let leaf = match child_node.kind {
            | DecodedNodeKind::Leaf(ref leaf) => leaf,
            | DecodedNodeKind::Internal(_) => {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "full proof child is not a leaf",
                });
            },
        };

        verify_child_leaf(child, leaf)?;
        total_records = checked_add_u64(
            total_records,
            child.record_count,
            "full proof record count overflow",
        )?;

        for record in leaf.records.as_ref() {
            if let Some(previous) = previous_key.as_ref()
                && previous.as_ref() >= record.key()
            {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "full proof records are not globally sorted",
                });
            }

            previous_key = Some(Box::<[u8]>::from(record.key()));
            records.push(record.clone());
        }
    }

    if total_records != expected_root.record_count() {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "full proof total record count mismatch",
        });
    }

    return Ok(VerifiedTree {
        records: records.into_boxed_slice(),
    });
}

/// Verifies one child reference against a decoded leaf.
#[cfg(feature = "proofs")]
fn verify_child_leaf(
    child: &DecodedChild,
    leaf: &DecodedLeaf,
) -> Result<(), ProllyBaoError>
{
    let first_key = leaf.first_key().ok_or(ProllyBaoError::InvalidProofShape {
        context: "child leaf has no first key",
    })?;

    if first_key != child.first_key.as_ref() {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "child first key does not match leaf",
        });
    }

    if leaf.record_count()? != child.record_count {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "child record count does not match leaf",
        });
    }

    return Ok(());
}

/// Verifies leaf record count.
#[cfg(feature = "proofs")]
fn verify_leaf_record_count(
    leaf: &DecodedLeaf,
    expected_count: u64,
) -> Result<(), ProllyBaoError>
{
    if leaf.record_count()? != expected_count {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "leaf root record count mismatch",
        });
    }

    return Ok(());
}

/// Verifies that `leaf` contains the requested key/value binding.
#[cfg(feature = "proofs")]
fn verify_leaf_contains(
    leaf: &DecodedLeaf,
    key: &[u8],
    value: &[u8],
) -> Result<(), ProllyBaoError>
{
    let record =
        find_record(leaf.records.as_ref(), key).ok_or(ProllyBaoError::InvalidProofShape {
            context: "membership key is not present in authenticated leaf",
        })?;

    if record.value() != value {
        return Err(ProllyBaoError::InvalidProofShape {
            context: "membership value is not bound to authenticated key",
        });
    }

    return Ok(());
}

/// Verifies root-manifest hash context.
#[cfg(feature = "proofs")]
fn verify_root_manifest(
    expected_root: &TreeRoot,
    root_node_hash: NodeHash,
) -> Result<(), ProllyBaoError>
{
    let actual = hash_root_manifest(
        expected_root.params(),
        expected_root.record_count(),
        root_node_hash,
    )?;

    if actual != expected_root.hash() {
        return Err(ProllyBaoError::HashMismatch {
            expected: expected_root.hash(),
            actual,
        });
    }

    return Ok(());
}

/// Computes the committed root manifest hash.
fn hash_root_manifest(
    params: &TreeParams,
    record_count: u64,
    root_node_hash: NodeHash,
) -> Result<NodeHash, ProllyBaoError>
{
    ensure_supported_params(params)?;

    let chunker_bytes = params.chunker_parameter_bytes();
    let chunker_len = u64_from_usize(
        chunker_bytes.len(),
        "chunker parameter length does not fit u64",
    )?;
    let mut bytes = Vec::<u8>::new();
    bytes.extend_from_slice(ROOT_MAGIC);
    push_u16(&mut bytes, params.encoding_version().as_u16());
    bytes.push(TREE_KIND_MERKLE_SEARCH);
    bytes.push(HASH_ALGORITHM_BLAKE3);
    bytes.push(SEPARATOR_FIRST_KEY);
    push_u64(&mut bytes, record_count);
    push_u64(&mut bytes, chunker_len);
    bytes.extend_from_slice(chunker_bytes);
    bytes.extend_from_slice(root_node_hash.as_slice());

    return Ok(NodeHash::from_bytes(
        *blake3::hash(bytes.as_slice()).as_bytes(),
    ));
}

/// Finds a decoded node by hash.
#[cfg(feature = "proofs")]
fn find_decoded_node(
    nodes: &[DecodedNode],
    hash: NodeHash,
) -> Option<&DecodedNode>
{
    return nodes.iter().find(|node| {
        return node.hash() == hash;
    });
}

/// Finds an encoded proof node by hash.
#[cfg(feature = "proofs")]
fn find_proof_node(
    nodes: &[ProofNode],
    hash: NodeHash,
) -> Option<&ProofNode>
{
    return nodes.iter().find(|node| {
        return node.hash() == hash;
    });
}

/// Selects the child index whose separator range contains `key`.
#[cfg(feature = "proofs")]
fn select_child_index_for_key(
    children: &[DecodedChild],
    key: &[u8],
) -> Option<usize>
{
    let mut selected = if children.is_empty() {
        return None;
    }
    else {
        0_usize
    };

    for (index, child) in children.iter().enumerate() {
        if child.first_key.as_ref() <= key {
            selected = index;
        }
        else {
            return Some(selected);
        }
    }

    return Some(selected);
}

/// Selects the child whose separator range contains `key`.
#[cfg(feature = "proofs")]
fn select_child_for_key<'child>(
    children: &'child [DecodedChild],
    key: &[u8],
) -> Option<&'child DecodedChild>
{
    let index = select_child_index_for_key(children, key)?;
    return children.get(index);
}

/// Finds one record by key.
fn find_record<'record>(
    records: &'record [Record],
    key: &[u8],
) -> Option<&'record Record>
{
    for record in records {
        match record.key().cmp(key) {
            | Ordering::Less => {},
            | Ordering::Equal => return Some(record),
            | Ordering::Greater => return None,
        }
    }

    return None;
}

/// Returns true when a compact non-membership proof needs the adjacent
/// successor leaf.
#[cfg(feature = "proofs")]
fn compact_non_membership_needs_next_leaf(
    selected_leaf: &DecodedLeaf,
    key: &[u8],
) -> Result<bool, ProllyBaoError>
{
    for record in selected_leaf.records.as_ref() {
        match record.key().cmp(key) {
            | Ordering::Less => {},
            | Ordering::Equal => {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "compact non-membership proof key is present",
                });
            },
            | Ordering::Greater => return Ok(false),
        }
    }

    return Ok(true);
}

/// Computes adjacent absence evidence from a compact selected leaf plus an
/// optional adjacent successor leaf.
#[cfg(feature = "proofs")]
fn compact_non_membership_evidence(
    selected_leaf: &DecodedLeaf,
    next_leaf: Option<&DecodedLeaf>,
    key: &[u8],
) -> Result<NonMembershipEvidence, ProllyBaoError>
{
    let mut predecessor: Option<Record> = None;

    for record in selected_leaf.records.as_ref() {
        match record.key().cmp(key) {
            | Ordering::Less => predecessor = Some(record.clone()),
            | Ordering::Equal => {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "compact non-membership proof key is present",
                });
            },
            | Ordering::Greater => {
                return Ok(NonMembershipEvidence::new(
                    predecessor,
                    Some(record.clone()),
                ));
            },
        }
    }

    let successor = match next_leaf {
        | Some(leaf) => {
            let record = leaf
                .records
                .first()
                .ok_or(ProllyBaoError::InvalidProofShape {
                    context: "compact non-membership successor leaf is empty",
                })?;
            if record.key() <= key {
                return Err(ProllyBaoError::InvalidProofShape {
                    context: "compact non-membership successor leaf does not advance key order",
                });
            }
            Some(record.clone())
        },
        | None => None,
    };

    return Ok(NonMembershipEvidence::new(predecessor, successor));
}

/// Computes adjacent absence evidence from authenticated records.
#[cfg(feature = "proofs")]
fn adjacent_evidence(
    records: &[Record],
    key: &[u8],
) -> NonMembershipEvidence
{
    let mut predecessor: Option<Record> = None;

    for record in records {
        match record.key().cmp(key) {
            | Ordering::Less => predecessor = Some(record.clone()),
            | Ordering::Equal | Ordering::Greater => {
                return NonMembershipEvidence::new(predecessor, Some(record.clone()));
            },
        }
    }

    return NonMembershipEvidence::new(predecessor, None);
}

/// Selects records in `range`.
fn select_range_records(
    records: &[Record],
    range: KeyRangeRef<'_>,
) -> Box<[Record]>
{
    let mut selected = Vec::<Record>::new();

    for record in records {
        if key_is_in_range(record.key(), range) {
            selected.push(record.clone());
        }
    }

    return selected.into_boxed_slice();
}

/// Returns true when `key` is inside `range`.
fn key_is_in_range(
    key: &[u8],
    range: KeyRangeRef<'_>,
) -> bool
{
    let start_ok = match range.start() {
        | KeyBound::Unbounded => true,
        | KeyBound::Included(start) => key >= start,
        | KeyBound::Excluded(start) => key > start,
    };

    if !start_ok {
        return false;
    }

    match range.end() {
        | KeyBound::Unbounded => return true,
        | KeyBound::Included(end) => return key <= end,
        | KeyBound::Excluded(end) => return key < end,
    }
}

/// Appends a big-endian `u16`.
fn push_u16(
    bytes: &mut Vec<u8>,
    value: u16,
)
{
    bytes.extend_from_slice(value.to_be_bytes().as_ref());
}

/// Appends a big-endian `u64`.
fn push_u64(
    bytes: &mut Vec<u8>,
    value: u64,
)
{
    bytes.extend_from_slice(value.to_be_bytes().as_ref());
}

/// Converts `usize` to `u64` with context.
fn u64_from_usize(
    value: usize,
    context: &'static str,
) -> Result<u64, ProllyBaoError>
{
    match u64::try_from(value) {
        | Ok(converted) => return Ok(converted),
        | Err(_) => return Err(ProllyBaoError::InvalidProofShape { context }),
    }
}

/// Converts `u64` to `usize` with context.
fn usize_from_u64(
    value: u64,
    context: &'static str,
) -> Result<usize, ProllyBaoError>
{
    match usize::try_from(value) {
        | Ok(converted) => return Ok(converted),
        | Err(_) => return Err(ProllyBaoError::InvalidProofShape { context }),
    }
}

/// Checked `u64` addition with static context.
const fn checked_add_u64(
    lhs: u64,
    rhs: u64,
    context: &'static str,
) -> Result<u64, ProllyBaoError>
{
    match lhs.checked_add(rhs) {
        | Some(sum) => return Ok(sum),
        | None => return Err(ProllyBaoError::InvalidProofShape { context }),
    }
}

/// Checked `usize` addition with static context.
const fn checked_add_usize(
    lhs: usize,
    rhs: usize,
    context: &'static str,
) -> Result<usize, ProllyBaoError>
{
    match lhs.checked_add(rhs) {
        | Some(sum) => return Ok(sum),
        | None => return Err(ProllyBaoError::InvalidProofShape { context }),
    }
}
