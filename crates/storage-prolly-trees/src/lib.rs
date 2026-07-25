//! Prolly-Bao ordered-record Merkle tree primitives.
//!
//! # Provenance
//!
//! This crate is a storage-tier skeleton absorbed directly from the owner's
//! unpublished `mach` `storage-prolly-trees` crate (Apache-2.0, same owner;
//! source commit `fb78601`). It is a direct source absorption adapted to the
//! gandr storage tier and lint discipline — not an external dependency, and not
//! yet wired into any export path. See the ratified vendor plan in
//! `docs/research/massive-term-design.md` §6.1.
//!
//! # Crate boundary
//!
//! `alloc`-based (over `core`/`alloc`), depending only on the gandr workspace
//! pins `blake3` and `thiserror` plus the sibling `gandr-storage-chunker`. Node
//! identity is [`NodeHash`] = `BLAKE3(encoded node bytes)`, a
//! `#[repr(transparent)]` newtype over `[u8; 32]`. The generic sorted-record
//! interface carries **no** declaration semantics: a future export layer is a
//! consumer that supplies the record model. Fail-closed record discipline
//! ([`ProllyBaoError::UnsortedInput`], [`ProllyBaoError::DuplicateKeys`]) is
//! preserved.
//!
//! # Feature flags
//!
//! - `proofs` (default): the membership / non-membership / range proof
//!   machinery and the native witness-transcript surface. Disabling it keeps
//!   tree construction, lookup/range queries, snapshot encode/verify, node
//!   hashing, and the block stores; the proof machinery is feature-gated, never
//!   stripped.
//!
//! # Inherited-deferred
//!
//! Carried honestly from the ratified vendor plan (§6.1), unbuilt here:
//! multi-level tree construction and its proofs (the scale ceiling; the tree is
//! two-level); a spec-asserted history-independence differential (canonicality
//! is by-construction, not a named theorem); incremental/streaming witness
//! verification (the witness verifier is full-rebuild; `bao` provides verified
//! streaming as dev evidence only); a persistent store backend; and
//! anti-boundary-grinding hardening (only hard byte/record caps exist).
//!
//! ## current
//!
//! - This crate exposes the first public value types for ordered record roots,
//!   BLAKE3 node hashes, committed tree parameters, store-facing node bytes,
//!   and proof metadata.
//! - Prolly-Bao consumes `storage-chunker` parameter commitments as consensus
//!   material. Rolling or Gear boundary metadata is not node identity.
//! - The public hash surface is [`NodeHash`]. This crate does not claim Bao
//!   wire compatibility.
//! - Native [`WitnessTranscript`] values are `current` Prolly-Bao
//!   ordered-record query-response transcripts. They are not Bao byte-stream
//!   proofs.
//! - [`verify_snapshot_bytes`] verifies deterministic `current` Prolly-Bao
//!   snapshot bytes against [`TreeRoot`] / [`TreeParams`]. Bao verification of
//!   those bytes remains adapter evidence outside core semantics.
//!
//! ## designed direction
//!
//! - Future modules will build deterministic Merkle search trees over sorted
//!   canonical key-value records.
//! - Future proof APIs will verify membership, non-membership, and range claims
//!   against [`TreeRoot`] while checking [`TreeParams`].
//! - Future adapters may translate records from higher-level systems, but SQL,
//!   `DataFusion`, `Iroh`, `Git`, `Automerge`, `IPLD/CAR`, filesystem paths,
//!   networks, and storage engines stay outside this core public API.
//!
//! ## open decision
//!
//! - Persistent storage, transport adapters, version graph ownership,
//!   multi-writer merge semantics, and standalone repository extraction remain
//!   outside this skeleton.

#![expect(
    clippy::pub_use,
    reason = "crate root intentionally exposes a flat public API for this small core crate"
)]

extern crate alloc;

#[expect(
    clippy::module_name_repetitions,
    reason = "ProllyBaoError keeps crate-specific error identity in the public API"
)]
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
