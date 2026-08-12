//! Prolly-Bao ordered-record Merkle tree primitives.
//!
//! # Provenance
//!
//! This crate is a direct source absorption of the owner's prior unpublished
//! implementation (Apache-2.0, same owner), adapted to the gandr storage tier
//! and lint discipline — not an external dependency. The ratified vendor plan
//! is the massive-term design study §6.1, which left this repository with the
//! research corpus.
//!
//! # Crate boundary
//!
//! `alloc`-based (over `core`/`alloc`), depending only on the gandr workspace
//! pins `blake3` and `thiserror` plus the sibling `gandr-storage-chunker`. Node
//! identity is [`NodeHash`] = `BLAKE3(encoded node bytes)`, a
//! `#[repr(transparent)]` newtype over `[u8; 32]`. The generic sorted-record
//! interface carries **no** declaration semantics: an export layer such as
//! `gandr-storage-artifact` is a consumer that supplies the record model.
//! Fail-closed record discipline ([`ProllyBaoError::UnsortedInput`],
//! [`ProllyBaoError::DuplicateKeys`]) is preserved.
//!
//! This crate exposes the public value types for ordered record roots, BLAKE3
//! node hashes, committed tree parameters, store-facing node bytes, and proof
//! metadata. Prolly-Bao consumes `storage-chunker` parameter commitments as
//! consensus material; rolling or Gear boundary metadata is not node identity.
//! The public hash surface is [`NodeHash`], and this crate does not claim Bao
//! wire compatibility. Native [`WitnessTranscript`] values are Prolly-Bao
//! ordered-record query-response transcripts, not Bao byte-stream proofs.
//! [`verify_snapshot_bytes`] verifies deterministic Prolly-Bao snapshot bytes
//! against [`TreeRoot`] / [`TreeParams`]; Bao verification of those bytes
//! remains adapter evidence outside core semantics.
//!
//! SQL, `DataFusion`, `Iroh`, `Git`, `Automerge`, `IPLD/CAR`, filesystem
//! paths, networks, persistent storage backends, transport adapters, version
//! graph ownership, multi-writer merge semantics, and standalone repository
//! extraction stay outside this core public API; adapters are separate work
//! with their own mapping and failure-mode decisions.
//!
//! # Limits
//!
//! The tree is two-level: multi-level construction and its proofs are unbuilt.
//! Canonicality is by construction rather than a named theorem. The witness
//! verifier is full-rebuild — there is no incremental or streaming witness
//! verification (`bao` provides verified streaming as dev evidence only). No
//! persistent store backend ships. Anti-boundary-grinding hardening is limited
//! to hard byte and record caps.
//!
//! # Feature flags
//!
//! - `proofs` (default): the membership / non-membership / range proof
//!   machinery and the native witness-transcript surface. Disabling it keeps
//!   tree construction, lookup/range queries, snapshot encode/verify, node
//!   hashing, and the block stores; the proof machinery is feature-gated, never
//!   stripped.

extern crate alloc;

pub mod error;
pub mod proof;
pub mod store;
pub mod tree;
pub mod types;

pub use crate::error::ProllyBaoError;
pub use crate::proof::EncodedLength;
pub use crate::proof::EncodedNodeKind;
pub use crate::proof::EncodedNodeLayout;
#[cfg(feature = "proofs")]
pub use crate::proof::MembershipProof;
pub use crate::proof::NodeChildCount;
pub use crate::proof::NodeOccupancy;
#[cfg(feature = "proofs")]
pub use crate::proof::NonMembershipEvidence;
#[cfg(feature = "proofs")]
pub use crate::proof::NonMembershipProof;
#[cfg(feature = "proofs")]
pub use crate::proof::OwnedKeyBound;
#[cfg(feature = "proofs")]
pub use crate::proof::OwnedKeyRange;
pub use crate::proof::OwnedSnapshotBytes;
pub use crate::proof::OwnedWitnessBytes;
pub use crate::proof::PortableProofTree;
pub use crate::proof::ProofNode;
pub use crate::proof::ProofNodeCount;
#[cfg(feature = "proofs")]
pub use crate::proof::RangeProof;
pub use crate::proof::SnapshotBuffer;
pub use crate::proof::SnapshotBytes;
pub use crate::proof::WitnessBuffer;
pub use crate::proof::WitnessBytes;
#[cfg(feature = "proofs")]
pub use crate::proof::WitnessEndSummary;
#[cfg(feature = "proofs")]
pub use crate::proof::WitnessKind;
#[cfg(feature = "proofs")]
pub use crate::proof::WitnessTranscript;
pub use crate::proof::hash_encoded_node;
pub use crate::proof::inspect_encoded_node;
pub use crate::proof::verify_snapshot_bytes;
pub use crate::proof::verify_stored_node;
pub use crate::store::BlockStore;
pub use crate::store::InMemoryBlockStore;
pub use crate::store::NodeSegmentEntry;
pub use crate::store::PackedSegmentBytes;
pub use crate::store::PackedSegmentLength;
pub use crate::store::PackedSegmentOffset;
pub use crate::store::PackedSegmentStore;
pub use crate::tree::OpenedProllyTree;
pub use crate::tree::ProllyTree;
pub use crate::types::EncodedNode;
pub use crate::types::EncodingVersion;
pub use crate::types::HashAlgorithm;
pub use crate::types::KeyBound;
pub use crate::types::KeyRangeRef;
pub use crate::types::NODE_HASH_LEN;
pub use crate::types::NodeHash;
pub use crate::types::OwnedEncodedNode;
pub use crate::types::OwnedRecordKey;
pub use crate::types::OwnedRecordValue;
pub use crate::types::ProofEnvelope;
pub use crate::types::ProofKind;
pub use crate::types::Record;
pub use crate::types::RecordKey;
pub use crate::types::RecordRef;
pub use crate::types::RecordValue;
pub use crate::types::SeparatorConvention;
pub use crate::types::StoredNodeRef;
pub use crate::types::TreeKind;
pub use crate::types::TreeParams;
pub use crate::types::TreeRecordCount;
pub use crate::types::TreeRoot;
