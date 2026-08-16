//! Outer-layer content-addressed wiring for kernel export artifacts, and their
//! manifest identity (the massive-term design record, §6).
//!
//! A v1 export artifact is, by construction, a **sorted unique keyed record
//! set**: E2 admission ordering keys each declaration by its admission index,
//! and the format is declaration-segmented, so records are
//! `(admission index → declaration segment bytes)`. This crate is the
//! gandr-specific **consumer** of that fact. It extracts the records
//! ([`record`]), flows them through record-safe (declaration-granular) chunking
//! into a `BlockStore`-backed prolly tree, and mints a canonical BLAKE3
//! **manifest identity** ([`manifest`]) binding the chunker parameter
//! commitment, the record count, the root node hash, and the inner kernel
//! export format version.
//!
//! # Placement and the two walls (kernel-boundary discipline)
//!
//! This is a `storage-*` tier crate — **untrusted plumbing** by the
//! kernel-boundary naming rule (only `gandr-kernel-*` is trusted). Hashing
//! lives here, outside the kernel TCB. The generic tree crates
//! ([`gandr_storage_prolly_trees`], [`gandr_storage_chunker`]) carry **no**
//! declaration semantics; this crate supplies the declaration record model as a
//! consumer of their generic sorted-record interface.
//!
//! **Integrity never substitutes validity.** The manifest hash is the **outer**
//! wall: it addresses and authenticates bytes. It binds the inner format
//! version so the identity commits to *which* canonical inner encoding the
//! records carry — but it does **not** re-check them. K2/E3 replay re-derives
//! every typing and well-formedness obligation from the canonical inner bytes
//! ([`gandr_kernel_core::read`]); the **inner** wall is the sole validity
//! authority. A matching identity proves provenance, never validity; the hash
//! is untrusted plumbing. This crate changes **no** replay semantics.
//!
//! # Canonicality is a stated property, not an accident
//!
//! The record set is canonical by construction — sorted and unique by admission
//! key — and the prolly tree is a deterministic function of the sorted record
//! set. So the artifact identity is **history-independent**: a permuted build
//! or insertion order yields the identical root and identity. This is a *tested
//! claim* (the history-independence differential), pinned as a property of the
//! outer layer here rather than left implicit.
//!
//! # The step-granularity sibling
//!
//! [`transport`] holds the durable identity of one certificate **step** — a
//! fixed-width BLAKE3 identity over a canonical framed preimage, minted only
//! through a streaming encoder with the versioned step-domain magic built in
//! (tracker item `gandr-4o8a`). It lives in this crate because it shares the
//! discipline: canonical bytes in, one digest out, validity never implied.

extern crate alloc;

pub mod error;
pub mod manifest;
pub mod record;
pub mod transport;

use gandr_kernel_core::Environment;
use gandr_storage_prolly_trees::BlockStore;
use gandr_storage_prolly_trees::NodeHash;
use gandr_storage_prolly_trees::ProllyTree;
use gandr_storage_prolly_trees::TreeParams;
use gandr_storage_prolly_trees::TreeRoot;

pub use crate::error::ArtifactError;
pub use crate::error::ManifestError;
pub use crate::error::StepIdError;
pub use crate::manifest::ARTIFACT_IDENTITY_LEN;
pub use crate::manifest::ArtifactIdentity;
pub use crate::manifest::ArtifactManifest;
pub use crate::manifest::ArtifactRecordCount;
pub use crate::manifest::CHUNKER_COMMITMENT_LEN;
pub use crate::manifest::ChunkerCommitment;
pub use crate::manifest::EncodedManifest;
pub use crate::manifest::InnerFormatVersion;
pub use crate::manifest::MANIFEST_FORMAT_VERSION_V1;
pub use crate::manifest::MANIFEST_MAGIC;
pub use crate::manifest::ManifestBytes;
pub use crate::manifest::ManifestFormatVersion;
pub use crate::record::ADMISSION_KEY_LEN;
pub use crate::record::AdmissionIndex;
pub use crate::record::AdmissionKey;
pub use crate::record::ArtifactHeader;
pub use crate::record::ArtifactRecord;
pub use crate::record::ArtifactRecordSet;
pub use crate::record::ReassembledArtifact;
pub use crate::record::RecordBytes;
pub use crate::transport::CanonicalBytes;
pub use crate::transport::CanonicalU64;
pub use crate::transport::StepIdEncoder;
pub use crate::transport::TRANSPORT_STEP_ID_LEN;
pub use crate::transport::TRANSPORT_STEP_MAGIC;
pub use crate::transport::TransportStepId;

/// A built artifact: the prolly-tree root, its stored root node hash, and the
/// canonical manifest and identity minted over them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltArtifact
{
    /// The prolly-tree root manifest (chunker params, record count, root hash).
    root: TreeRoot,
    /// The root node hash stored in the backing `BlockStore`.
    root_node_hash: NodeHash,
    /// The canonical outer manifest binding the four identity fields.
    manifest: ArtifactManifest,
    /// The artifact identity — `BLAKE3` of the manifest.
    identity: ArtifactIdentity,
}

impl BuiltArtifact
{
    /// Returns the prolly-tree root manifest.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> &TreeRoot
    {
        return &self.root;
    }

    /// Returns the root node hash stored in the backing store.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.root_node_hash;
    }

    /// Returns the canonical outer manifest.
    #[inline]
    #[must_use]
    pub const fn manifest(&self) -> &ArtifactManifest
    {
        return &self.manifest;
    }

    /// Returns the artifact identity.
    #[inline]
    #[must_use]
    pub const fn identity(&self) -> ArtifactIdentity
    {
        return self.identity;
    }
}

/// Builds an artifact's prolly tree and mints its manifest identity, storing
/// every tree node in `store`.
///
/// # Contract
/// - requires: `records` is a canonical record set (sorted, unique by key);
///   `params` is a supported prolly-tree parameter set.
/// - ensures: `Ok(built)` when the records chunk and build; every carried tree
///   node is inserted into `store` (verified on insert), the manifest binds the
///   85-byte chunker commitment, the record count, the root node hash, and the
///   record set's inner format version, and the identity is `BLAKE3` of the
///   manifest. Building is a deterministic function of the sorted record set —
///   history-independent (the outer wall only; validity is re-checked at replay
///   from the canonical inner bytes, never here).
/// - provides: the outer content-addressed identity and its stored tree.
/// - fails: [`ArtifactError::Tree`] on a chunker/tree/store failure, or
///   [`ArtifactError::Manifest`] on an ill-formed commitment.
/// - panics: none.
///
/// # Errors
/// [`ArtifactError`].
///
/// # Adequacy
/// - hypothesis: L2 — the round-trip differential pins record extraction and
///   reassembly to byte identity, the determinism differential pins that the
///   same artifact mints the same identity, and the sensitivity differential
///   pins that any field perturbation changes it; the L3 residues are the empty
///   record set and the store round-trip.
/// - witness: `artifact_contract::records_round_trip_to_a_byte_identical_artifact`
/// - witness: `artifact_contract::the_same_artifact_mints_the_same_identity`
/// - witness: `artifact_contract::any_perturbation_changes_the_identity`
/// - witness: `artifact_contract::a_permuted_build_order_yields_the_same_identity`
/// - witness: `artifact_contract::tree_nodes_store_and_reopen`
#[inline]
pub fn build<S>(
    records: &ArtifactRecordSet,
    params: TreeParams,
    store: &mut S,
) -> Result<BuiltArtifact, ArtifactError>
where
    S: BlockStore + ?Sized,
{
    let record_refs = records.record_refs();
    let tree = ProllyTree::build(record_refs.as_slice(), params, store)?;
    let root = tree.root().clone();
    let root_node_hash = tree.root_node_hash();
    let chunker_commitment =
        ChunkerCommitment::try_from(root.params().chunker_parameter_commitment().as_ref())?;
    let manifest = ArtifactManifest::new(
        records.inner_format_version(),
        chunker_commitment,
        ArtifactRecordCount::from(u64::from(root.record_count())),
        root_node_hash,
    );
    let identity = manifest.identity();

    return Ok(BuiltArtifact {
        root,
        root_node_hash,
        manifest,
        identity,
    });
}

/// Extracts an environment's record set and builds its artifact identity.
///
/// # Contract
/// - requires: `params` is a supported prolly-tree parameter set.
/// - ensures: `build(&ArtifactRecordSet::from_environment(environment), params,
///   store)` — the producer-side convenience.
/// - provides: the one-call path from a kernel environment to an artifact
///   identity.
/// - fails: [`ArtifactError`], as [`build`].
/// - panics: none.
///
/// # Errors
/// [`ArtifactError`].
#[inline]
pub fn build_from_environment<S>(
    environment: &Environment,
    params: TreeParams,
    store: &mut S,
) -> Result<BuiltArtifact, ArtifactError>
where
    S: BlockStore + ?Sized,
{
    let records = ArtifactRecordSet::from_environment(environment);
    return build(&records, params, store);
}
