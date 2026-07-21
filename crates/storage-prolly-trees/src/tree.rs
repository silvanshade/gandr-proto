//! Thin tree wrapper over proof-oriented Prolly-Bao nodes.
//!
//! ## current
//!
//! - [`ProllyTree`] builds through [`PortableProofTree`] and stores every
//!   carried encoded node through [`BlockStore`].
//! - Record lookup, range, and proof production delegate to the carried
//!   [`PortableProofTree`] instead of duplicating proof encoding or decoding.
//! - [`OpenedProllyTree`] verifies that a named root node is present and
//!   hash-valid in a supplied [`BlockStore`].
//!
//! ## designed direction
//!
//! - Store-backed traversal can reconstruct record-dependent APIs once a public
//!   traversal helper exists for loading all reachable nodes from a root.
//!
//! ## open decision
//!
//! - Persistent stores, transport adapters, and full store-backed
//!   reconstruction remain outside this module.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::error::ProllyBaoError;
#[cfg(feature = "proofs")]
use crate::proof::MembershipProof;
#[cfg(feature = "proofs")]
use crate::proof::NonMembershipProof;
use crate::proof::PortableProofTree;
#[cfg(feature = "proofs")]
use crate::proof::RangeProof;
use crate::store::BlockStore;
use crate::types::KeyRangeRef;
use crate::types::NodeHash;
use crate::types::Record;
use crate::types::RecordRef;
use crate::types::StoredNodeRef;
use crate::types::TreeParams;
use crate::types::TreeRoot;

/// Public tree handle backed by an in-memory [`PortableProofTree`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::module_name_repetitions,
    reason = "public API intentionally uses ProllyTree terminology"
)]
#[non_exhaustive]
pub struct ProllyTree
{
    /// Proof-oriented tree carrying encoded nodes and owned records.
    portable: PortableProofTree,
}

impl ProllyTree
{
    /// Builds a deterministic tree and writes every carried encoded node to
    /// `store`.
    ///
    /// The returned handle delegates lookup, range, and proof APIs to the
    /// underlying [`PortableProofTree`]. The [`TreeRoot`] and proof envelopes
    /// produced by that tree carry the committed chunker parameter bytes.
    ///
    /// # Errors
    ///
    /// Returns construction errors from [`PortableProofTree::build`] or storage
    /// verification errors from [`BlockStore::insert`].
    #[inline]
    pub fn build<S>(
        records: &[RecordRef<'_>],
        params: TreeParams,
        store: &mut S,
    ) -> Result<Self, ProllyBaoError>
    where
        S: BlockStore + ?Sized,
    {
        let portable = PortableProofTree::build(records, params)?;

        for node in portable.nodes() {
            store.insert(StoredNodeRef::new(node.hash(), node.bytes()))?;
        }

        return Ok(Self { portable });
    }

    /// Opens a root against a block store by verifying the named root node is
    /// present and hash-valid.
    ///
    /// This does not reconstruct records or prove full reachability. The
    /// current proof module does not expose a traversal helper that can
    /// load every child node from a store-backed root without duplicating
    /// the private decoder. Returning [`OpenedProllyTree`] keeps that
    /// limitation explicit instead of fabricating lookup, range, or proof
    /// results from an incomplete handle.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::UnknownNodeHash`] when `root_node_hash` is not
    /// present in `store`, or verification errors when the stored root-node
    /// bytes do not hash to `root_node_hash` or do not decode as canonical
    /// node material.
    #[inline]
    pub fn open<S>(
        root: TreeRoot,
        root_node_hash: NodeHash,
        store: &S,
    ) -> Result<OpenedProllyTree, ProllyBaoError>
    where
        S: BlockStore + ?Sized,
    {
        store.load(root_node_hash)?;

        return Ok(OpenedProllyTree {
            root,
            root_node_hash,
        });
    }

    /// Returns this tree's root manifest.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> &TreeRoot
    {
        return self.portable.root();
    }

    /// Returns the root node hash named by the root manifest.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.portable.root_node_hash();
    }

    /// Appends canonical Prolly-Bao snapshot bytes to `out`.
    ///
    /// Delegates to [`PortableProofTree::encode_snapshot_bytes`]. The emitted
    /// bytes materialize this tree's ordered records under its root context and
    /// are not Bao proofs.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying portable tree cannot represent a
    /// length or count in the deterministic snapshot format.
    #[inline]
    pub fn encode_snapshot_bytes(
        &self,
        out: &mut Vec<u8>,
    ) -> Result<(), ProllyBaoError>
    {
        return self.portable.encode_snapshot_bytes(out);
    }

    /// Encodes this tree into owned canonical Prolly-Bao snapshot bytes.
    ///
    /// Delegates to [`PortableProofTree::to_snapshot_bytes`]. Bao verification
    /// of these bytes remains outside the core tree API.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying portable tree cannot represent a
    /// length or count in the deterministic snapshot format.
    #[inline]
    pub fn to_snapshot_bytes(&self) -> Result<Box<[u8]>, ProllyBaoError>
    {
        return self.portable.to_snapshot_bytes();
    }

    /// Looks up a key in this deterministic tree.
    #[inline]
    #[must_use]
    pub fn lookup(
        &self,
        key: &[u8],
    ) -> Option<&[u8]>
    {
        return self.portable.lookup(key);
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
        return self.portable.range(range);
    }

    /// Builds a membership proof for `key`.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::InvalidProofShape`] when `key` is absent or
    /// when the underlying proof tree cannot build a valid path.
    #[inline]
    #[cfg(feature = "proofs")]
    pub fn prove_membership(
        &self,
        key: &[u8],
    ) -> Result<MembershipProof, ProllyBaoError>
    {
        return self.portable.prove_membership(key);
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
        return self.portable.prove_non_membership(key);
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
        return self.portable.prove_range(range);
    }
}

/// Store-verified root handle returned by [`ProllyTree::open`].
///
/// This handle confirms only that the named root node was present and
/// hash-valid in a supplied [`BlockStore`]. It intentionally does not expose
/// lookup, range, or proof-producing methods until store-backed traversal can
/// be implemented without duplicating the proof module's private decoder.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::module_name_repetitions,
    reason = "public API intentionally distinguishes opened ProllyTree handles"
)]
#[non_exhaustive]
pub struct OpenedProllyTree
{
    /// Root manifest supplied by the caller.
    root: TreeRoot,
    /// Root-node hash verified against a store.
    root_node_hash: NodeHash,
}

impl OpenedProllyTree
{
    /// Returns the caller-supplied root manifest.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> &TreeRoot
    {
        return &self.root;
    }

    /// Returns the store-verified root node hash.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.root_node_hash;
    }

    /// Re-verifies that this handle's root node is present and hash-valid in
    /// `store`.
    ///
    /// # Errors
    ///
    /// Returns [`ProllyBaoError::UnknownNodeHash`] when the root node is
    /// absent, or verification errors when stored bytes no longer match the
    /// root node hash or canonical node encoding.
    #[inline]
    pub fn verify_root_present<S>(
        &self,
        store: &S,
    ) -> Result<(), ProllyBaoError>
    where
        S: BlockStore + ?Sized,
    {
        store.load(self.root_node_hash)?;

        return Ok(());
    }
}
